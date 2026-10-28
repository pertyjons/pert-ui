//! The options menu in a tool window's panel header: move the tool window to
//! another edge or group, reorder it in its stripe, switch it between docked
//! and overlay, and dock, float or detach it.
use super::model::{self, Link, Model, Window};
use super::{EdgeMode, StripeGroup, ToolEdge, ToolWindowMode};
use crate::*;
use std::rc::Rc;

pub(super) struct Options {
    link: Rc<Link>,
    key: String,
    title: Caption,
    /// The edges holding panels, the string table and the title when the
    /// menu was built; the menu is built again when one changes, and after a
    /// re-description.
    built: Option<(Vec<ToolEdge>, UiStrings, String)>,
    generation: u64,
}
impl Options {
    pub(super) fn new(link: Rc<Link>, key: String, title: TextSource) -> Self {
        Self {
            link,
            key,
            title: Caption::new(title),
            built: None,
            generation: 0,
        }
    }
}

/// Runs a user change from the menu when its permission allows it now.
fn act(
    link: &Link,
    key: &str,
    permission: fn(&Model, &str) -> Permission,
    change: impl FnOnce(&mut Model) -> bool,
) -> Result<(), RuntimeError> {
    let model = link.model();
    let allowed = permission(&model.borrow(), key);
    if !allowed.get()? {
        return Ok(());
    }
    model::check_writable(&model)?;
    let changed = change(&mut model.borrow_mut());
    if changed {
        model::touch(&model)?;
    }
    Ok(())
}

/// Builds the menu's items. Each row's enabled state follows the model
/// through the shared revision and the permission's bindings, so the menu
/// is not rebuilt, and its trigger keeps focus, when the tool window moves.
struct Builder<'a> {
    link: &'a Rc<Link>,
    strings: &'a UiStrings,
    key: &'a str,
    revision: ReadSignal<u64>,
    /// The permission to float, and to detach, as bindings.
    floatable: ReadSignal<bool>,
    detachable: ReadSignal<bool>,
}
impl Builder<'_> {
    /// Whether a state check on the tool window holds, as a binding.
    fn holds(&self, check: fn(&Model, &Window) -> bool) -> Result<ReadSignal<bool>, RuntimeError> {
        let (link, key) = (self.link.clone(), self.key.to_owned());
        self.revision.map(move |_| {
            let model = link.model();
            let model = model.borrow();
            model
                .window(&key)
                .is_some_and(|window| check(&model, window))
        })
    }
    /// Disables `item` unless `check` holds and `permission` allows.
    fn enable(
        &self,
        item: MenuItem,
        permission: &Permission,
        check: fn(&Model, &Window) -> bool,
    ) -> Result<MenuItem, RuntimeError> {
        if permission.fixed() == Some(false) {
            return Ok(item.disabled());
        }
        let holds = self.holds(check)?;
        let enabled = match permission.signal()? {
            Some(permission) => {
                ReadSignal::combine((&holds, &permission), |(holds, allowed)| *holds && *allowed)?
            }
            None => holds,
        };
        Ok(item.enabled(enabled))
    }
}

/// The tool window's position in its group and the group's size.
fn position(model: &Model, window: &Window) -> (usize, usize) {
    let keys = model.group_keys(window.edge, window.group);
    let position = keys.iter().position(|key| key == &window.key).unwrap_or(0);
    (position, keys.len())
}

/// Whether the user may switch from `current` to `target` with the float
/// and detach permissions `float` and `detach`: leaving or entering floating
/// needs the first, leaving or entering a native window the second.
fn may_switch(current: ToolWindowMode, target: ToolWindowMode, float: bool, detach: bool) -> bool {
    let need = |mode| match mode {
        ToolWindowMode::Floating => float,
        ToolWindowMode::Window => detach,
        _ => true,
    };
    current != target && need(current) && need(target)
}

