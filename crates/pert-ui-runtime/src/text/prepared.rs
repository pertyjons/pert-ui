//! The prepared-text cache: bounded by bytes and entries, evicting the least
//! recently used entry, with hashed lookups by preparation key, by plain text
//! and by raster signature.

use super::{CACHE_BYTES, CachedText, Key, RasterSignature, Resolved, ResolvedSpan, TextColor};
use std::{
    collections::{BTreeMap, HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
};

/// The most prepared texts the cache holds. The byte budget
/// ([`CACHE_BYTES`]) is the primary limit; this one bounds the index sizes
/// when many small texts are prepared. It holds a page of a thousand labels
/// measured at several widths.
pub(super) const TEXT_ENTRIES: usize = 4096;

/// A cached entry with its recency and index hashes.
struct Slot {
    entry: CachedText,
    stamp: u64,
    key_hash: u64,
    signature_hash: u64,
}

/// Prepared texts by identity, ordered by last use.
#[derive(Default)]
pub(super) struct PreparedCache {
    slots: HashMap<u64, Slot>,
    /// Last-use stamp to slot identity; the first entry is evicted first.
    order: BTreeMap<u64, u64>,
    by_key: HashMap<u64, Vec<u64>>,
    by_signature: HashMap<u64, Vec<u64>>,
    next_id: u64,
    next_stamp: u64,
    bytes: usize,
}

/// The bits of a float under `==`: zero's two signs hash alike. Callers reject
/// NaN, which equals nothing and so needs no consistent hash.
fn float_bits(value: f32) -> u32 {
    if value == 0.0 { 0 } else { value.to_bits() }
}
fn double_bits(value: f64) -> u64 {
    if value == 0.0 { 0 } else { value.to_bits() }
}

/// Hashes the fields of a preparation key that plain-text lookups can also
/// compute, so equal keys, and a plain lookup of the same text, share a hash.
fn key_hash(text: &str, font: usize, size: f32, width: Option<f32>, scale: f64) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    font.hash(&mut hasher);
    float_bits(size).hash(&mut hasher);
    width.map(float_bits).hash(&mut hasher);
    double_bits(scale).hash(&mut hasher);
    hasher.finish()
}

/// Hashes what a coverage reuse compares: the text, its raster scale and
/// its positioned glyphs.
fn signature_hash(resolved: &Resolved, scale: f32, signature: &RasterSignature) -> u64 {
    let mut hasher = DefaultHasher::new();
    resolved.text.hash(&mut hasher);
    resolved.default_font.hash(&mut hasher);
    float_bits(resolved.default_size).hash(&mut hasher);
    float_bits(scale).hash(&mut hasher);
    signature.hash(&mut hasher);
    hasher.finish()
}

fn entry_key_hash(key: &Key) -> u64 {
    let r = &key.resolved;
    key_hash(
        &r.text,
        r.default_font,
        r.default_size,
        key.width,
        key.scale,
    )
}

