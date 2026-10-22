//! Backend-independent retained rectangle drawing for custom components.

/// Invalid drawing input or lifecycle use, the error of
/// [`Component::paint`](crate::Component::paint).
///
/// Drawing sits above [`RuntimeError`](crate::RuntimeError): every runtime
/// failure during paint, including invalid geometry and text preparation, is
/// [`Self::Runtime`], and `?` converts each leaf error such as
/// [`GeometryError`](crate::GeometryError) into that form. The other variants
/// are failures only drawing has.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{DrawError, GeometryError, Rect, RuntimeError};
/// fn bar() -> Result<Rect, DrawError> {
///     Ok(Rect::new(0.0, 0.0, -1.0, 4.0)?)
/// }
/// assert_eq!(
///     bar(),
///     Err(DrawError::Runtime(RuntimeError::Geometry(GeometryError::Negative)))
/// );
/// ```
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum DrawError {
    /// Linear RGBA channels must be finite and within zero through one.
    #[error("color channels must be finite and within 0..=1")]
    InvalidColor,
    /// A coverage mask or raster image has a zero dimension, a byte count that
    /// does not match its dimensions, or more pixel data than its type admits.
    #[error("invalid image: zero dimension, inconsistent byte length or over its size limit")]
    InvalidImage,
    /// A vector path or stroke style was rejected.
    #[error(transparent)]
    Path(#[from] crate::PathError),
    /// Covering a vector at its raster scale would take more than
    /// [`VectorRasterizer::MAX_COVERAGE_WORK`](crate::VectorRasterizer::MAX_COVERAGE_WORK)
    /// sub-scanline steps, as a long path crossing the mask's height many
    /// times does. Draw fewer points, a shorter vertical extent, or clip it.
    #[error("vector coverage exceeds its work budget")]
    TooComplex,
    /// Updates must be flushed before recording the scene.
    #[error("flush pending component updates before painting")]
    PendingUpdates,
    /// The UI runtime rejected the operation, geometry was invalid, or text could
    /// not be prepared.
    #[error(transparent)]
    Runtime(#[from] crate::RuntimeError),
}
crate::error::leaf_conversions!(DrawError => DrawError::Runtime);

/// Finite rectangle in logical pixels, with nonnegative width and height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect([f32; 4]);
impl Rect {
    /// Creates `[x, y, width, height]`; negative coordinates are permitted.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`](crate::GeometryError::NonFinite) for
    /// non-finite data and [`GeometryError::Negative`](crate::GeometryError::Negative)
    /// for a negative size.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Result<Self, crate::GeometryError> {
        let values = [x, y, width, height];
        if !values.iter().all(|v| v.is_finite()) {
            return Err(crate::GeometryError::NonFinite);
        }
        if width < 0.0 || height < 0.0 {
            return Err(crate::GeometryError::Negative);
        }
        Ok(Self(values))
    }
    /// The rectangle a validated size occupies at its own origin.
    ///
    /// This is the allocation a component paints into, so it is the usual argument
    /// to [`Painter::fill_rect`] and to [`ArrangeContext::place`](crate::ArrangeContext::place)
    /// for a child that fills its parent. A [`Size`](crate::Size) is already finite
    /// and nonnegative, so no geometry can be rejected here.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::{Rect, Size};
    /// let size = Size::new(80.0, 24.0)?;
    /// assert_eq!(Rect::from_size(size).values(), [0.0, 0.0, 80.0, 24.0]);
    /// # Ok::<(), pert_ui::GeometryError>(())
    /// ```
    #[must_use]
    pub const fn from_size(size: crate::Size) -> Self {
        Self([0.0, 0.0, size.width(), size.height()])
    }
    /// The rectangle a validated size occupies at a validated origin.
    ///
    /// A [`Point`](crate::Point) is finite and a [`Size`](crate::Size) is finite and
    /// nonnegative, so the result is always valid geometry; negative coordinates are
    /// permitted, as in [`Rect::new`].
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::{Point, Rect, Size};
    /// let rect = Rect::from_origin_size(Point::new(-4.0, 8.0)?, Size::new(80.0, 24.0)?);
    /// assert_eq!(rect.values(), [-4.0, 8.0, 80.0, 24.0]);
    /// # Ok::<(), pert_ui::GeometryError>(())
    /// ```
    #[must_use]
    pub const fn from_origin_size(origin: crate::Point, size: crate::Size) -> Self {
        Self([origin.x(), origin.y(), size.width(), size.height()])
    }
    /// Whether a point lies inside this rectangle.
    ///
    /// Containment is half-open on both axes: the left and top edges belong to the
    /// rectangle and the right and bottom edges do not, so adjacent rectangles
    /// tile a surface without a shared pixel column or row and without a gap. A
    /// zero-width or zero-height rectangle therefore contains nothing. The
    /// comparison is performed in double precision, so a far-off origin does not
    /// lose the extent to rounding.
    ///
    /// Both values must be in the same space. Pointer events carry logical window
    /// coordinates, while a component's retained geometry is usually local to its
    /// own allocation; convert the point with
    /// [`EventContext::to_local`](crate::EventContext::to_local) first.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::{Point, Rect};
    /// let rect = Rect::new(10.0, 10.0, 20.0, 20.0)?;
    /// assert!(rect.contains(Point::new(10.0, 10.0)?));
    /// assert!(!rect.contains(Point::new(30.0, 20.0)?));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn contains(self, point: crate::Point) -> bool {
        let [x, y, width, height] = self.0.map(f64::from);
        let px = f64::from(point.x());
        let py = f64::from(point.y());
        px >= x && py >= y && px < x + width && py < y + height
    }
    /// Whether the rectangle covers no pixels.
    ///
    /// A zero-width or zero-height rectangle produces nothing when filled and
    /// hides everything it clips, so composition and backends can drop such work
    /// instead of encoding it. [`Rect::intersection`] returns zero area for
    /// disjoint rectangles, which makes this the test for "the clip removed it".
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::Rect;
    /// let visible = Rect::new(0.0, 0.0, 10.0, 10.0)?;
    /// assert!(!visible.is_empty());
    /// assert!(visible.intersection(Rect::new(20.0, 0.0, 10.0, 10.0)?).is_empty());
    /// # Ok::<(), pert_ui::DrawError>(())
    /// ```
    #[must_use]
    pub const fn is_empty(self) -> bool {
        let [_, _, width, height] = self.0;
        width <= 0.0 || height <= 0.0
    }
    /// Intersects two logical rectangles; disjoint rectangles return zero area.
    #[must_use]
    pub fn intersection(self, other: Self) -> Self {
        let [ax, ay, aw, ah] = self.0.map(f64::from);
        let [bx, by, bw, bh] = other.0.map(f64::from);
        let x = ax.max(bx);
        let y = ay.max(by);
        Self([
            x as f32,
            y as f32,
            ((ax + aw).min(bx + bw) - x).max(0.0) as f32,
            ((ay + ah).min(by + bh) - y).max(0.0) as f32,
        ])
    }
    /// Horizontal position of the left edge in logical pixels; may be negative.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::Rect;
    /// let rect = Rect::new(-4.0, 8.0, 80.0, 24.0)?;
    /// assert_eq!((rect.x(), rect.y()), (-4.0, 8.0));
    /// assert_eq!((rect.width(), rect.height()), (80.0, 24.0));
    /// # Ok::<(), pert_ui::DrawError>(())
    /// ```
    #[must_use]
    pub const fn x(self) -> f32 {
        self.0[0]
    }
    /// Vertical position of the top edge in logical pixels; may be negative.
    #[must_use]
    pub const fn y(self) -> f32 {
        self.0[1]
    }
    /// Finite, nonnegative width in logical pixels.
    #[must_use]
    pub const fn width(self) -> f32 {
        self.0[2]
    }
    /// Finite, nonnegative height in logical pixels.
    #[must_use]
    pub const fn height(self) -> f32 {
        self.0[3]
    }
    /// Returns `[x, y, width, height]` in logical pixels, for destructuring all
    /// four at once or uploading them to a backend.
    ///
    /// Read a single component through [`Rect::x`], [`Rect::y`], [`Rect::width`]
    /// or [`Rect::height`] instead of indexing this array, so the call site names
    /// what it reads.
    #[must_use]
    pub const fn values(self) -> [f32; 4] {
        self.0
    }
}