/// A dock, float or open-in-window row: enabled while the tool window is in
/// another mode and the permissions allow the switch, checked while it is in
/// this mode.
fn mode_item(
    builder: &Builder<'_>,
    target: ToolWindowMode,
    item_key: &str,
    label: &str,
) -> Result<MenuItem, RuntimeError> {
    let (link, key) = (builder.link.clone(), builder.key.to_owned());
    let current = builder.revision.map(move |_| {
        link.model()
            .borrow()
            .window(&key)
            .map_or(ToolWindowMode::Docked, |window| window.mode)
    })?;
    let enabled = ReadSignal::combine(
        (&current, &builder.floatable, &builder.detachable),
        move |(mode, float, detach)| may_switch(*mode, target, *float, *detach),
    )?;
    let checked = current.map(move |mode| *mode == target)?;
    let (action_link, action_key) = (builder.link.clone(), builder.key.to_owned());
    Ok(MenuItem::new(item_key, label, move |_| {
        let model = action_link.model();
        let allowed = model.borrow().mode_permission(&action_key, target);
        if !allowed.get()? {
            return Ok(());
        }
        model::check_writable(&model)?;
        let changed = model
            .borrow_mut()
            .set_mode(&action_key, target, ChangeOrigin::User);
        if changed {
            model::touch(&model)?;
        }
        Ok(())
    })
    .checked_signal(checked)
    .enabled(enabled))
}

fn items(
    builder: &Builder<'_>,
    edges: &[ToolEdge],
    strings: &UiStrings,
) -> Result<Vec<MenuItem>, RuntimeError> {
    let (link, key) = (builder.link, builder.key);
    let (movable, reorderable, overlayable) = {
        let model = link.model();
        let model = model.borrow();
        (
            model.move_permission(key),
            model.reorder_permission(key),
            model.overlay_permission(key),
        )
    };
    let mut items = Vec::new();
    for edge in edges.iter().copied() {
        let (action_link, action_key) = (link.clone(), key.to_owned());
        let item = MenuItem::new(
            format!("move-{}", edge.name()),
            builder.strings.text(edge.move_role()),
            move |_| {
                act(&action_link, &action_key, Model::move_permission, |model| {
                    let group = model
                        .window(&action_key)
                        .map_or(StripeGroup::Start, |w| w.group);
                    model.move_window(&action_key, edge, group, None, ChangeOrigin::User)
                })
            },
        );
        let check: fn(&Model, &Window) -> bool = match edge {
            ToolEdge::Left => |_, window| window.edge != ToolEdge::Left,
            ToolEdge::Right => |_, window| window.edge != ToolEdge::Right,
            ToolEdge::Top => |_, window| window.edge != ToolEdge::Top,
            ToolEdge::Bottom => |_, window| window.edge != ToolEdge::Bottom,
        };
        items.push(builder.enable(item, &movable, check)?);
    }
    for group in [StripeGroup::Start, StripeGroup::End] {
        let (action_link, action_key) = (link.clone(), key.to_owned());
        let item = MenuItem::new(
            format!("group-{}", group.name()),
            builder.strings.text(match group {
                StripeGroup::Start => StringRole::ToolWindowMoveStartGroup,
                StripeGroup::End => StringRole::ToolWindowMoveEndGroup,
            }),
            move |_| {
                act(&action_link, &action_key, Model::move_permission, |model| {
                    let edge = model.window(&action_key).map_or(ToolEdge::Left, |w| w.edge);
                    model.move_window(&action_key, edge, group, None, ChangeOrigin::User)
                })
            },
        );
        let check: fn(&Model, &Window) -> bool = match group {
            StripeGroup::Start => |_, window| window.group != StripeGroup::Start,
            StripeGroup::End => |_, window| window.group != StripeGroup::End,
        };
        items.push(builder.enable(item, &movable, check)?);
    }
    for (item_key, label, earlier) in [
        ("earlier", StringRole::ToolWindowMoveEarlier, true),
        ("later", StringRole::ToolWindowMoveLater, false),
    ] {
        let (action_link, action_key) = (link.clone(), key.to_owned());
        let item = MenuItem::new(item_key, builder.strings.text(label), move |_| {
            act(
                &action_link,
                &action_key,
                Model::reorder_permission,
                |model| {
                    let Some(window) = model.window(&action_key) else {
                        return false;
                    };
                    let (edge, group) = (window.edge, window.group);
                    let (position, count) = position(model, window);
                    let target = if earlier {
                        position.checked_sub(1)
                    } else {
                        (position + 1 < count).then_some(position + 1)
                    };
                    target.is_some_and(|target| {
                        model.move_window(
                            &action_key,
                            edge,
                            group,
                            Some(target),
                            ChangeOrigin::User,
                        )
                    })
                },
            )
        });
        let check: fn(&Model, &Window) -> bool = if earlier {
            |model, window| position(model, window).0 > 0
        } else {
            |model, window| {
                let (position, count) = position(model, window);
                position + 1 < count
            }
        };
        items.push(builder.enable(item, &reorderable, check)?);
    }
    items.push(MenuItem::separator());
    let (action_link, action_key) = (link.clone(), key.to_owned());
    let mode = MenuItem::new(
        "overlay",
        builder.strings.text(StringRole::ToolWindowOverlay),
        move |_| {
            act(
                &action_link,
                &action_key,
                Model::overlay_permission,
                |model| {
                    let overlay = model.window(&action_key).is_some_and(|w| w.overlay);
                    model.set_overlay(&action_key, !overlay, ChangeOrigin::User)
                },
            )
        },
    )
    .checked_signal(builder.holds(|_, window| window.overlay)?);
    items.push(builder.enable(mode, &overlayable, |_, window| {
        window.mode == ToolWindowMode::Docked
    })?);
    items.push(MenuItem::separator());
    for (target, item_key, role) in [
        (ToolWindowMode::Docked, "dock", StringRole::ToolWindowDock),
        (
            ToolWindowMode::Floating,
            "float",
            StringRole::ToolWindowFloat,
        ),
        (
            ToolWindowMode::Window,
            "window",
            StringRole::ToolWindowOpenWindow,
        ),
    ] {
        items.push(mode_item(builder, target, item_key, strings.text(role))?);
    }
    Ok(items)
}

