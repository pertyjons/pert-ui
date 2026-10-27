//! CPU encoding retention; unchanged command slots retain their encoded quad.
use super::*;

#[derive(Default)]
pub(super) struct RetainedGeometry {
    viewport: Option<pert_ui_runtime::Viewport>,
    commands: Vec<DrawCommand>,
    chunks: Vec<(std::sync::Arc<[u8]>, Option<Batch>)>,
    pub(super) bytes: Vec<u8>,
    pub(super) batches: Vec<Batch>,
    #[cfg(test)]
    encoded_commands: usize,
}
impl RetainedGeometry {
    pub(super) fn prepare(&self, scene: &DisplayList) -> Result<Self, RenderError> {
        let mut next = Self {
            viewport: Some(scene.viewport()),
            bytes: Vec::with_capacity(self.bytes.len()),
            chunks: Vec::with_capacity(scene.commands().len()),
            commands: Vec::with_capacity(scene.commands().len()),
            ..Default::default()
        };
        for (index, command) in scene.commands().iter().enumerate() {
            let chunk =
                if self.viewport == next.viewport && self.commands.get(index) == Some(command) {
                    self.chunks[index].clone()
                } else {
                    #[cfg(test)]
                    {
                        next.encoded_commands += 1;
                    }
                    let (bytes, batches) = geometry_commands(scene, std::slice::from_ref(command))?;
                    (bytes.into(), batches.into_iter().next())
                };
            let offset =
                u32::try_from(next.bytes.len() / 32).map_err(|_| LimitError::VertexCount)?;
            if let Some(batch) = &chunk.1 {
                let start = offset
                    .checked_add(batch.range.start)
                    .ok_or(LimitError::VertexCount)?;
                let end = offset
                    .checked_add(batch.range.end)
                    .ok_or(LimitError::VertexCount)?;
                if let Some(last) = next.batches.last_mut()
                    && last.source == batch.source
                {
                    last.range.end = end;
                } else {
                    next.batches.push(Batch {
                        range: start..end,
                        source: batch.source,
                    });
                }
            }
            next.bytes.extend_from_slice(&chunk.0);
            next.commands.push(*command);
            next.chunks.push(chunk);
        }
        Ok(next)
    }
}
// Queue writes require 4-byte alignment. Deletion at the end needs no upload:
// draw ranges, not stale capacity, decide which vertices are visible.
pub(super) fn changed_bytes(before: &[u8], after: &[u8]) -> std::ops::Range<usize> {
    let prefix = before.iter().zip(after).take_while(|(a, b)| a == b).count();
    if prefix == after.len() {
        return 0..0;
    }
    let suffix = if before.len() == after.len() {
        before[prefix..]
            .iter()
            .rev()
            .zip(after[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count()
    } else {
        0
    };
    let start = prefix / 4 * 4;
    let end = (after.len() - suffix).div_ceil(4) * 4;
    start..end.min(after.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pert_ui_runtime::*;
    use std::time::Duration;
    struct Scene {
        dynamic: ReadSignal<f32>,
    }
    impl Component for Scene {
        fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
            cx.read_paint(&self.dynamic)?;
            Ok(())
        }
        fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
            for i in 0..1000 {
                p.fill_rect(
                    Rect::new((i % 100) as f32 * 4.0, (i / 100) as f32 * 4.0, 2.0, 2.0)?,
                    Color::WHITE,
                );
            }
            p.fill_rect(
                Rect::new(self.dynamic.get()?, 50.0, 3.0, 3.0)?,
                Color::ACCENT,
            );
            Ok(())
        }
    }
    #[test]
    fn mixed_scene_reencodes_and_uploads_only_the_changing_quad()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(500.0, 100.0, 1.0)?)?;
        let dynamic = ui.signal(0.0);
        ui.mount_tree(Element::new(
            "scene",
            Scene {
                dynamic: dynamic.read_only(),
            },
        ))?;
        ui.flush(Duration::ZERO)?;
        let mut cache = RetainedGeometry::default().prepare(&ui.paint()?)?;
        assert_eq!(cache.encoded_commands, 1001);
        let start = std::time::Instant::now();
        for i in 1..61 {
            dynamic.set(i as f32)?;
            ui.flush(Duration::ZERO)?;
            let scene = ui.paint()?;
            let next = cache.prepare(&scene)?;
            assert_eq!(next.encoded_commands, 1);
            assert_eq!(
                (next.bytes.clone(), next.batches.clone()),
                geometry(&scene)?
            );
            let upload = changed_bytes(&cache.bytes, &next.bytes);
            assert!(upload.start >= 1000 * 6 * 32);
            assert!(upload.len() <= 6 * 32);
            cache = next;
        }
        eprintln!(
            "mixed-scene CPU contract, 60 changes: {:?}; one quad encoded and <=192 bytes uploaded per change",
            start.elapsed()
        );
        Ok(())
    }
    // A scene of unit squares at the given x positions in a `width`×10 viewport.
    fn squares(width: f32, xs: &[f32]) -> Result<DisplayList, Box<dyn std::error::Error>> {
        struct Squares(Vec<f32>);
        impl Component for Squares {
            fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
                Ok(())
            }
            fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
                for &x in &self.0 {
                    p.fill_rect(Rect::new(x, 0.0, 1.0, 1.0)?, Color::WHITE);
                }
                Ok(())
            }
        }
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(width, 10.0, 1.0)?)?;
        ui.mount_tree(Element::new("squares", Squares(xs.to_vec())))?;
        ui.flush(Duration::ZERO)?;
        Ok(ui.paint()?)
    }

    #[test]
    fn slots_are_reused_until_their_command_or_the_viewport_changes()
    -> Result<(), Box<dyn std::error::Error>> {
        let cache = RetainedGeometry::default().prepare(&squares(10.0, &[0.0, 2.0, 4.0])?)?;
        assert_eq!(cache.encoded_commands, 3);
        // Appending keeps every existing slot.
        let scene = squares(10.0, &[0.0, 2.0, 4.0, 6.0])?;
        let appended = cache.prepare(&scene)?;
        assert_eq!(appended.encoded_commands, 1);
        assert_eq!(
            (appended.bytes.clone(), appended.batches.clone()),
            geometry(&scene)?
        );
        // Inserting at the front shifts, and so re-encodes, every later slot.
        let scene = squares(10.0, &[8.0, 0.0, 2.0, 4.0, 6.0])?;
        let inserted = appended.prepare(&scene)?;
        assert_eq!(inserted.encoded_commands, 5);
        assert_eq!(
            (inserted.bytes.clone(), inserted.batches.clone()),
            geometry(&scene)?
        );
        // A viewport change invalidates all geometry, even for equal commands.
        let scene = squares(20.0, &[8.0, 0.0, 2.0, 4.0, 6.0])?;
        let resized = inserted.prepare(&scene)?;
        assert_eq!(resized.encoded_commands, 5);
        assert_eq!(
            (resized.bytes.clone(), resized.batches.clone()),
            geometry(&scene)?
        );
        // An empty scene keeps nothing; its successor encodes from scratch.
        let empty = resized.prepare(&squares(20.0, &[])?)?;
        assert!(empty.bytes.is_empty() && empty.batches.is_empty());
        assert_eq!(empty.prepare(&squares(20.0, &[0.0])?)?.encoded_commands, 1);
        Ok(())
    }

    #[test]
    fn adjacent_equal_sources_merge_into_one_batch_across_slots()
    -> Result<(), Box<dyn std::error::Error>> {
        let cache = RetainedGeometry::default().prepare(&squares(10.0, &[0.0, 2.0])?)?;
        let next = cache.prepare(&squares(10.0, &[0.0, 2.0, 4.0])?)?;
        assert_eq!(next.batches.len(), 1);
        assert_eq!(next.batches[0].range, 0..18);
        Ok(())
    }

    #[test]
    fn changed_ranges_handle_shrink_growth_and_alignment() {
        assert_eq!(changed_bytes(&[1; 8], &[1; 4]), 0..0);
        assert_eq!(changed_bytes(&[1; 4], &[1; 8]), 4..8);
        assert_eq!(changed_bytes(&[0; 8], &[0, 1, 0, 0, 0, 0, 0, 0]), 0..4);
    }
}