/// Straight-alpha linear RGBA color; the renderer blends into an sRGB surface.
///
/// Named constants are fixed dark-palette values, unaffected by theme changes.
/// Use [`Painter::color`] with a semantic role for adaptive component styling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color([f32; 4]);
impl Color {
    // Internal literal palette defaults only; public callers use validated rgba.
    pub(crate) const fn linear_rgb(red: f32, green: f32, blue: f32) -> Self {
        Self::linear_rgba(red, green, blue, 1.0)
    }
    pub(crate) const fn linear_rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        assert!(
            red >= 0.0
                && red <= 1.0
                && green >= 0.0
                && green <= 1.0
                && blue >= 0.0
                && blue <= 1.0
                && alpha >= 0.0
                && alpha <= 1.0
        );
        Self([red, green, blue, alpha])
    }

    /// Default high-contrast light foreground on dark surfaces.
    pub const TEXT: Self = Self([0.86, 0.89, 0.94, 1.0]);
    /// Muted neutral literal and the dark palette's default outline.
    /// Supporting text uses the independently configured [`crate::ColorRole::Muted`].
    pub const MUTED: Self = Self([0.32, 0.36, 0.43, 1.0]);
    /// Default neutral control/panel surface.
    pub const SURFACE: Self = Self([0.045, 0.055, 0.075, 1.0]);
    /// Default hovered control surface.
    pub const SURFACE_HOVER: Self = Self([0.075, 0.095, 0.13, 1.0]);
    /// Filled action-control surface, distinct from group panels and tab navigation.
    pub const BUTTON: Self = Self([0.09, 0.125, 0.175, 1.0]);
    /// Hovered action-control surface.
    pub const BUTTON_HOVER: Self = Self([0.11, 0.16, 0.225, 1.0]);
    /// Filled inactive navigation tab surface.
    pub const TAB: Self = Self([0.075, 0.085, 0.105, 1.0]);
    /// Filled selected navigation tab surface.
    pub const TAB_SELECTED: Self = Self([0.08, 0.16, 0.25, 1.0]);
    /// Default opaque neutral window background.
    pub const BACKGROUND: Self = Self([0.018, 0.022, 0.032, 1.0]);
    /// Default opaque blue accent.
    pub const ACCENT: Self = Self([0.08, 0.32, 0.85, 1.0]);
    /// Opaque white. As the tint of [`Painter::draw_image_tinted`] it leaves the
    /// image's pixels unchanged, which is what [`Painter::draw_image`] uses.
    pub const WHITE: Self = Self([1.0, 1.0, 1.0, 1.0]);
    /// Creates a linear color. RGB input is not interpreted as sRGB bytes.
    ///
    /// # Errors
    /// Returns [`DrawError::InvalidColor`] for a non-finite channel or one
    /// outside `0..=1`.
    pub fn rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Result<Self, DrawError> {
        let values = [red, green, blue, alpha];
        if values
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err(DrawError::InvalidColor);
        }
        Ok(Self(values))
    }
    /// Linear red channel within `0..=1`.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::Color;
    /// let color = Color::rgba(0.25, 0.5, 0.75, 1.0)?;
    /// assert_eq!((color.red(), color.green(), color.blue()), (0.25, 0.5, 0.75));
    /// assert_eq!(color.alpha(), 1.0);
    /// # Ok::<(), pert_ui::DrawError>(())
    /// ```
    #[must_use]
    pub const fn red(self) -> f32 {
        self.0[0]
    }
    /// Linear green channel within `0..=1`.
    #[must_use]
    pub const fn green(self) -> f32 {
        self.0[1]
    }
    /// Linear blue channel within `0..=1`.
    #[must_use]
    pub const fn blue(self) -> f32 {
        self.0[2]
    }
    /// Straight (not premultiplied) alpha within `0..=1`; zero is fully transparent.
    #[must_use]
    pub const fn alpha(self) -> f32 {
        self.0[3]
    }
    /// Returns straight-alpha linear `[red, green, blue, alpha]` channels, for
    /// destructuring all four at once or uploading them to a backend.
    ///
    /// Read a single channel through [`Color::red`], [`Color::green`],
    /// [`Color::blue`] or [`Color::alpha`] instead of indexing this array.
    #[must_use]
    pub const fn channels(self) -> [f32; 4] {
        self.0
    }

    /// Creates a color from gamma-encoded sRGB channels within `0..=1`, such as
    /// a design tool's or a color picker's values divided by their maximum.
    ///
    /// Red, green and blue are decoded with the exact IEC 61966-2-1 transfer
    /// function, the encoding the renderer's sRGB surface applies on output, so
    /// the color appears with the channel values it was given. Alpha is a
    /// straight coverage fraction and is stored as given; it is never
    /// gamma-encoded.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::Color;
    /// let gray = Color::from_srgb(0.5, 0.5, 0.5, 1.0)?;
    /// // Mid-gray in sRGB is about 21% linear light.
    /// assert!((gray.red() - 0.214).abs() < 0.001);
    /// assert_eq!(gray.to_srgb()[3], 1.0);
    /// # Ok::<(), pert_ui::DrawError>(())
    /// ```
    ///
    /// # Errors
    /// [`DrawError::InvalidColor`] for a non-finite channel or one outside
    /// `0..=1`.
    pub fn from_srgb(red: f32, green: f32, blue: f32, alpha: f32) -> Result<Self, DrawError> {
        let values = [red, green, blue, alpha];
        if values
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err(DrawError::InvalidColor);
        }
        Ok(Self([
            srgb_decode(f64::from(red)),
            srgb_decode(f64::from(green)),
            srgb_decode(f64::from(blue)),
            alpha,
        ]))
    }

    /// Creates a color from 8-bit sRGB channels, as in `#rrggbbaa` notation or
    /// 0–255 values.
    ///
    /// Each color byte is divided by 255 and decoded as by
    /// [`Color::from_srgb`]; the alpha byte divided by 255 is the straight
    /// alpha. Every byte quadruple is valid, and
    /// [`Color::to_srgb8`] returns the same bytes.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::Color;
    /// let teal = Color::from_srgb8(0x20, 0x90, 0xa0, 0xff);
    /// assert_eq!(teal.to_srgb8(), [0x20, 0x90, 0xa0, 0xff]);
    /// assert_eq!(teal.alpha(), 1.0);
    /// ```
    #[must_use]
    pub fn from_srgb8(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        let channel = |byte: u8| srgb_decode(f64::from(byte) / 255.0);
        Self([
            channel(red),
            channel(green),
            channel(blue),
            // The quotient of two exact integers is within 0..=1 and f32
            // rounding cannot leave that range.
            (f64::from(alpha) / 255.0) as f32,
        ])
    }

    /// Returns gamma-encoded sRGB `[red, green, blue, alpha]` within `0..=1`.
    ///
    /// The color channels use the exact IEC 61966-2-1 encoding, the inverse of
    /// [`Color::from_srgb`]; alpha is returned unchanged. Converting back with
    /// [`Color::from_srgb`] reproduces the color to within `f32` rounding.
    #[must_use]
    pub fn to_srgb(self) -> [f32; 4] {
        let [red, green, blue, alpha] = self.0;
        [
            srgb_encode(f64::from(red)) as f32,
            srgb_encode(f64::from(green)) as f32,
            srgb_encode(f64::from(blue)) as f32,
            alpha,
        ]
    }

    /// Returns 8-bit sRGB `[red, green, blue, alpha]`.
    ///
    /// Each channel of [`Color::to_srgb`] is multiplied by 255 and rounded to
    /// the nearest byte, ties away from zero. Distinct linear colors can share
    /// bytes, so this is lossy except for colors made by
    /// [`Color::from_srgb8`] or [`Color::from_hex`], which round-trip exactly.
    #[must_use]
    pub fn to_srgb8(self) -> [u8; 4] {
        let [red, green, blue, alpha] = self.0;
        let byte = |value: f64| {
            // Valid channels keep `value` within 0..=1, so the rounded product
            // is an integer within 0..=255; the clamp only guards rounding.
            (value * 255.0).round().clamp(0.0, 255.0) as u8
        };
        [
            byte(srgb_encode(f64::from(red))),
            byte(srgb_encode(f64::from(green))),
            byte(srgb_encode(f64::from(blue))),
            byte(f64::from(alpha)),
        ]
    }

    /// Parses CSS-style hexadecimal sRGB notation.
    ///
    /// Accepted forms are `rgb`, `rgba`, `rrggbb` and `rrggbbaa`, each with or
    /// without one leading `#`. Digits are case-insensitive. A short form
    /// repeats each digit (`#f80` is `#ff8800`), and a form without alpha is
    /// opaque. The bytes are decoded as by [`Color::from_srgb8`]. Surrounding
    /// whitespace, signs and other prefixes such as `0x` are rejected rather
    /// than skipped. [`Color`] also implements [`FromStr`](std::str::FromStr)
    /// with this grammar.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::{Color, HexColorError};
    /// let orange = Color::from_hex("#FF8800")?;
    /// assert_eq!(orange, Color::from_hex("f80")?);
    /// assert_eq!(orange.to_srgb8(), [0xff, 0x88, 0x00, 0xff]);
    /// assert_eq!(Color::from_hex("#12345"), Err(HexColorError::Length(5)));
    /// assert_eq!(
    ///     Color::from_hex("#12g456"),
    ///     Err(HexColorError::Digit { index: 3, character: 'g' })
    /// );
    /// # Ok::<(), HexColorError>(())
    /// ```
    ///
    /// # Errors
    /// [`HexColorError::Length`] when the digits after an optional `#` are not
    /// 3, 4, 6 or 8 characters long, and [`HexColorError::Digit`] for the first
    /// character that is not a hexadecimal digit.
    pub fn from_hex(text: &str) -> Result<Self, HexColorError> {
        let digits = text.strip_prefix('#').unwrap_or(text);
        let offset = text.len() - digits.len();
        let mut values = Vec::with_capacity(8);
        for (index, character) in digits.char_indices() {
            let Some(value) = character.to_digit(16) else {
                return Err(HexColorError::Digit {
                    index: offset + index,
                    character,
                });
            };
            // Text longer than any accepted form is still checked for digits
            // and then reported by its length, without retaining its values.
            if values.len() < 8 {
                // `to_digit(16)` returns a value below 16.
                values.push(value as u8);
            }
        }
        let length = digits.chars().count();
        let bytes: [u8; 4] = match (length, values.as_slice()) {
            (3, &[r, g, b]) => [r * 17, g * 17, b * 17, 255],
            (4, &[r, g, b, a]) => [r * 17, g * 17, b * 17, a * 17],
            (6, &[r1, r0, g1, g0, b1, b0]) => [r1 << 4 | r0, g1 << 4 | g0, b1 << 4 | b0, 255],
            (8, &[r1, r0, g1, g0, b1, b0, a1, a0]) => {
                [r1 << 4 | r0, g1 << 4 | g0, b1 << 4 | b0, a1 << 4 | a0]
            }
            _ => return Err(HexColorError::Length(length)),
        };
        let [red, green, blue, alpha] = bytes;
        Ok(Self::from_srgb8(red, green, blue, alpha))
    }

    /// Formats the color as lowercase hexadecimal sRGB with a leading `#`:
    /// `#rrggbb` when the alpha byte is 255 and `#rrggbbaa` otherwise.
    ///
    /// The digits are [`Color::to_srgb8`]'s bytes, so the text is lossy in the
    /// same way, and [`Color::from_hex`] reads it back to the same bytes.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::Color;
    /// assert_eq!(Color::from_srgb8(255, 136, 0, 255).to_hex(), "#ff8800");
    /// assert_eq!(Color::from_srgb8(255, 136, 0, 128).to_hex(), "#ff880080");
    /// ```
    #[must_use]
    pub fn to_hex(self) -> String {
        let [red, green, blue, alpha] = self.to_srgb8();
        if alpha == u8::MAX {
            format!("#{red:02x}{green:02x}{blue:02x}")
        } else {
            format!("#{red:02x}{green:02x}{blue:02x}{alpha:02x}")
        }
    }

    /// Returns this color with its straight alpha replaced and its linear RGB
    /// unchanged.
    ///
    /// Use [`Color::scale_alpha`] instead to fade a color while keeping the
    /// translucency it already has, such as a theme role's.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::Color;
    /// let veil = Color::WHITE.with_alpha(0.25)?;
    /// assert_eq!(veil.channels(), [1.0, 1.0, 1.0, 0.25]);
    /// # Ok::<(), pert_ui::DrawError>(())
    /// ```
    ///
    /// # Errors
    /// [`DrawError::InvalidColor`] for a non-finite `alpha` or one outside
    /// `0..=1`.
    pub fn with_alpha(self, alpha: f32) -> Result<Self, DrawError> {
        Self::rgba(self.red(), self.green(), self.blue(), alpha)
    }

    /// Returns this color with its straight alpha multiplied by `factor`, a
    /// value within `0..=1`; linear RGB is unchanged.
    ///
    /// Unlike [`Color::with_alpha`], a color that is already translucent stays
    /// at most as opaque as it was: an animation fading a role to half opacity
    /// keeps the role's own alpha as the upper limit. A factor above one is
    /// rejected rather than clamped, because it would raise an alpha the
    /// application chose.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::Color;
    /// let thumb = Color::rgba(0.6, 0.7, 0.8, 0.5)?;
    /// assert_eq!(thumb.scale_alpha(0.5)?.alpha(), 0.25);
    /// assert!(thumb.scale_alpha(2.0).is_err());
    /// # Ok::<(), pert_ui::DrawError>(())
    /// ```
    ///
    /// # Errors
    /// [`DrawError::InvalidColor`] for a non-finite `factor` or one outside
    /// `0..=1`.
    pub fn scale_alpha(self, factor: f32) -> Result<Self, DrawError> {
        if !factor.is_finite() || !(0.0..=1.0).contains(&factor) {
            return Err(DrawError::InvalidColor);
        }
        // Both values are within 0..=1, so the product is too.
        Ok(Self([
            self.red(),
            self.green(),
            self.blue(),
            self.alpha() * factor,
        ]))
    }

    /// Interpolates from this color (`t = 0`) to `other` (`t = 1`) in
    /// premultiplied linear light, with `t` within `0..=1`.
    ///
    /// Premultiplied interpolation matches how the renderer blends and keeps
    /// the RGB of a nearly transparent endpoint from bleeding into the result:
    /// mixing toward [`Color::rgba`]`(0, 0, 0, 0)` fades a color without
    /// darkening it. A result whose alpha is zero has no meaningful RGB and is
    /// returned as transparent black, including when a tiny alpha rounds to
    /// zero. At `t = 0` and `t = 1` the endpoint is returned unchanged, bit for
    /// bit, unless it is fully transparent.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::Color;
    /// let red = Color::rgba(1.0, 0.0, 0.0, 1.0)?;
    /// let clear = Color::rgba(0.0, 0.0, 0.0, 0.0)?;
    /// // Fading keeps the hue: no black is mixed into the red.
    /// assert_eq!(red.mix(clear, 0.5)?.channels(), [1.0, 0.0, 0.0, 0.5]);
    /// assert_eq!(red.mix(clear, 0.0)?, red);
    /// # Ok::<(), pert_ui::DrawError>(())
    /// ```
    ///
    /// # Errors
    /// [`DrawError::InvalidColor`] for a non-finite `t` or one outside `0..=1`.
    pub fn mix(self, other: Self, t: f32) -> Result<Self, DrawError> {
        if !t.is_finite() || !(0.0..=1.0).contains(&t) {
            return Err(DrawError::InvalidColor);
        }
        const TRANSPARENT: Color = Color([0.0; 4]);
        let endpoint = match t {
            0.0 => Some(self),
            1.0 => Some(other),
            _ => None,
        };
        if let Some(color) = endpoint {
            return Ok(if color.alpha() == 0.0 {
                TRANSPARENT
            } else {
                color
            });
        }
        let t = f64::from(t);
        let near = f64::from(self.alpha()) * (1.0 - t);
        let far = f64::from(other.alpha()) * t;
        let alpha = near + far;
        // A weighted mean of values within 0..=1; the clamp only guards rounding.
        let stored = alpha.clamp(0.0, 1.0) as f32;
        // Checked after narrowing: a positive f64 alpha can round to zero.
        if stored == 0.0 {
            return Ok(TRANSPARENT);
        }
        let [left, right] = [self.0, other.0];
        let channel = |index: usize| {
            let value = (f64::from(left[index]) * near + f64::from(right[index]) * far) / alpha;
            value.clamp(0.0, 1.0) as f32
        };
        Ok(Self([channel(0), channel(1), channel(2), stored]))
    }

    /// Composites this color over an opaque `background` with source-over
    /// blending in linear light, as the renderer draws it.
    ///
    /// The result is opaque, so it can be measured with
    /// [`OpaqueColor::contrast_ratio`]: a translucent fill or foreground is
    /// measured as it appears over the color actually behind it. A fully
    /// transparent color returns `background` and an opaque one returns
    /// itself, both unchanged.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::{Color, OpaqueColor};
    /// let black = OpaqueColor::try_from(Color::rgba(0.0, 0.0, 0.0, 1.0)?)?;
    /// let veil = Color::WHITE.with_alpha(0.5)?;
    /// assert_eq!(Color::from(veil.over(black)).channels(), [0.5, 0.5, 0.5, 1.0]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn over(self, background: OpaqueColor) -> OpaqueColor {
        match self.alpha() {
            0.0 => return background,
            1.0 => return OpaqueColor(self),
            _ => {}
        }
        let alpha = f64::from(self.alpha());
        let back = background.0.0;
        let channel = |index: usize| {
            let value = f64::from(self.0[index]) * alpha + f64::from(back[index]) * (1.0 - alpha);
            // A weighted mean of values within 0..=1; the clamp only guards rounding.
            value.clamp(0.0, 1.0) as f32
        };
        OpaqueColor(Self([channel(0), channel(1), channel(2), 1.0]))
    }
}

