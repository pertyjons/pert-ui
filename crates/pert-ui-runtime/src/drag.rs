//! Drag and drop between components of one runtime.
//!
//! A captured pointer gesture has one participant. A drag session has two: the
//! source that started it and whatever accepts it under the pointer. The
//! runtime owns the session, because capture hides the second participant from
//! every widget; `docs/contracts.md` records the contract.
use std::{any::Any, rc::Rc};

/// Window-logical distance the pointer travels from an armed press before a
/// built-in source starts a drag; below it a release is still a click.
///
/// It describes the hand, not the content, so it is never scaled by a
/// [view transform](crate::ViewTransform). Custom sources should use it too, so
/// every drag in an application starts alike.
pub const DRAG_THRESHOLD: f32 = 4.0;

/// The application-typed value a drag carries, owned by the session.
///
/// One value of any `'static` type. Targets read it by reference with
/// [`Self::downcast_ref`] and reject what they do not recognize, which is the whole
/// compatibility protocol: inside one application an `enum` expresses
/// alternatives and the compiler checks it. The payload never leaves the UI
/// thread, is never serialized, and never enters inspection or event history.
///
/// **Carry application identities, never positions.** A drag lasts long enough
/// for live data, a worker result or an undo to reorder the collection under
/// it, so name *which* items by the application's stable keys and never by a
/// row index, a display order or a [`WidgetId`](crate::WidgetId).
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::DragPayload;
/// struct Items(Vec<String>);
/// let payload = DragPayload::new(Items(vec!["report".into()]));
/// assert_eq!(payload.downcast_ref::<Items>().map(|items| items.0.len()), Some(1));
/// assert!(payload.downcast_ref::<u32>().is_none());
/// ```
#[derive(Clone)]
pub struct DragPayload {
    value: Rc<dyn Any>,
    type_name: &'static str,
}

impl DragPayload {
    /// Wraps one application value.
    pub fn new<T: 'static>(value: T) -> Self {
        Self {
            value: Rc::new(value),
            type_name: std::any::type_name::<T>(),
        }
    }
    /// The value, when it is a `T`.
    #[must_use]
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.value.downcast_ref()
    }
    /// The payload's Rust type name, for diagnostics only; not a stable identity.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
}

impl std::fmt::Debug for DragPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DragPayload")
            .field("type_name", &self.type_name)
            .finish_non_exhaustive()
    }
}

/// What a completed drop does with the payload.
///
/// With the `serde` feature it implements `Serialize` and `Deserialize`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum DropOperation {
    /// The items leave the source and arrive at the target. The application
    /// performs this as **one** edit on the target's side; the source does not
    /// delete what the target inserted.
    Move,
    /// The items stay and a copy arrives at the target.
    Copy,
    /// The target refers to the items without taking them.
    Link,
}

impl DropOperation {
    const fn bit(self) -> u8 {
        match self {
            Self::Move => 1,
            Self::Copy => 2,
            Self::Link => 4,
        }
    }
}

/// The set of operations a source permits.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{DropOperation, DropOperations, Modifiers};
/// let permitted = DropOperations::MOVE | DropOperations::COPY;
/// assert!(permitted.contains(DropOperation::Copy));
/// assert!(!permitted.contains(DropOperation::Link));
/// assert_eq!(permitted.preferred(Modifiers::default()), Some(DropOperation::Move));
/// assert_eq!(DropOperations::NONE.preferred(Modifiers::default()), None);
/// ```
///
/// # Serialization
///
/// With the `serde` feature a set is the list of the operations it permits, in
/// move, copy, link order, such as `["Move", "Copy"]`; a repeated operation
/// reads as permitted once.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct DropOperations(u8);

impl DropOperations {
    /// Nothing; a session cannot start with it.
    pub const NONE: Self = Self(0);
    /// [`DropOperation::Move`] only.
    pub const MOVE: Self = Self(1);
    /// [`DropOperation::Copy`] only.
    pub const COPY: Self = Self(2);
    /// [`DropOperation::Link`] only.
    pub const LINK: Self = Self(4);
    /// Every operation.
    pub const ALL: Self = Self(7);

