//! Composition of the retained drawing caches into a window scene.
use super::*;

impl Runtime {
    /// Composes cached drawing in root mount order and child presentation order.
    ///
    /// Only successfully updated components record again. Scene composition copies
    /// commands but never re-runs unchanged components' paint callbacks. The desktop
    /// adapter calls this after flushing updates. No timers or GPU work occur here.
    ///
    /// The scene carries what can produce pixels. A component whose effective clip
    /// has no area, such as an item scrolled out of a list, contributes no command
    /// and no coverage resource, and neither does an individual command that its
    /// clip removes; the cost of a composition follows the content on screen rather
    /// than the content mounted. Such a component is still retained, still measured
    /// and still records when it is invalidated, so revealing it composes it again
    /// without repainting it.
    ///
    /// # Errors
    /// Rejects pending updates or an invalid runtime phase; propagates component
    /// drawing errors. Failed or panicking callbacks keep their prior cache dirty.
    pub fn paint(&mut self) -> Result<crate::DisplayList, crate::DrawError> {
        self.paint_scene(true)
    }

    /// Composes the selected window's scene, with the active development
    /// overlay when `overlay` is set and the build has debug assertions.
    pub(super) fn paint_scene(
        &mut self,
        overlay: bool,
    ) -> Result<crate::DisplayList, crate::DrawError> {
        let _timing = self
            .shared
            .profiler
            .span(crate::ProfileOperation::Compose, None, "");
        self.shared.writable()?;
        if !self.shared.command_requests.borrow().is_empty()
            || !self.window.focus_errors.is_empty()
            || self.window.restore_focus
            || self.window.shared.stack_hover_pending.get()
            || !self.shared.requested_removals.borrow().is_empty()
            || !self.shared.requested_moves.borrow().is_empty()
            || !self.shared.requested_children.borrow().is_empty()
            || !self.shared.requested_text_commands.borrow().is_empty()
            || !self.window.shared.pending.borrow().is_empty()
            || self.window.shared.layout_pending.get()
        {
            return Err(crate::DrawError::PendingUpdates);
        }
        let previous = self.shared.phase.replace(Phase::Paint);
        let _guard = InspectionGuard {
            shared: self.shared.clone(),
            previous,
        };
        let viewport = self.window.shared.viewport.get();
        let palette = self.shared.theme.borrow().active();
        let mut scene = crate::DisplayList {
            background: palette.color(crate::ColorRole::Background),
            viewport,
            ..Default::default()
        };
        for (serial, foreground) in self.composition_order() {
            let entry = self
                .entries
                .get_mut(&serial)
                .ok_or(RuntimeError::Unmounted)?;
            let [bx, by, _, _] = entry
                .node
                .bounds
                .get()
                .map_or([0.0; 4], crate::Rect::values);
            let physical_origin = [
                f64::from(bx) * viewport.scale(),
                f64::from(by) * viewport.scale(),
            ];
            // Integer-pixel movement changes only where retained commands are
            // composed, not the phase of their local text coverage.
            let origin_key = physical_origin.map(|value| (value - value.floor()).to_bits());
            let text_mode = self.shared.text_rendering.get();
            if entry.text_drawn
                && (entry.text_mode != text_mode
                    || (text_mode != crate::TextRenderingMode::LineMask
                        && entry.text_origin != Some(origin_key)))
            {
                entry.paint_dirty.set(true);
            }
            if entry.paint_dirty.get() {
                let _timing = self.shared.profiler.span(
                    crate::ProfileOperation::Paint,
                    Some(serial),
                    &entry.key,
                );
                let mut focused = self.window.focus.as_ref().map(|id| id.0.clone());
                let mut focus_within = false;
                while let Some(node) = focused {
                    if node.serial == serial {
                        focus_within = true;
                        break;
                    }
                    focused = node.presentation_parent();
                }
                let mut painter = crate::Painter {
                    shapes: self.shared.shapes.clone(),
                    text: self.text.clone(),
                    font_reads: Cell::new(0),
                    text_drawn: Cell::new(false),
                    text_rendering: text_mode,
                    physical_origin,
                    physical_scale: viewport.scale() * f64::from(entry.node.scale.get()),
                    profiler: self.shared.profiler.clone(),
                    focus_within,
                    focus_read: Cell::new(false),
                    metrics: self.shared.theme.borrow().active_metrics(),
                    metric_roles: Cell::new(0),
                    palette,
                    roles: Cell::new(0),
                    size: entry.node.size.get(),
                    clip: None,
                    view_scale: entry.node.prepared_scale.get(),
                    window_scale: viewport.scale(),
                    list: crate::DisplayList {
                        viewport: viewport.with_scale(entry.node.raster_scale()),
                        ..Default::default()
                    },
                };
                entry.component.paint(&mut painter)?;
                let foreground = painter.list.fills.len();
                entry.component.paint_foreground(&mut painter)?;
                self.shared
                    .theme
                    .borrow_mut()
                    .subscribe(&entry.node, painter.roles.get());
                entry.node.focus_reader.set(painter.focus_read.get());
                entry.node.metric_paint.set(painter.metric_roles.get());
                entry.node.font_paint.set(painter.font_reads.get());
                entry.text_drawn = painter.text_drawn.get();
                entry.text_origin = Some(origin_key);
                entry.text_mode = text_mode;
                self.shared.font_subscribers.borrow_mut().track(&entry.node);
                if painter.metric_roles.get() != 0 {
                    self.shared.theme.borrow_mut().track_metrics(&entry.node);
                }
                entry.foreground = foreground;
                entry.drawing = painter.list.fills;
                entry.masks = painter.list.masks.into_items();
                entry.images = painter.list.images.into_items();
                entry.vectors = painter.list.vectors.into_items();
                entry.paint_dirty.set(false);
            }
            let commands = if foreground {
                &entry.drawing[entry.foreground..]
            } else {
                &entry.drawing[..entry.foreground]
            };
            if commands.is_empty() {
                continue;
            }
            // `composition_order` already left out the components an empty clip
            // removes, except one that still owed this recording. That one has
            // now made it, and its commands and coverage stay out of the scene.
            let clip = entry.node.clip.get();
            if clip.is_some_and(crate::Rect::is_empty) {
                continue;
            }
            // A resource enters the scene's deduplicating tables only when a
            // command that references it survives clipping, so the scene holds
            // what a backend has to upload and nothing a clip removed.
            let mut masks = vec![None; entry.masks.len()];
            let mut images = vec![None; entry.images.len()];
            let [bx, by, _, _] = entry
                .node
                .bounds
                .get()
                .map_or([0.0; 4], crate::Rect::values);
            // Local geometry reaches the window through this component's origin
            // and the accumulated view scale of its ancestors, 1 outside any view.
            let view = entry.node.scale.get();
            let origin = (f64::from(bx), f64::from(by));
            for command in commands {
                let place = |rect: crate::Rect| {
                    crate::transform::map_rect(rect, origin, view)
                        .ok_or(crate::GeometryError::NonFinite)
                };
                let command = match *command {
                    crate::DrawCommand::DrawVector {
                        vector,
                        origin: local_origin,
                        scale,
                        color,
                        clip: local_clip,
                    } => {
                        let local_clip = local_clip.map(place).transpose()?;
                        let clip = match (clip, local_clip) {
                            (Some(a), Some(b)) => Some(a.intersection(b)),
                            (a, b) => a.or(b),
                        };
                        if clip.is_some_and(crate::Rect::is_empty) {
                            continue;
                        }
                        let local = entry
                            .vectors
                            .get(vector.0)
                            .ok_or(crate::DrawError::InvalidImage)?;
                        let vector = crate::VectorId(scene.vectors.intern(local));
                        let origin = crate::Point::new(
                            (origin.0 + f64::from(view) * f64::from(local_origin.x())) as f32,
                            (origin.1 + f64::from(view) * f64::from(local_origin.y())) as f32,
                        )?;
                        crate::DrawCommand::DrawVector {
                            vector,
                            origin,
                            scale: scale * view,
                            color,
                            clip,
                        }
                    }
                    crate::DrawCommand::FillRect { rect, color } => {
                        let rect = place(rect)?;
                        let rect = clip.map_or(rect, |clip| rect.intersection(clip));
                        if rect.is_empty() {
                            continue;
                        }
                        crate::DrawCommand::FillRect { rect, color }
                    }
                    crate::DrawCommand::DrawMask {
                        rect,
                        mask,
                        color,
                        turn,
                        clip: local_clip,
                    } => {
                        let local_clip = local_clip.map(place).transpose()?;
                        let clip = match (clip, local_clip) {
                            (Some(a), Some(b)) => Some(a.intersection(b)),
                            (a, b) => a.or(b),
                        };
                        let rect = place(rect)?;
                        if rect.is_empty()
                            || clip.is_some_and(|clip| rect.intersection(clip).is_empty())
                        {
                            continue;
                        }
                        let slot = masks
                            .get_mut(mask.0)
                            .ok_or(crate::DrawError::InvalidImage)?;
                        let local = entry
                            .masks
                            .get(mask.0)
                            .ok_or(crate::DrawError::InvalidImage)?;
                        let mask = *slot.get_or_insert_with(|| scene.masks.intern(local));
                        crate::DrawCommand::DrawMask {
                            rect,
                            mask: crate::MaskId(mask),
                            color,
                            turn,
                            clip,
                        }
                    }
                    crate::DrawCommand::DrawImage {
                        rect,
                        image,
                        tint,
                        sampling,
                        turn,
                        clip: local_clip,
                    } => {
                        let local_clip = local_clip.map(place).transpose()?;
                        let clip = match (clip, local_clip) {
                            (Some(a), Some(b)) => Some(a.intersection(b)),
                            (a, b) => a.or(b),
                        };
                        let rect = place(rect)?;
                        if rect.is_empty()
                            || clip.is_some_and(|clip| rect.intersection(clip).is_empty())
                        {
                            continue;
                        }
                        let slot = images
                            .get_mut(image.0)
                            .ok_or(crate::DrawError::InvalidImage)?;
                        let local = entry
                            .images
                            .get(image.0)
                            .ok_or(crate::DrawError::InvalidImage)?;
                        let image = *slot.get_or_insert_with(|| scene.images.intern(local));
                        crate::DrawCommand::DrawImage {
                            rect,
                            image: crate::ImageId(image),
                            tint,
                            sampling,
                            turn,
                            clip,
                        }
                    }
                };
                scene.fills.push(command);
            }
        }
        #[cfg(debug_assertions)]
        if overlay {
            self.compose_overlay(&mut scene);
        }
        #[cfg(not(debug_assertions))]
        let _ = overlay;
        self.window.shared.scene_dirty.set(false);
        Ok(scene)
    }

    /// Whether visible drawing or the scene background needs fresh composition.
    ///
    /// Custom hosts check this after events/flushes, then call `paint` after pending
    /// updates are flushed. Successful painting clears scene damage. Hidden component
    /// dirtiness alone does not request presentation. This query creates no timers.
    #[must_use]
    pub fn needs_paint(&self) -> bool {
        !self.window.focus_errors.is_empty()
            || self.window.shared.scene_dirty.get()
            || self
                .shared
                .captures
                .borrow()
                .iter()
                .any(|capture| capture.window == self.window.shared.serial)
            || self.window.restore_focus
            || self.window.shared.stack_hover_pending.get()
            || !self.shared.requested_removals.borrow().is_empty()
            || !self.shared.requested_moves.borrow().is_empty()
            || !self.shared.requested_children.borrow().is_empty()
            || !self.shared.requested_text_commands.borrow().is_empty()
            || self.window.shared.layout_pending.get()
            || !self.window.shared.pending.borrow().is_empty()
            || self.entries.values().any(|entry| {
                Rc::ptr_eq(&entry.node.window(), &self.window.shared)
                    && entry.node.is_presented()
                    && entry.paint_dirty.get()
            })
    }
}