/// A [`Color`] whose alpha is exactly one, the input WCAG 2.x contrast
/// measurement requires.
///
/// A contrast ratio is undefined for a translucent color until it is
/// composited over what lies behind it, so the measurements take this type
/// rather than [`Color`]. Obtain one with `OpaqueColor::try_from(color)`, which
/// refuses a translucent color, or by compositing with [`Color::over`].
/// `Color::from(opaque)` returns the underlying color.
///
/// The measurements report; they never adjust a color. The library does not
/// refuse or correct a palette that falls below a contrast minimum.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// # use pert_ui::{Color, OpaqueColor, Palette, ColorRole};
/// let palette = Palette::dark();
/// let text = OpaqueColor::try_from(palette.color(ColorRole::OnSurface))?;
/// let surface = OpaqueColor::try_from(palette.color(ColorRole::Surface))?;
/// // WCAG 2.x asks 4.5:1 for ordinary text.
/// assert!(text.contrast_ratio(surface) >= 4.5);
///
/// // A translucent color must be composited first.
/// let veil = Color::WHITE.with_alpha(0.3)?;
/// assert!(OpaqueColor::try_from(veil).is_err());
/// assert!(veil.over(surface).contrast_ratio(surface) > 1.0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Contrast is not defined on [`Color`] itself:
///
/// ```compile_fail
/// # use pert_ui_runtime as pert_ui;
/// # use pert_ui::Color;
/// let _ = Color::WHITE.contrast_ratio(Color::TEXT);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpaqueColor(Color);

