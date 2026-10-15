//! The atomics the live-data protocols are built from.
//!
//! Ordinary builds use the standard library. A build with `--cfg loom` swaps in
//! loom's instrumented equivalents so the ring and the wake flag can be
//! model-checked across every interleaving the memory model allows; see the
//! `loom` integration test. Nothing else in the crate depends on the choice.

#[cfg(loom)]
pub(crate) use loom::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
#[cfg(not(loom))]
pub(crate) use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