    /// Whether the set permits `operation`.
    #[must_use]
    pub const fn contains(self, operation: DropOperation) -> bool {
        self.0 & operation.bit() != 0
    }
    /// Whether the set permits nothing.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
    /// The operation a target without an opinion should perform, from this set
    /// and the held modifiers, following the platform's convention.
    ///
    /// The copy modifier is Control, or Option on macOS; with Shift (Command on
    /// macOS) it asks for a link. A requested operation the set does not permit
    /// falls back to the first permitted of move, copy and link. `None` only for
    /// the empty set.
    #[must_use]
    pub fn preferred(self, modifiers: crate::Modifiers) -> Option<DropOperation> {
        let (copy, link) = if cfg!(target_os = "macos") {
            (modifiers.alt, modifiers.alt && modifiers.super_key)
        } else {
            (modifiers.control, modifiers.control && modifiers.shift)
        };
        let wanted = if link {
            DropOperation::Link
        } else if copy {
            DropOperation::Copy
        } else {
            DropOperation::Move
        };
        [
            wanted,
            DropOperation::Move,
            DropOperation::Copy,
            DropOperation::Link,
        ]
        .into_iter()
        .find(|operation| self.contains(*operation))
    }
}

impl std::ops::BitOr for DropOperations {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl From<DropOperation> for DropOperations {
    fn from(operation: DropOperation) -> Self {
        Self(operation.bit())
    }
}

/// What follows the pointer during a session.
///
/// [`DragPreview::none`] is for a drag the participants draw themselves, such as
/// a cable a canvas draws from a port to the pointer, or a reorder shown only by
/// an insertion line. [`DragPreview::element`] mounts ordinary components for
/// the session, above every other view and surface, outside hit testing, focus
/// and keyboard navigation, and unmounts them when the session ends. The preview
/// is never scaled by a [view transform](crate::ViewTransform) and is clipped by
/// the window rather than kept inside it.
#[must_use]
pub struct DragPreview {
    pub(crate) element: Option<crate::Element>,
    pub(crate) hot_spot: crate::Point,
}
description_debug!(DragPreview);

impl DragPreview {
    /// No preview.
    pub fn none() -> Self {
        Self {
            element: None,
            hot_spot: crate::Point::ZERO,
        }
    }
    /// A preview built from an element, measured at its preferred size. Its
    /// top-left sits at the pointer until [`Self::hot_spot`] says otherwise.
    pub fn element(element: crate::Element) -> Self {
        Self {
            element: Some(element),
            hot_spot: crate::Point::ZERO,
        }
    }
    /// The point of the preview, from its top-left in logical pixels, that stays
    /// under the pointer. Negative values hold the preview away from the pointer.
    pub fn hot_spot(mut self, hot_spot: crate::Point) -> Self {
        self.hot_spot = hot_spot;
        self
    }
}

/// Where a drag event sits in a target's part of the session.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DragPhase {
    /// The pointer is over this component, which is not the current target: does
    /// it want the drag? A component that rejects owes nothing and is simply
    /// asked again as the pointer moves; one that accepts or forbids becomes the
    /// target.
    Enter,
    /// The pointer moved, a modifier changed or layout changed while this
    /// component is the target (or an observer). Coalesced like pointer motion.
    /// The target may change its answer, for example to forbid part of itself.
    Over,
    /// The session left this component without a drop: the pointer moved away,
    /// another component claimed the drag, or the session was cancelled. Remove
    /// every drop indication and cancel any auto-scroll timer.
    Leave,
    /// The button was released over this target while it accepted. Perform the
    /// application edit now; the response reports whether it happened.
    Drop,
}

/// Whether a component is asked for an answer or only told.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DragRole {
    /// The component may claim the drag; its response counts.
    Candidate,
    /// An ancestor of the current target. It receives [`DragPhase::Over`] and
    /// [`DragPhase::Leave`] so that, for example, a scroll container can scroll
    /// while the pointer rests near its edge. Its response is ignored.
    Observer,
}

/// One delivery of [`Component::drag_event`](crate::Component::drag_event).
#[derive(Debug)]
pub struct DragEvent<'a> {
    pub(crate) phase: DragPhase,
    pub(crate) role: DragRole,
    pub(crate) position: crate::Point,
    pub(crate) payload: &'a DragPayload,
    pub(crate) permitted: DropOperations,
    pub(crate) preferred: DropOperation,
    pub(crate) source: &'a crate::WidgetId,
}