impl OpaqueColor {
    /// WCAG 2.x relative luminance, within `0..=1`: black is zero and white
    /// is one.
    ///
    /// Computed directly from the stored linear channels with the WCAG
    /// weights. Calculators that decode 8-bit sRGB with WCAG 2.x's rounded
    /// 0.03928 threshold can differ in the last digits for very dark channels.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::{Color, OpaqueColor};
    /// let gray = OpaqueColor::try_from(Color::rgba(0.5, 0.5, 0.5, 1.0)?)?;
    /// assert!((gray.relative_luminance() - 0.5).abs() < 1e-6);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn relative_luminance(self) -> f32 {
        let [red, green, blue, _] = self.0.0;
        let luminance =
            0.2126 * f64::from(red) + 0.7152 * f64::from(green) + 0.0722 * f64::from(blue);
        // The weights sum to one; the clamp only guards rounding.
        luminance.clamp(0.0, 1.0) as f32
    }

    /// WCAG 2.x contrast ratio between two opaque colors, within `1..=21`.
    ///
    /// The ratio is symmetric. WCAG 2.x asks at least 4.5 for ordinary text,
    /// 3 for large text, and 3 for user-interface boundaries, graphics and
    /// focus indicators against their adjacent colors.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::{Color, OpaqueColor};
    /// let white = OpaqueColor::try_from(Color::WHITE)?;
    /// let black = OpaqueColor::try_from(Color::from_hex("#000")?)?;
    /// assert_eq!(white.contrast_ratio(black), 21.0);
    /// assert_eq!(black.contrast_ratio(white), 21.0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn contrast_ratio(self, other: Self) -> f32 {
        let (a, b) = (
            f64::from(self.relative_luminance()),
            f64::from(other.relative_luminance()),
        );
        ((a.max(b) + 0.05) / (a.min(b) + 0.05)) as f32
    }
}

impl TryFrom<Color> for OpaqueColor {
    type Error = TranslucentColorError;
    /// Accepts a color whose alpha is exactly one.
    ///
    /// # Errors
    /// [`TranslucentColorError`] for any lower alpha; composite such a color
    /// over its background with [`Color::over`] instead.
    fn try_from(color: Color) -> Result<Self, Self::Error> {
        if color.alpha() == 1.0 {
            Ok(Self(color))
        } else {
            Err(TranslucentColorError)
        }
    }
}

impl From<OpaqueColor> for Color {
    fn from(color: OpaqueColor) -> Self {
        color.0
    }
}

/// A translucent [`Color`] was offered where an [`OpaqueColor`] is required.
/// Composite it over the color behind it with [`Color::over`] first.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("contrast needs an opaque color; composite a translucent one over its background first")]
pub struct TranslucentColorError;

impl std::str::FromStr for Color {
    type Err = HexColorError;
    /// Parses hexadecimal sRGB notation as [`Color::from_hex`] does.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::from_hex(text)
    }
}

/// Rejected hexadecimal color text, the error of [`Color::from_hex`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum HexColorError {
    /// The digits after an optional leading `#` are not 3, 4, 6 or 8
    /// characters long; the value is the number of characters found.
    #[error("hex color must have 3, 4, 6 or 8 digits, found {0}")]
    Length(usize),
    /// A character that is not a hexadecimal digit.
    #[error("invalid hex digit {character:?} at byte {index}")]
    Digit {
        /// Byte offset of the character in the parsed text, counting a
        /// leading `#`.
        index: usize,
        /// The rejected character.
        character: char,
    },
}

// IEC 61966-2-1 sRGB decoding of one color channel within 0..=1, computed in
// f64 so 8-bit values round-trip exactly through the f32 result.
fn srgb_decode(encoded: f64) -> f32 {
    let linear = if encoded <= 0.040_45 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    };
    // The function maps 0..=1 onto 0..=1; the clamp only absorbs rounding.
    linear.clamp(0.0, 1.0) as f32
}

// IEC 61966-2-1 sRGB encoding of one linear channel within 0..=1.
fn srgb_encode(linear: f64) -> f64 {
    let encoded = if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    encoded.clamp(0.0, 1.0)
}

/// Logical window extent and physical pixels per logical pixel.
///
/// Default is an unavailable zero-sized viewport at scale one for headless use.
///
/// Hosts pass [`Runtime::set_viewport`](crate::Runtime::set_viewport) the *native*
/// viewport: the window's size in operating-system logical pixels and the native
/// DPI scale. [`UiZoom`](crate::UiZoom) derives the *effective* viewport that
/// layout, painting and [`Painter::viewport`] see, dividing the logical size by the
/// zoom factor and multiplying the scale by it. The physical extent is therefore
/// unchanged by zoom: zooming shrinks the logical space inside a window of the same
/// physical size, which is why content appears larger.
///
/// Zero width or height is a valid suspended viewport, as reported by a minimized
/// or zero-sized window, and the default before a host supplies one. Layout
/// still runs against it if the runtime is flushed, giving components empty
/// boxes that inspection reports, but presentation suspends and development
/// validation does not check that geometry (see
/// [`Runtime::validation_report`](crate::Runtime::validation_report)).
/// A nonpositive or non-finite scale is invalid input instead.
///
/// Fractional scales are ordinary and never round logical coordinates. The full
/// rules are the [coordinate contract](https://github.com/pertyjons/pert-ui/blob/main/docs/rendering-platform.md).
///
/// A viewport built with [`Viewport::from_physical`] also carries the operating
/// system's integer physical extent, which [`Viewport::physical`] returns
/// unchanged by zoom. The logical size is the result of a division and may be
/// fractional, so `width * scale` need not reproduce that extent; a host sizes
/// its surface from [`Viewport::physical`], never from that product.
///
/// # Serialization
///
/// With the `serde` feature a viewport is `{ width, height, scale, physical }`,
/// `physical` being `null` or `{ width, height }`. Reading one validates the
/// logical size and scale as [`Viewport::new`] does and rejects a physical
/// extent that is zero on an axis where the logical size is not, or the
/// reverse, which no viewport this library builds can have.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    width: f32,
    height: f32,
    scale: f64,
    physical: Option<PhysicalExtent>,
}
impl Default for Viewport {
    fn default() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            scale: 1.0,
            physical: None,
        }
    }
}

