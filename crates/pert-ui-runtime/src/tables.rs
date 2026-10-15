//! Crate-private helpers for per-variant tables (defaults, palettes, metrics),
//! which are keyed by variant and checked for completeness at compile time
//! rather than indexed by array position.

/// Declares a constant array of every variant of a fieldless enum, in
/// declaration order, and checks at compile time that it is complete: the list
/// is also the pattern of an exhaustive match, so adding a variant fails to
/// compile until it is listed, and every entry's discriminant must equal its
/// position, which rejects duplicates and reordering. The array's length is
/// therefore the variant count, for per-variant tables and bit masks.
macro_rules! variant_list {
    ($(#[$meta:meta])* $vis:vis const $name:ident: $ty:ident = [$($variant:ident),+ $(,)?];) => {
        $(#[$meta])*
        $vis const $name: [$ty; [$($ty::$variant),+].len()] = [$($ty::$variant),+];
        const _: () = {
            let _: fn($ty) = |value| match value {
                $($ty::$variant)|+ => {}
            };
            let mut index = 0;
            while index < $name.len() {
                assert!(
                    $name[index] as usize == index,
                    concat!("list every ", stringify!($ty), " once, in declaration order"),
                );
                index += 1;
            }
        };
    };
}
pub(crate) use variant_list;