impl DragEvent<'_> {
    /// Where the event sits in this component's part of the session.
    #[must_use]
    pub const fn phase(&self) -> DragPhase {
        self.phase
    }
    /// Whether the response counts.
    #[must_use]
    pub const fn role(&self) -> DragRole {
        self.role
    }
    /// Pointer position in window-logical coordinates, the last known one for
    /// [`DragPhase::Leave`]. Convert with
    /// [`EventContext::to_local`](crate::EventContext::to_local).
    #[must_use]
    pub const fn position(&self) -> crate::Point {
        self.position
    }
    /// What is being dragged.
    #[must_use]
    pub const fn payload(&self) -> &DragPayload {
        self.payload
    }
    /// The operations the source permits.
    #[must_use]
    pub const fn permitted(&self) -> DropOperations {
        self.permitted
    }
    /// The permitted operation the held modifiers ask for; accept with this
    /// unless the target has its own rule.
    #[must_use]
    pub const fn preferred(&self) -> DropOperation {
        self.preferred
    }
    /// The component that started the session, which may be this one.
    #[must_use]
    pub const fn source(&self) -> &crate::WidgetId {
        self.source
    }
}

/// A component's answer to a [`DragEvent`].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum DropResponse {
    /// Not mine: not this payload, or not this component's business. The runtime
    /// asks the next ancestor, and a current target that answers this gives the
    /// drag up and receives [`DragPhase::Leave`]. For [`DragPhase::Drop`]: the
    /// drop was refused, for example because the application rejected the edit.
    #[default]
    Rejected,
    /// Mine, but not here: this component claims the drag so that nothing behind
    /// or above it is asked, and a release cancels. The cursor shows that nothing
    /// can be dropped, which is how a locked or full target answers. For
    /// [`DragPhase::Drop`] it means the same as [`Self::Rejected`].
    Forbidden,
    /// A drop here would perform this operation, which must be one the source
    /// permits. For [`DragPhase::Drop`]: it was performed.
    Accepted(DropOperation),
}

/// How a session ended, as told to its source.
///
/// Endings are added as drag sessions gain features, so matches need a wildcard
/// arm; most sources treat an unknown ending like [`Self::Cancelled`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum DragOutcome {
    /// A target performed this operation.
    Dropped(DropOperation),
    /// The button was released over an accepting target, which then refused.
    Refused,
    /// The last button was released where nothing claimed the drag: over no
    /// target, or outside the window. Nothing was dropped. Most sources treat
    /// this like [`Self::Cancelled`]; a source that gives a release in empty
    /// space its own meaning, as a detachable [`Tabs`](crate::Tabs) strip does by
    /// moving the tab into its own window, acts on it here.
    Unclaimed,
    /// Cancelled without a drop: released over a target that forbids the drop,
    /// Escape, focus loss, a blocking modal surface, an error, or the source
    /// losing its input context.
    Cancelled,
}

/// Owned observations of the active drag session, captured on the UI thread.
/// The payload itself is never included.
///
/// With the `serde` feature it implements `Serialize` and `Deserialize`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct DragInspection {
    /// The component that started the session.
    pub source: u64,
    /// Native presentation root the session is confined to.
    pub window: u64,
    /// The component that currently claims the drag, if any.
    pub target: Option<u64>,
    /// The operation the target accepts; `None` while it forbids the drop or
    /// there is no target.
    pub accepted: Option<DropOperation>,
    /// The operations the source permits.
    pub permitted: DropOperations,
    /// The payload's Rust type name, for diagnostics only.
    pub payload_type: String,
    /// Root of the mounted preview, if the session has one.
    pub preview: Option<u64>,
}

// The serde representation; the public type documents it.
#[cfg(feature = "serde")]
mod serde_impls {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::{DropOperation, DropOperations};

    impl Serialize for DropOperations {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let permitted: Vec<DropOperation> = [
                DropOperation::Move,
                DropOperation::Copy,
                DropOperation::Link,
            ]
            .into_iter()
            .filter(|operation| self.contains(*operation))
            .collect();
            permitted.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for DropOperations {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let bits = Vec::<DropOperation>::deserialize(deserializer)?
                .into_iter()
                .fold(0, |bits, operation| bits | operation.bit());
            Ok(Self(bits))
        }
    }
}