/// Integer extent of a native window's drawable area in physical pixels, as the
/// operating system reports it.
///
/// This is the size a presentation surface is configured with. It is not drawing
/// geometry: every [`Rect`], [`Point`](crate::Point) and [`Size`](crate::Size)
/// stays in logical pixels. Every pair of values is valid; a zero width or height
/// is what a minimized or zero-sized window reports, and suspends presentation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct PhysicalExtent {
    width: u32,
    height: u32,
}
impl PhysicalExtent {
    /// Creates an extent from the operating system's physical pixel counts.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
    /// Width in physical pixels.
    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }
    /// Height in physical pixels.
    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }
    /// Whether either dimension is zero, so no surface can be presented.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
}
impl Viewport {
    /// Creates a logical viewport with no physical extent, for headless use and
    /// tests. Zero size suspends native presentation.
    ///
    /// A host presenting to a native surface uses [`Viewport::from_physical`]
    /// instead, so the extent its surface needs stays available.
    ///
    /// # Errors
    /// [`GeometryError::NonFinite`](crate::GeometryError::NonFinite) for a
    /// non-finite dimension or scale,
    /// [`GeometryError::Negative`](crate::GeometryError::Negative) for a negative
    /// dimension and
    /// [`GeometryError::NonPositiveScale`](crate::GeometryError::NonPositiveScale)
    /// for a zero or negative scale.
    pub fn new(width: f32, height: f32, scale: f64) -> Result<Self, crate::GeometryError> {
        Rect::new(0.0, 0.0, width, height)?;
        if !scale.is_finite() {
            return Err(crate::GeometryError::NonFinite);
        }
        if scale <= 0.0 {
            return Err(crate::GeometryError::NonPositiveScale);
        }
        Ok(Self {
            width,
            height,
            scale,
            physical: None,
        })
    }
    /// Creates the native viewport of a window from the operating system's
    /// physical extent and scale factor, and retains that extent.
    ///
    /// The logical size is the extent divided by `scale`, which is the conversion
    /// the [coordinate contract](https://github.com/pertyjons/pert-ui/blob/main/docs/rendering-platform.md)
    /// requires of a host. A zero extent is a valid suspended viewport.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::{PhysicalExtent, Viewport};
    /// let viewport = Viewport::from_physical(PhysicalExtent::new(1001, 601), 1.5)?;
    /// assert_eq!(viewport.width(), (1001.0_f64 / 1.5) as f32);
    /// assert_eq!(viewport.physical(), Some(PhysicalExtent::new(1001, 601)));
    /// # Ok::<(), pert_ui::GeometryError>(())
    /// ```
    ///
    /// # Errors
    /// Rejects a non-finite or nonpositive scale, as [`Self::new`] does.
    pub fn from_physical(extent: PhysicalExtent, scale: f64) -> Result<Self, crate::GeometryError> {
        let logical = Self::new(
            (f64::from(extent.width()) / scale) as f32,
            (f64::from(extent.height()) / scale) as f32,
            scale,
        )?;
        Ok(logical.with_physical(Some(extent)))
    }
    // Zoom derives an effective viewport with a new logical size and scale for the
    // same window, so the extent is copied across rather than recomputed.
    // A component under a view transform records against the window's extent at
    // its own effective scale.
    /// The physical size of a rendered frame of this viewport: the extent the
    /// host supplied through [`Viewport::from_physical`], or otherwise the
    /// logical size times the scale, rounded to the nearest pixel. Every
    /// renderer and capture uses this one rule, so a frame and the regions
    /// cropped from it agree.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::{PhysicalExtent, Viewport};
    /// assert_eq!(Viewport::new(100.0, 50.0, 1.5)?.frame_extent(), PhysicalExtent::new(150, 75));
    /// let native = Viewport::from_physical(PhysicalExtent::new(1001, 601), 1.5)?;
    /// assert_eq!(native.frame_extent(), PhysicalExtent::new(1001, 601));
    /// # Ok::<(), pert_ui::GeometryError>(())
    /// ```
    #[must_use]
    pub fn frame_extent(self) -> PhysicalExtent {
        self.physical.unwrap_or_else(|| {
            PhysicalExtent::new(
                (f64::from(self.width) * self.scale).round() as u32,
                (f64::from(self.height) * self.scale).round() as u32,
            )
        })
    }
    pub(crate) const fn with_scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }
    pub(crate) const fn with_physical(mut self, physical: Option<PhysicalExtent>) -> Self {
        self.physical = physical;
        self
    }
    /// The operating system's physical extent, when the host supplied one through
    /// [`Viewport::from_physical`]; `None` for a viewport built from a logical size.
    ///
    /// [`UiZoom`](crate::UiZoom) leaves it unchanged, so the effective viewport
    /// read from [`Runtime::viewport`](crate::Runtime::viewport) or
    /// [`DisplayList::viewport`] reports exactly the extent the host passed in. It
    /// is absent rather than reconstructed from `width * scale`, because that
    /// product can miss the integer extent by a fraction of a pixel.
    #[must_use]
    pub const fn physical(self) -> Option<PhysicalExtent> {
        self.physical
    }
    /// Logical width in pixels.
    #[must_use]
    pub const fn width(self) -> f32 {
        self.width
    }
    /// Logical height in pixels.
    #[must_use]
    pub const fn height(self) -> f32 {
        self.height
    }
    /// Physical pixels per logical pixel, including native DPI scaling.
    #[must_use]
    pub const fn scale(self) -> f64 {
        self.scale
    }
    /// Converts an operating-system position in physical pixels into this
    /// viewport's logical coordinates, for a custom host translating native input.
    ///
    /// Use the viewport from [`Runtime::viewport`](crate::Runtime::viewport), which
    /// is the effective one. Dividing by the operating system's own scale factor
    /// instead yields positions that are correct only while the user's zoom is
    /// 100%, because zoom multiplies the scale.
    ///
    /// # Errors
    /// [`GeometryError::NonFinite`](crate::GeometryError::NonFinite) for
    /// non-finite input.
    pub fn to_logical(self, x: f64, y: f64) -> Result<crate::Point, crate::GeometryError> {
        crate::Point::new((x / self.scale) as f32, (y / self.scale) as f32)
    }
}

/// A backend-independent drawing operation, in logical window coordinates.
///
/// Backends interpret commands in order, clip to the display list's viewport, and
/// blend in linear light: straight-alpha colors for fills and masks, premultiplied
/// pixels for images. Convert to the destination color space at output. An
/// adapter must report unsupported future commands rather than silently dropping
/// them. These are drawing primitives, not application/widget semantics.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum DrawCommand {
    /// Preserved vector geometry. Backends may tessellate or use [`crate::VectorRasterizer`].
    DrawVector {
        /// Resource in this display list's vector table.
        vector: crate::VectorId,
        /// Translation from geometry-local coordinates into the window.
        origin: crate::Point,
        /// Uniform magnification of the geometry about `origin`, from the view
        /// transforms of the recording component's ancestors; 1 outside any.
        /// Geometry and stroke widths both scale, and a backend prepares the
        /// result at the viewport scale times this factor so it stays sharp.
        scale: f32,
        /// Straight-alpha linear fill/stroke color.
        color: Color,
        /// Additional window-coordinate clipping rectangle.
        clip: Option<Rect>,
    },
    /// Fill a rectangle with a solid color.
    FillRect {
        /// Finite logical bounds; zero area produces no pixels.
        rect: Rect,
        /// Straight-alpha linear RGBA fill.
        color: Color,
    },
    /// Draw a linear alpha-coverage mask, tinted by a straight-alpha color.
    DrawMask {
        /// Destination bounds in logical pixels; the full source maps to these bounds.
        rect: Rect,
        /// Resource reference, valid only within this display list.
        mask: MaskId,
        /// Straight-alpha linear tint; image coverage multiplies its alpha.
        color: Color,
        /// Quarter turn of the source inside `rect`; see
        /// [`QuarterTurn::corners`](crate::QuarterTurn::corners).
        turn: crate::QuarterTurn,
        /// Additional window-coordinate clip; cropping must preserve source UVs.
        clip: Option<Rect>,
    },
    /// Draw a color raster image, multiplied by a straight-alpha tint.
    ///
    /// The whole image maps onto `rect`, whatever its pixel dimensions, and a clip
    /// crops the quad and its source region together, exactly as for
    /// [`DrawCommand::DrawMask`]. Pixels are premultiplied sRGB-encoded RGBA8
    /// (see [`RasterImage`](crate::RasterImage)); a backend samples them as
    /// linear light, multiplies each by the tint's color scaled by its alpha and
    /// by its alpha, and blends the premultiplied result over the destination.
    DrawImage {
        /// Destination bounds in logical pixels; the full source maps to these bounds.
        rect: Rect,
        /// Resource reference, valid only within this display list.
        image: crate::ImageId,
        /// Straight-alpha linear multiplier; [`Color::WHITE`] leaves pixels unchanged
        /// and a lower alpha fades the image uniformly.
        tint: Color,
        /// How source pixels are filtered onto the physical pixels they cover.
        sampling: crate::ImageSampling,
        /// Quarter turn of the source inside `rect`, as for
        /// [`DrawCommand::DrawMask`].
        turn: crate::QuarterTurn,
        /// Additional window-coordinate clip; cropping must preserve source UVs.
        clip: Option<Rect>,
    },
}

