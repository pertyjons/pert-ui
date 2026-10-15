//! A PNG whose compressed ancillary chunks inflate far beyond any image: the
//! decoder must stay within its documented allocation bound and still decode
//! the pixels. This is its own test binary so the process's peak resident size
//! measures this decode alone.
#![cfg(feature = "png")]
use pert_ui::images;

/// What the embedded profile inflates to without a bound: 1 GiB of zeros.
const BOMB: u64 = 1 << 30;
/// The documented decoder bound, four times `RasterImage::MAX_BYTES`. Only
/// Linux reports the peak resident size this test compares with it.
#[cfg(target_os = "linux")]
const BOUND: u64 = 4 * pert_ui::RasterImage::MAX_BYTES as u64;

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
    let start = png.len();
    png.extend_from_slice(kind);
    png.extend_from_slice(data);
    let crc = crc32(&png[start..]);
    png.extend_from_slice(&crc.to_be_bytes());
}

/// Writes deflate bits least significant first, as the format requires.
#[derive(Default)]
struct Bits {
    bytes: Vec<u8>,
    used: u32,
}
impl Bits {
    fn push(&mut self, value: u32, count: u32) {
        for bit in 0..count {
            if self.used.is_multiple_of(8) {
                self.bytes.push(0);
            }
            if value >> bit & 1 == 1 {
                *self.bytes.last_mut().unwrap() |= 1 << (self.used % 8);
            }
            self.used += 1;
        }
    }
    /// A Huffman code, which deflate stores most significant bit first.
    fn code(&mut self, code: u32, length: u32) {
        let reversed = code.reverse_bits() >> (32 - length);
        self.push(reversed, length);
    }
}

/// A zlib stream of `length` zero bytes in one fixed-Huffman block: a literal
/// zero, then back references of 258 bytes at distance 1.
fn zeros(length: u64) -> Vec<u8> {
    let mut bits = Bits::default();
    bits.push(1, 1); // Final block.
    bits.push(1, 2); // Fixed Huffman codes.
    let literal_zero = || (0x30, 8);
    let (code, width) = literal_zero();
    bits.code(code, width);
    let mut written = 1;
    while length - written >= 258 {
        bits.code(0xC5, 8); // Length 258.
        bits.code(0, 5); // Distance 1.
        written += 258;
    }
    while written < length {
        let (code, width) = literal_zero();
        bits.code(code, width);
        written += 1;
    }
    bits.code(0, 7); // End of block.
    let mut stream = vec![0x78, 0x01];
    stream.extend_from_slice(&bits.bytes);
    // Adler-32 of zeros: the sum of bytes stays 1, the sum of sums is the length.
    let adler = u32::try_from(length % 65521).unwrap() << 16 | 1;
    stream.extend_from_slice(&adler.to_be_bytes());
    stream
}

/// A 4x4 opaque red PNG carrying an iCCP and a zTXt chunk that each inflate to
/// `BOMB` bytes.
fn bomb() -> Vec<u8> {
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::new();
    header.extend_from_slice(&4u32.to_be_bytes());
    header.extend_from_slice(&4u32.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]); // 8-bit RGBA, no interlace.
    chunk(&mut png, b"IHDR", &header);

    let inflating = zeros(BOMB);
    let mut profile = b"bomb\0\0".to_vec();
    profile.extend_from_slice(&inflating);
    chunk(&mut png, b"iCCP", &profile);
    let mut text = b"Raw profile type exif\0\0".to_vec();
    text.extend_from_slice(&inflating);
    chunk(&mut png, b"zTXt", &text);

    let mut raw = Vec::new();
    for _ in 0..4 {
        raw.push(0); // No filter.
        raw.extend_from_slice(&[255, 0, 0, 255].repeat(4));
    }
    let length = u16::try_from(raw.len()).unwrap();
    let mut pixels = vec![0x78, 0x01, 0x01];
    pixels.extend_from_slice(&length.to_le_bytes());
    pixels.extend_from_slice(&(!length).to_le_bytes());
    pixels.extend_from_slice(&raw);
    let (a, b) = raw.iter().fold((1u32, 0u32), |(a, b), &byte| {
        let a = (a + u32::from(byte)) % 65521;
        (a, (b + a) % 65521)
    });
    pixels.extend_from_slice(&(b << 16 | a).to_be_bytes());
    chunk(&mut png, b"IDAT", &pixels);
    chunk(&mut png, b"IEND", &[]);
    png
}

#[cfg(target_os = "linux")]
fn peak_resident_bytes() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let line = status
        .lines()
        .find(|line| line.starts_with("VmHWM:"))
        .expect("Linux reports the peak resident size");
    let kib: u64 = line
        .trim_start_matches("VmHWM:")
        .trim()
        .trim_end_matches("kB")
        .trim()
        .parse()
        .unwrap();
    kib * 1024
}

#[test]
fn inflating_metadata_stays_within_the_decoder_bound() {
    let png = bomb();
    assert!(
        (png.len() as u64) < BOMB / 64,
        "the fixture is small; its chunks inflate"
    );
    let image = images::decode(&png).expect("the pixels decode without the profile");
    assert_eq!((image.width(), image.height()), (4, 4));
    assert_eq!(image.pixels()[..4], [255, 0, 0, 255]);
    // Unbounded, the profile alone would inflate to the full gigabyte.
    #[cfg(target_os = "linux")]
    {
        let peak = peak_resident_bytes();
        assert!(
            peak < BOUND + BOMB / 4,
            "peak resident size {peak} exceeds the decoder bound {BOUND} plus overhead",
        );
    }
}