impl PreparedCache {
    /// The total accounted bytes of the cached entries.
    #[cfg(test)]
    pub(super) fn bytes(&self) -> usize {
        self.bytes
    }
    /// How many entries the cache holds.
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.slots.len()
    }
    /// The entries, least recently used first.
    #[cfg(test)]
    pub(super) fn iter(&self) -> impl Iterator<Item = &CachedText> {
        self.order.values().map(|id| &self.slots[id].entry)
    }
    /// The most recently used or inserted entry.
    #[cfg(test)]
    pub(super) fn newest(&self) -> Option<&CachedText> {
        self.order
            .last_key_value()
            .map(|(_, id)| &self.slots[id].entry)
    }
    /// Empties the cache.
    #[cfg(test)]
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    /// Marks `id` most recently used and returns its entry.
    fn touch(&mut self, id: u64) -> Option<&CachedText> {
        let stamp = self.next_stamp;
        let slot = self.slots.get_mut(&id)?;
        self.order.remove(&slot.stamp);
        slot.stamp = stamp;
        self.order.insert(stamp, id);
        self.next_stamp += 1;
        Some(&slot.entry)
    }
    fn find(
        &mut self,
        index: Index,
        hash: u64,
        matches: impl Fn(&CachedText) -> bool,
    ) -> Option<&CachedText> {
        let candidates = match index {
            Index::Key => self.by_key.get(&hash),
            Index::Signature => self.by_signature.get(&hash),
        }?;
        let id = *candidates
            .iter()
            .find(|id| matches(&self.slots[*id].entry))?;
        self.touch(id)
    }

    /// The entry prepared under exactly `key`.
    pub(super) fn find_key(&mut self, key: &Key) -> Option<&CachedText> {
        self.find(Index::Key, entry_key_hash(key), |entry| entry.key == *key)
    }

    /// The entry plain `text` in `font` prepares to: one span covering the
    /// text (none for empty text) with no gap, icon or color of its own.
    pub(super) fn find_plain(
        &mut self,
        text: &str,
        width: Option<f32>,
        scale: f64,
        font: usize,
        font_size: f32,
    ) -> Option<&CachedText> {
        let hash = key_hash(text, font, font_size, width, scale);
        self.find(Index::Key, hash, |entry| {
            let key = &entry.key;
            let r = &key.resolved;
            key.width == width
                && key.scale == scale
                && r.text == text
                && r.default_font == font
                && r.default_size == font_size
                && r.gaps.is_empty()
                && r.colors == [TextColor::Inherit]
                && r.icon_brushes == [false]
                && if text.is_empty() {
                    r.spans.is_empty()
                } else {
                    r.spans.as_slice()
                        == [ResolvedSpan {
                            range: 0..text.len(),
                            font,
                            size: font_size,
                            brush: 0,
                            icon: false,
                        }]
                }
        })
    }

    /// An entry of the same resolved text and raster scale whose positioned
    /// glyphs are `signature`, so its coverage can be reused.
    pub(super) fn find_signature(
        &mut self,
        resolved: &Resolved,
        scale: f32,
        signature: &RasterSignature,
    ) -> Option<&CachedText> {
        let hash = signature_hash(resolved, scale, signature);
        self.find(Index::Signature, hash, |entry| {
            entry.key.resolved == *resolved
                && entry.key.scale as f32 == scale
                && entry.signature.as_ref() == signature
        })
    }

    /// Caches `entry`, evicting the least recently used entries until it fits
    /// both limits. An entry larger than the whole budget is not cached.
    pub(super) fn insert(&mut self, entry: CachedText) {
        if entry.bytes > CACHE_BYTES {
            return;
        }
        while self.slots.len() >= TEXT_ENTRIES || self.bytes + entry.bytes > CACHE_BYTES {
            if !self.evict_oldest() {
                break;
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        let stamp = self.next_stamp;
        self.next_stamp += 1;
        let key_hash = entry_key_hash(&entry.key);
        let signature_hash = signature_hash(
            &entry.key.resolved,
            entry.key.scale as f32,
            &entry.signature,
        );
        self.by_key.entry(key_hash).or_default().push(id);
        self.by_signature
            .entry(signature_hash)
            .or_default()
            .push(id);
        self.order.insert(stamp, id);
        self.bytes += entry.bytes;
        self.slots.insert(
            id,
            Slot {
                entry,
                stamp,
                key_hash,
                signature_hash,
            },
        );
    }

    /// Evicts the least recently used entry; false when the cache is empty.
    fn evict_oldest(&mut self) -> bool {
        let Some((_, id)) = self.order.pop_first() else {
            return false;
        };
        let Some(slot) = self.slots.remove(&id) else {
            return false;
        };
        self.bytes -= slot.entry.bytes;
        for (index, hash) in [
            (&mut self.by_key, slot.key_hash),
            (&mut self.by_signature, slot.signature_hash),
        ] {
            if let Some(ids) = index.get_mut(&hash) {
                ids.retain(|candidate| *candidate != id);
                if ids.is_empty() {
                    index.remove(&hash);
                }
            }
        }
        true
    }
}

/// Which hash index a lookup uses.
#[derive(Clone, Copy)]
enum Index {
    Key,
    Signature,
}