/// Opaque index into one owned display list's coverage resources.
///
/// Resolve through [`DisplayList::mask`]. Equal coverage recorded several times in
/// one scene shares one ID, whichever allocation each use came from. Never persist
/// an ID or reuse it in another scene.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MaskId(pub(crate) usize);
impl MaskId {
    /// Position of the mask in [`DisplayList::masks`] of the scene that issued
    /// this ID, for an adapter keeping per-resource data in a parallel table.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Owned immutable row-major 8-bit linear coverage, shared cheaply between scenes.
///
/// Each byte is alpha coverage from 0 to 255. No padding or color channels exist.
/// Construction admits at most [`MaskImage::MAX_PIXELS`] pixels, computes a content key once in time linear
/// in the pixel count, and clones share storage. Two masks are equal when their
/// dimensions and coverage are, whether or not they share an allocation, and a
/// scene stores equal masks once. This is not a callback-safe endpoint;
/// allocations and final deallocation belong outside time-critical callbacks.
#[derive(Clone, Debug)]
pub struct MaskImage {
    width: u32,
    height: u32,
    key: u64,
    pixels: std::sync::Arc<[u8]>,
}
impl PartialEq for MaskImage {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.key == other.key
            && (std::sync::Arc::ptr_eq(&self.pixels, &other.pixels) || self.pixels == other.pixels)
    }
}
impl crate::resources::SceneResource for MaskImage {
    fn content_key(&self) -> u64 {
        self.key
    }
}
impl MaskImage {
    /// The largest mask, in pixels (one byte each): 16 MiB. Code that builds a
    /// mask checks against this before allocating one.
    pub const MAX_PIXELS: usize = 16 * 1024 * 1024;
    /// Validates nonzero dimensions, byte count and the [`Self::MAX_PIXELS`]
    /// image limit.
    ///
    /// # Errors
    /// Returns [`DrawError::InvalidImage`] for a zero dimension, a byte count
    /// other than `width * height`, or more than [`MaskImage::MAX_PIXELS`].
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, DrawError> {
        let count = u64::from(width) * u64::from(height);
        if count == 0 || count > Self::MAX_PIXELS as u64 || count != pixels.len() as u64 {
            return Err(DrawError::InvalidImage);
        }
        Ok(Self {
            width,
            height,
            key: crate::resources::content_key(width, height, &pixels),
            pixels: pixels.into(),
        })
    }
    /// Source width in physical raster pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }
    /// Source height in physical raster pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }
    /// Borrowed immutable coverage bytes, row-major without padding.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
    /// A 64-bit digest of the dimensions and coverage, computed at construction.
    ///
    /// Equal masks always have equal keys, so an adapter can cache an upload
    /// under this key instead of an allocation address and find it again for
    /// equal content from another allocation, scene or window. Different masks
    /// can share a key, rarely, so confirm a cache hit with `==`, which compares
    /// the pixels only when the allocations differ. The value is stable within
    /// one process and not across versions; never persist it.
    #[must_use]
    pub const fn content_key(&self) -> u64 {
        self.key
    }
}

/// Owned drawing commands with no window, GPU handles or live component references.
///
/// Produced by [`crate::Runtime::paint`]. Commands are retained per component;
/// scene composition resolves local coordinates and ancestor clips into ordered
/// window-coordinate commands. Text uses coverage masks; vectors retain paths and
/// styles for backend preparation. Arbitrary transforms
/// remain future work.
///
/// The scene owns typed tables of coverage masks, color images and vectors,
/// addressed by [`MaskId`], [`ImageId`](crate::ImageId) and [`crate::VectorId`]. Each table
/// holds equal content once, so repeated uses of one mask or image share one ID,
/// and holds only resources that a command in [`DisplayList::commands`]
/// references. A resource's content, not its allocation, is its identity, which
/// is what lets a backend cache uploads across scenes.
#[derive(Clone, Debug, PartialEq)]
pub struct DisplayList {
    pub(crate) background: Color,
    pub(crate) viewport: Viewport,
    pub(crate) fills: Vec<DrawCommand>,
    pub(crate) masks: crate::resources::Arena<MaskImage>,
    pub(crate) images: crate::resources::Arena<crate::RasterImage>,
    pub(crate) vectors: crate::resources::Arena<crate::VectorGeometry>,
}
impl Default for DisplayList {
    fn default() -> Self {
        Self {
            background: Color::BACKGROUND,
            viewport: Viewport::default(),
            fills: Vec::new(),
            masks: crate::resources::Arena::default(),
            images: crate::resources::Arena::default(),
            vectors: crate::resources::Arena::default(),
        }
    }
}
impl DisplayList {
    /// Preserved vector resources, indexed by [`crate::VectorId`].
    pub fn vectors(&self) -> &[crate::VectorGeometry] {
        self.vectors.items()
    }
    /// Resolves a scene-local vector ID; a foreign ID may refer to different content.
    pub fn vector(&self, id: crate::VectorId) -> Option<&crate::VectorGeometry> {
        self.vectors.items().get(id.0)
    }
    /// Explicit CPU coverage fallback. Prefer retaining a [`crate::VectorRasterizer`]
    /// in a backend to reuse unchanged geometry.
    /// The input scene remains intact on failure.
    ///
    /// # Errors
    /// The errors of [`VectorRasterizer::prepare`](crate::VectorRasterizer::prepare).
    pub fn rasterized(&self) -> Result<Self, DrawError> {
        crate::VectorRasterizer::default().prepare(self)
    }

    /// Scene clear color, resolved from the active theme's Background role.
    /// Adapters clear to this linear RGBA color before drawing ordered commands.
    /// An empty display list still has a background; default is the built-in dark one.
    #[must_use]
    pub const fn background(&self) -> Color {
        self.background
    }

    /// Resolves a mask resource in this scene, or `None` for a foreign/out-of-range index.
    /// IDs cannot detect same-index references copied from another display list.
    #[must_use]
    pub fn mask(&self, id: MaskId) -> Option<&MaskImage> {
        self.masks.items().get(id.0)
    }
    /// All owned coverage resources, each distinct content once, indexed by
    /// [`MaskId`]. Adapters may cache uploads while these remain used.
    #[must_use]
    pub fn masks(&self) -> &[MaskImage] {
        self.masks.items()
    }
    /// Resolves a color image in this scene, or `None` for a foreign/out-of-range
    /// index. IDs cannot detect same-index references copied from another scene.
    #[must_use]
    pub fn image(&self, id: crate::ImageId) -> Option<&crate::RasterImage> {
        self.images.items().get(id.0)
    }
    /// All owned color images, each distinct content once, indexed by
    /// [`ImageId`](crate::ImageId). Adapters may cache uploads while these remain used.
    #[must_use]
    pub fn images(&self) -> &[crate::RasterImage] {
        self.images.items()
    }
    /// Window extent and scale for interpreting commands in any graphics backend.
    #[must_use]
    pub const fn viewport(&self) -> Viewport {
        self.viewport
    }
    /// Ordered immutable primitives; this slice borrows no component or GPU state.
    ///
    /// External adapters can rasterize these into a texture, canvas or host UI.
    /// Window creation and presentation remain the adapter/host's responsibility.
    #[must_use]
    pub fn commands(&self) -> &[DrawCommand] {
        &self.fills
    }

