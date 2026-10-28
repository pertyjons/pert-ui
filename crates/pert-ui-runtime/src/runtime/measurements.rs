//! A component's retained measurements: the few most recently used
//! `(constraints, size)` results, kept until its layout is invalidated.
use crate::{Constraints, Size};

/// How many measurements a component keeps. A row asks a child it shrinks for
/// two constraints per computation, and a row in a column computes at two
/// heights (measured unbounded, arranged at its allocation), so four cover
/// both without recomputing; a component asked for more computes again
/// whatever was replaced, with the same result.
pub(crate) const RETAINED: usize = 4;

/// Up to [`RETAINED`] measurements, most recently used first. Fixed size and
/// inline: nothing is allocated, aged or swept. The component's measurement
/// dependency masks live beside it on the node and are never reduced by an
/// eviction, so a result computed from an evicted entry anywhere stays covered.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Measurements {
    entries: [Option<(Constraints, Size)>; RETAINED],
}

impl Measurements {
    /// The size measured at `constraints`, made the most recently used.
    pub(crate) fn get(&mut self, constraints: Constraints) -> Option<Size> {
        let position = self
            .entries
            .iter()
            .position(|entry| entry.is_some_and(|(cached, _)| cached == constraints))?;
        self.entries[..=position].rotate_right(1);
        self.entries[0].map(|(_, size)| size)
    }
    /// Records a measurement as the most recently used, replacing the least
    /// recently used one when every entry is taken.
    pub(crate) fn insert(&mut self, constraints: Constraints, size: Size) {
        let position = self
            .entries
            .iter()
            .position(|entry| entry.is_none_or(|(cached, _)| cached == constraints))
            .unwrap_or(RETAINED - 1);
        self.entries[..=position].rotate_right(1);
        self.entries[0] = Some((constraints, size));
    }
    /// The most recently used measurement's size: the one the last
    /// measurement returned.
    #[cfg(debug_assertions)]
    pub(crate) fn latest(&self) -> Option<Size> {
        self.entries[0].map(|(_, size)| size)
    }
    /// Whether no measurement is retained.
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.iter().all(Option::is_none)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn width(value: f32) -> Constraints {
        Constraints::new(Some(value), None).unwrap_or_default()
    }
    fn size(value: f32) -> Size {
        Size::new(value, 1.0).unwrap_or(Size::ZERO)
    }

    #[test]
    fn the_least_recently_used_measurement_is_replaced() {
        let mut store = Measurements::default();
        for value in [1.0, 2.0, 3.0, 4.0] {
            store.insert(width(value), size(value));
        }
        // A hit refreshes A, so the fifth insertion replaces B.
        assert_eq!(store.get(width(1.0)), Some(size(1.0)));
        store.insert(width(5.0), size(5.0));
        assert_eq!(store.get(width(2.0)), None, "B was the least recently used");
        for value in [1.0, 3.0, 4.0, 5.0] {
            assert_eq!(store.get(width(value)), Some(size(value)), "{value} kept");
        }
        // Equal constraints replace their entry instead of taking another.
        store.insert(width(3.0), size(30.0));
        assert_eq!(store.get(width(3.0)), Some(size(30.0)));
        for value in [1.0, 4.0, 5.0] {
            assert_eq!(store.get(width(value)), Some(size(value)), "{value} kept");
        }
    }

    #[test]
    fn a_lookup_returns_the_matching_entry() {
        let mut store = Measurements::default();
        store.insert(width(1.0), size(10.0));
        store.insert(width(2.0), size(20.0));
        assert_eq!(store.get(width(1.0)), Some(size(10.0)));
        assert_eq!(store.get(width(2.0)), Some(size(20.0)));
        assert_eq!(store.get(width(3.0)), None);
    }
}