impl Component for Options {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        // A new description may change titles and permissions: build again.
        self.title.reconcile(next.title);
        self.built = None;
        Some(Invalidation::Layout)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let model = self.link.model();
        let revision = model::revision(&model, cx);
        cx.read(&revision)?;
        let edges: Vec<ToolEdge> = {
            let model = model.borrow();
            ToolEdge::ALL
                .into_iter()
                .filter(|edge| *model.edges.get(*edge) != EdgeMode::None)
                .collect()
        };
        let strings = cx.strings()?;
        self.title.update_paint(cx)?;
        let built = (edges.clone(), strings.clone(), self.title.text().to_owned());
        if self.built.as_ref() == Some(&built) {
            return Ok(());
        }
        let (floatable, detachable) = {
            let model = model.borrow();
            (
                model.float_permission(&self.key),
                model.detach_permission(&self.key),
            )
        };
        let binding = |permission: Permission| -> Result<ReadSignal<bool>, RuntimeError> {
            Ok(match permission.signal()? {
                Some(signal) => signal,
                None => cx.signal(permission.fixed() != Some(false)).read_only(),
            })
        };
        let builder = Builder {
            link: &self.link,
            strings: &strings,
            key: &self.key,
            revision: revision.read_only(),
            floatable: binding(floatable)?,
            detachable: binding(detachable)?,
        };
        let menu = Menu::icon(
            cx,
            Icon::MoreVertical,
            strings.format(UiMessage::ToolWindowOptions {
                title: self.title.text(),
            }),
            items(&builder, &edges, &strings)?,
        )?;
        self.generation = self.generation.wrapping_add(1);
        cx.request_children(vec![Element::new(
            format!("menu-{}", self.generation),
            menu,
        )]);
        // The header arranges around this component's size, which the menu
        // trigger now gives it.
        cx.request_layout();
        self.built = Some(built);
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        if cx.is_empty() {
            return Ok(Size::ZERO);
        }
        cx.measure(0, Constraints::new(None, available.height())?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            let rect = Rect::new(0.0, 0.0, size.width(), size.height())?;
            cx.place_clipped(0, rect, rect)?;
        }
        Ok(())
    }
}