    /// Number of commands in the scene, including fully transparent ones.
    ///
    /// Composition omits a command that its clip has already reduced to nothing,
    /// so this counts what a backend has to encode, not what the components in the
    /// tree recorded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fills.len()
    }
    /// Whether no commands were recorded; the same as `len() == 0`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fills.is_empty()
    }
}

/// Command recorder passed to [`crate::Component::paint`].
///
/// Record from retained component data. Binding reads belong in `update`, where
/// dependency tracking happens. Paint must not mutate state or perform I/O.
/// Commands remain cached until this component updates, a read theme role changes,
/// or the viewport changes.
pub struct Painter {
    pub(crate) font_reads: std::cell::Cell<u64>,
    pub(crate) text_drawn: std::cell::Cell<bool>,
    pub(crate) text_rendering: crate::TextRenderingMode,
    pub(crate) physical_origin: [f64; 2],
    pub(crate) physical_scale: f64,
    pub(crate) text: std::rc::Rc<crate::text::TextCache>,
    pub(crate) profiler: crate::profiling::Profiler,
    pub(crate) shapes: std::rc::Rc<std::cell::RefCell<crate::shapes::ShapeCache>>,
    pub(crate) focus_within: bool,
    pub(crate) view_scale: f32,
    pub(crate) window_scale: f64,
    pub(crate) focus_read: std::cell::Cell<bool>,
    pub(crate) metrics: crate::ThemeMetrics,
    pub(crate) metric_roles: std::cell::Cell<u64>,
    pub(crate) palette: crate::Palette,
    pub(crate) roles: std::cell::Cell<u64>,
    pub(crate) size: Option<crate::Size>,
    pub(crate) clip: Option<Rect>,
    pub(crate) list: DisplayList,
}
impl Painter {
    /// Prepares text for presentation inside geometry already reserved in layout.
    /// Use `MeasureContext::text` when content affects preferred size. This method
    /// tracks the style's size as a paint dependency; it never requests layout or
    /// reads bindings. Read binding content in `Component::update` with `read_paint`.
    ///
    /// Width is optional logical wrapping space. Preparation uses the same UI-local
    /// cache, bundled fonts, effective scale, 128-entry/16 MiB budget and typed
    /// text errors as `MeasureContext::text`. Repainting identical text reuses its
    /// coverage. Returned owned layouts can outlive cache eviction.
    ///
    /// # Errors
    /// Returns [`DrawError::Runtime`] holding the errors of
    /// [`MeasureContext::text`](crate::MeasureContext::text):
    /// [`RuntimeError::Text`](crate::RuntimeError::Text) for invalid geometry (a
    /// non-finite or negative width, a zero font size), an unreadable font or a result
    /// beyond a [`TextLimit`](crate::TextLimit);
    /// [`RuntimeError::TextEdit`](crate::RuntimeError::TextEdit) with
    /// [`TextEditError::TooLarge`](crate::TextEditError::TooLarge) for text longer than
    /// [`TextEditError::MAX_TEXT_BYTES`](crate::TextEditError::MAX_TEXT_BYTES); and
    /// `BorrowConflict` when text is prepared reentrantly. The failed paint stays pending
    /// for retry.
    pub fn text(
        &self,
        text: &str,
        width: Option<f32>,
        style: crate::TextStyle,
    ) -> Result<crate::TextLayout, DrawError> {
        let _timing = self.profiler.span(crate::ProfileOperation::Text, None, "");
        let size = self.metric(style.metric());
        Ok(self.text.prepare(
            text,
            crate::text::TextRequest::new(style, size, &self.font_reads)
                .width(width)
                .scale(crate::runtime::raster_scale(
                    self.window_scale,
                    self.view_scale,
                )),
        )?)
    }
    /// Prepares rich text for geometry already reserved in layout. Font/icon and
    /// size reads subscribe paint only. Prefer `MeasureContext::rich_text` when
    /// content affects preferred size. Span colors resolve when the layout is painted.
    ///
    /// # Errors
    /// Returns [`DrawError::Runtime`] holding the errors of
    /// [`MeasureContext::rich_text`](crate::MeasureContext::rich_text).
    pub fn rich_text(
        &self,
        text: &crate::RichText,
        width: Option<f32>,
        style: crate::TextStyle,
    ) -> Result<crate::TextLayout, DrawError> {
        let _timing = self.profiler.span(crate::ProfileOperation::Text, None, "");
        let (icon_size, gap) = text.metrics(|role| self.metric(role));
        let size = self.metric(style.metric());
        Ok(self.text.prepare_rich(
            text,
            crate::text::TextRequest::new(style, size, &self.font_reads)
                .width(width)
                .scale(crate::runtime::raster_scale(
                    self.window_scale,
                    self.view_scale,
                ))
                .inline(icon_size, gap),
        )?)
    }
    /// Paints a standard or custom font icon centered inside reserved logical bounds.
    /// Nominal font size is the shorter side. Ink is centered and shrunk uniformly
    /// only if it overflows, preserving aspect ratio. Coverage uses the next power
    /// of two in physical pixels, capped at
    /// [`TextError::MAX_FONT_PIXELS`](crate::TextError::MAX_FONT_PIXELS); larger
    /// icons upscale that bounded raster. Sizes in the same bucket share coverage during resize.
    /// Zero-area bounds draw nothing. Mapping reads invalidate paint only;
    /// the caller owns layout/hit bounds. Built-in indicators use this same path.
    ///
    /// # Errors
    /// As [`Self::icon_turned`].
    pub fn icon(
        &mut self,
        icon: impl Into<crate::IconSource>,
        rect: Rect,
        color: Color,
    ) -> Result<(), DrawError> {
        self.icon_turned(icon, rect, color, crate::QuarterTurn::None)
    }
    /// Paints an icon turned by a quarter turn, centered inside `rect` with the
    /// rules of [`Self::icon`]. The upright icon's coverage is reused, so a
    /// disclosure arrow can point four ways from one rasterization.
    ///
    /// # Errors
    /// Returns [`DrawError::Runtime`] holding
    /// [`RuntimeError::Font`](crate::RuntimeError::Font) with
    /// [`FontError::ForeignFont`](crate::FontError::ForeignFont) for a custom icon glyph
    /// of another runtime, or [`RuntimeError::Text`](crate::RuntimeError::Text) when the
    /// icon's coverage cannot be prepared (see
    /// [`MeasureContext::rich_text`](crate::MeasureContext::rich_text)).
    /// Zero-area bounds never fail.
    pub fn icon_turned(
        &mut self,
        icon: impl Into<crate::IconSource>,
        rect: Rect,
        color: Color,
        turn: crate::QuarterTurn,
    ) -> Result<(), DrawError> {
        let [_, _, w, h] = rect.values();
        if w == 0.0 || h == 0.0 {
            return Ok(());
        }
        let size = w.min(h);
        let physical =
            (f64::from(size) * self.viewport().scale()).clamp(1.0, crate::text::MAX_FONT_PIXELS);
        let raster_size = (physical.ceil() as u32).next_power_of_two() as f32;
        let text = self.text.prepare_rich(
            &crate::RichText::new().icon(icon),
            crate::text::TextRequest::new(crate::TextStyle::Body, raster_size, &self.font_reads)
                .inline(raster_size, 0.0),
        )?;
        text.paint_icon(self, rect, color, size / raster_size, turn)
    }

    /// Whether this component or a descendant has focus. Reading subscribes paint
    /// to focus transfers, loss, hiding and removal; it does not make this component
    /// focusable. Useful for a panel outline containing focused controls.
    #[must_use]
    pub fn focus_within(&self) -> bool {
        self.focus_read.set(true);
        self.focus_within
    }

    /// Resolves a logical theme measurement and tracks a paint-only dependency.
    /// Use `MeasureContext::metric` instead when a value affects layout or hit regions.
    #[must_use]
    pub fn metric(&self, role: crate::MetricRole) -> f32 {
        self.metric_roles.set(self.metric_roles.get() | role.mask());
        self.metrics.get(role)
    }

    /// Resolves a semantic color and tracks a paint dependency for this component.
    ///
    /// Reads in both paint callbacks are collected together on successful recording.
    /// Changing a read role refreshes this component without invalidating geometry;
    /// roles no longer read are unsubscribed. Literal `Color` values remain fixed.
    /// This is the same public path used by standard controls.
    #[must_use]
    pub fn color(&self, role: crate::ColorRole) -> Color {
        self.roles.set(self.roles.get() | role.mask());
        self.palette.color(role)
    }

    /// Records a shared coverage image at local logical bounds, using the active clip.
    /// Adapters multiply image coverage by tint alpha and blend in linear space.
    ///
    /// The whole image maps onto `rect`, whatever its pixel dimensions. Unlike
    /// [`Self::fill_rect`], the destination is recorded unclipped and the active
    /// clip travels with the command, because shrinking the destination would
    /// rescale the image rather than crop it; the backend crops the quad and its
    /// source region together, so a cropped mask keeps the sampling it would have
    /// had uncropped.
    pub fn draw_mask(&mut self, image: &MaskImage, rect: Rect, color: Color) {
        self.draw_mask_turned(image, rect, color, crate::QuarterTurn::None);
    }
    /// Records a coverage image turned by a quarter turn inside `rect`.
    ///
    /// `rect` is where the turned picture appears, so for
    /// [`QuarterTurn::Right`](crate::QuarterTurn::Right) and
    /// [`QuarterTurn::Left`](crate::QuarterTurn::Left) its width corresponds to the
    /// image's height. The same coverage is sampled: nothing is rasterized again
    /// and the scene shares the resource with upright uses. Placement and clipping
    /// are otherwise those of [`Self::draw_mask`].
    pub fn draw_mask_turned(
        &mut self,
        image: &MaskImage,
        rect: Rect,
        color: Color,
        turn: crate::QuarterTurn,
    ) {
        let mask = MaskId(self.list.masks.intern(image));
        self.list.fills.push(DrawCommand::DrawMask {
            rect,
            mask,
            color,
            turn,
            clip: self.clip,
        });
    }
    /// Records a color image at local logical bounds, using the active clip.
    ///
    /// The whole image maps onto `rect`, whatever its pixel dimensions, so the
    /// caller chooses the aspect ratio; [`Image`](crate::Image) does that for
    /// ordinary use. As with [`Self::draw_mask`], the destination is recorded
    /// unclipped and the active clip travels with the command, so a clip crops
    /// the image rather than rescaling it. Recording clones a shared handle; it
    /// neither copies nor converts pixels, and equal images recorded several times
    /// share one entry in the scene.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::*;
    /// struct Swatch(RasterImage);
    /// impl Component for Swatch {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
    ///         Ok(())
    ///     }
    ///     fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
    ///         let rect = Rect::new(0.0, 0.0, 32.0, 32.0)?;
    ///         painter.draw_image(&self.0, rect, ImageSampling::Nearest);
    ///         Ok(())
    ///     }
    /// }
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(64.0, 64.0, 1.0)?)?;
    /// let pixels = [0, 0, 0, 255, 255, 255, 255, 255].repeat(2);
    /// ui.mount("swatch", Swatch(RasterImage::from_srgba8(2, 2, pixels)?))?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// let scene = ui.paint()?;
    /// assert_eq!(scene.images().len(), 1);
    /// assert!(matches!(
    ///     scene.commands()[0],
    ///     DrawCommand::DrawImage { sampling: ImageSampling::Nearest, .. }
    /// ));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn draw_image(
        &mut self,
        image: &crate::RasterImage,
        rect: Rect,
        sampling: crate::ImageSampling,
    ) {
        self.draw_image_tinted(image, rect, sampling, Color::WHITE);
    }
    /// Records a color image multiplied by a straight-alpha linear tint.
    ///
    /// Each pixel's color is multiplied by the tint's color and its alpha by the
    /// tint's alpha, so `Color::rgba(1.0, 1.0, 1.0, 0.5)` fades the image to half
    /// opacity and [`Color::WHITE`] leaves it unchanged. A fully transparent tint
    /// draws nothing. Placement and clipping are those of [`Self::draw_image`].
    pub fn draw_image_tinted(
        &mut self,
        image: &crate::RasterImage,
        rect: Rect,
        sampling: crate::ImageSampling,
        tint: Color,
    ) {
        self.draw_image_turned(image, rect, sampling, tint, crate::QuarterTurn::None);
    }
    /// Records a tinted color image turned by a quarter turn inside `rect`, with
    /// the placement rule of [`Self::draw_mask_turned`].
    pub fn draw_image_turned(
        &mut self,
        image: &crate::RasterImage,
        rect: Rect,
        sampling: crate::ImageSampling,
        tint: Color,
        turn: crate::QuarterTurn,
    ) {
        let id = crate::ImageId(self.list.images.intern(image));
        self.list.fills.push(DrawCommand::DrawImage {
            rect,
            image: id,
            tint,
            sampling,
            turn,
            clip: self.clip,
        });
    }
    /// Allocated component size in logical pixels, or `None` for legacy roots.
    /// Composed components draw relative to their own top-left origin.
    #[must_use]
    pub const fn size(&self) -> Option<crate::Size> {
        self.size
    }
    /// Records a clipped group in local coordinates, intersecting any outer clip.
    /// The previous clip is restored even if the callback returns an error.
    ///
    /// Local clips compose with the ancestor clip applied during composition and the
    /// viewport clip applied by the backend. A group's clip never leaks to later
    /// siblings, including when its callback panics.
    pub fn with_clip<T>(&mut self, clip: Rect, draw: impl FnOnce(&mut Self) -> T) -> T {
        let previous = self.clip;
        self.clip = Some(previous.map_or(clip, |outer| outer.intersection(clip)));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| draw(self)));
        self.clip = previous;
        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    /// Current logical window size, and the scale at which this component's
    /// coverage is prepared. This is not component layout bounds.
    ///
    /// The scale is physical pixels per local logical pixel: the window's
    /// effective scale outside a [view transform](crate::ViewTransform), and that
    /// scale times [`Self::view_scale`] inside one, held at eight physical pixels
    /// per local pixel when a view magnifies further (coverage is stretched
    /// beyond that). The extent stays the window's and is not converted.
    #[must_use]
    pub const fn viewport(&self) -> Viewport {
        self.list.viewport
    }
    /// Accumulated [view scale](crate::ViewTransform) of this component's local
    /// space that this drawing is prepared for, 1 outside any view transform.
    ///
    /// Everything recorded here is magnified by the view, stroke widths
    /// included. Divide a length by this scale to keep a constant on-screen size,
    /// such as a cable that stays two pixels wide at every zoom. During a
    /// continuous zoom the runtime keeps a component's drawing and stretches it
    /// until the scale rests, so this is then the scale it was last prepared at,
    /// and the component paints again at the scale it is shown at once the
    /// gesture rests; see [`Component::child_transform`](crate::Component::child_transform).
    #[must_use]
    pub const fn view_scale(&self) -> f32 {
        self.view_scale
    }
    /// Appends a solid rectangle. Later commands draw over earlier commands.
    ///
    /// The rectangle is intersected with the active clip when recorded and only the
    /// result is kept, since intersecting a rectangle with a rectangle is exact. An
    /// empty intersection is a zero-area rectangle, never a negative one, and
    /// produces no pixels. Compare [`Self::draw_mask`], which must carry its clip.
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let rect = self.clip.map_or(rect, |clip| rect.intersection(clip));
        self.list.fills.push(DrawCommand::FillRect { rect, color });
    }
}

// The serde representation of a viewport; the type documents it.
#[cfg(feature = "serde")]
mod serde_impls {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

    use super::{PhysicalExtent, Viewport};

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct Repr {
        width: f32,
        height: f32,
        scale: f64,
        physical: Option<PhysicalExtent>,
    }

    impl Serialize for Viewport {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            Repr {
                width: self.width,
                height: self.height,
                scale: self.scale,
                physical: self.physical,
            }
            .serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for Viewport {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let repr = Repr::deserialize(deserializer)?;
            let viewport =
                Self::new(repr.width, repr.height, repr.scale).map_err(D::Error::custom)?;
            // A physical extent comes from dividing itself by the scale, and zoom
            // keeps it, so an axis is zero physically exactly when it is zero
            // logically. Anything else was not produced by this library.
            if let Some(extent) = repr.physical
                && ((extent.width() == 0) != (repr.width == 0.0)
                    || (extent.height() == 0) != (repr.height == 0.0))
            {
                return Err(D::Error::custom(
                    "physical extent disagrees with the logical size",
                ));
            }
            Ok(viewport.with_physical(repr.physical))
        }
    }
}
