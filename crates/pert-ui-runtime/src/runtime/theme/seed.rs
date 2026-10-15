//! Seed palettes: a built-in palette recolored from seed colors.
//!
//! Generation works in Oklch, the polar form of Björn Ottosson's Oklab, on
//! linear sRGB. Each generated role takes the hue rotation and chroma factor
//! that map a built-in reference role onto its seed, and is then solved back to
//! its built-in color's WCAG relative luminance, so every luminance contrast of
//! the built-in palette holds for any seed.

use super::{COLOR_ROLE_COUNT, COLOR_ROLES, COLOR_SOURCES, ColorRole, Source, ThemeMode};
use crate::{Color, OpaqueColor};

/// How a role is generated from the seeds.
#[derive(Clone, Copy, PartialEq)]
enum Class {
    /// Takes the primary seed's hue rotation and full chroma factor.
    Accent,
    /// Takes the secondary seed's transform, or the primary one without it.
    Secondary,
    /// Takes the primary hue rotation with the chroma factor capped at one.
    Neutral,
    /// Keeps its built-in value: status colors and foregrounds.
    Fixed,
    /// Has no value of its own; it follows another role.
    Follower,
}

const fn class(role: ColorRole) -> Class {
    use ColorRole::*;
    match role {
        Primary | FocusRing | PrimaryContainer | TextSelection | Link | TabSelected => {
            Class::Accent
        }
        Secondary | SecondaryContainer => Class::Secondary,
        Background | Surface | SurfaceHover | Muted | Outline | OnSurface | Button
        | ButtonHover | Tab | Divider | ResizeHandle | ResizeHandleHover | SurfaceContainer
        | ScrollbarThumb | ScrollbarThumbHover => Class::Neutral,
        OnPrimary | OnSecondary | Error | OnError | Warning | OnWarning | Info | OnInfo
        | Success | OnSuccess | FocusRingContrast | ErrorContainer | WarningContainer
        | InfoContainer | SuccessContainer | Highlight => Class::Fixed,
        OnButton | ButtonPressed | OnButtonPressed | OnTab | PanelBorder | OnSurfaceContainer
        | OnPrimaryContainer | OnSecondaryContainer | OnErrorContainer | OnWarningContainer
        | OnInfoContainer | OnSuccessContainer | OnTextSelection => Class::Follower,
    }
}

// Exactly the roles that follow another are classed as followers, so a role
// moved between its own value and following cannot be generated wrongly.
const _: () = {
    let mut index = 0;
    while index < COLOR_ROLE_COUNT {
        let follower = matches!(class(COLOR_ROLES[index]), Class::Follower);
        assert!(
            follower == matches!(COLOR_SOURCES[index], Source::Follows(_)),
            "a seed class disagrees with the role's default source"
        );
        index += 1;
    }
};

/// Bisection steps for lightness and for chroma; each halves an interval no
/// wider than one, well past `f64` resolution of the result.
const STEPS: usize = 60;
/// Largest accepted difference between a result's luminance and its target.
const LUMINANCE_TOLERANCE: f64 = 1e-6;
/// How far a channel may fall outside `0..=1` before clamping and still count
/// as in gamut.
const GAMUT_TOLERANCE: f64 = 1e-9;

/// The colors of every role with its own value for `mode`, recolored from the
/// seeds. A follower's entry holds its source's generated color and is never
/// read, since a palette resolves followers through their source. A role the
/// solve could not place keeps its built-in value; the solve accepts gray at
/// any luminance, so this fallback is not expected to occur.
pub(super) fn generate(
    mode: ThemeMode,
    primary: OpaqueColor,
    secondary: Option<OpaqueColor>,
) -> [Color; COLOR_ROLE_COUNT] {
    let generated = try_generate(mode, primary, secondary);
    std::array::from_fn(|index| {
        generated[index].unwrap_or_else(|| built_in(COLOR_ROLES[index], mode))
    })
}

/// [`generate`] with `None` for a role the solve could not place.
fn try_generate(
    mode: ThemeMode,
    primary: OpaqueColor,
    secondary: Option<OpaqueColor>,
) -> [Option<Color>; COLOR_ROLE_COUNT] {
    let primary = Transform::mapping(built_in(ColorRole::Primary, mode), primary);
    let secondary = secondary.map_or(primary, |seed| {
        Transform::mapping(built_in(ColorRole::Secondary, mode), seed)
    });
    let own = |role: ColorRole| {
        let color = built_in(role, mode);
        match class(role) {
            Class::Accent => recolor(color, primary, f64::INFINITY),
            Class::Secondary => recolor(color, secondary, f64::INFINITY),
            Class::Neutral => recolor(color, primary, 1.0),
            Class::Fixed | Class::Follower => Some(color),
        }
    };
    std::array::from_fn(|index| match COLOR_SOURCES[index] {
        Source::Own { .. } => own(COLOR_ROLES[index]),
        Source::Follows(source) => own(source),
    })
}

/// The built-in value of a role with its own value, or of the role it follows.
fn built_in(role: ColorRole, mode: ThemeMode) -> Color {
    match COLOR_SOURCES[role as usize] {
        Source::Own { dark, light } => match mode {
            ThemeMode::Dark => dark,
            ThemeMode::Light => light,
        },
        Source::Follows(source) => built_in(source, mode),
    }
}

/// The hue rotation and chroma factor that map a reference color onto a seed.
#[derive(Clone, Copy)]
struct Transform {
    rotation: f64,
    chroma: f64,
}
impl Transform {
    fn mapping(reference: Color, seed: OpaqueColor) -> Self {
        let reference = Lch::of(reference);
        let seed = Lch::of(Color::from(seed));
        Self {
            rotation: seed.hue - reference.hue,
            // A built-in reference is saturated; an achromatic one would
            // leave nothing to scale, so it maps to gray.
            chroma: if reference.chroma > 0.0 {
                seed.chroma / reference.chroma
            } else {
                0.0
            },
        }
    }
}

/// Applies `transform` to `color`, with its chroma factor capped at
/// `max_factor`, at `color`'s luminance and alpha, or `None` when the solve
/// cannot place it.
fn recolor(color: Color, transform: Transform, max_factor: f64) -> Option<Color> {
    let [red, green, blue, alpha] = color.channels();
    let rgb = [red, green, blue].map(f64::from);
    let lch = Lch::from_linear(rgb);
    let chroma = lch.chroma * transform.chroma.min(max_factor);
    let [red, green, blue] = solve(luminance(rgb), chroma, lch.hue + transform.rotation)?;
    Color::rgba(red as f32, green as f32, blue as f32, alpha).ok()
}

/// The most saturated color, up to `chroma`, of hue `hue` and luminance
/// `target` that lies in the sRGB gamut. Luminance is not monotonic in
/// lightness at every chroma outside the gamut, so each candidate is accepted
/// only on its own result; chroma zero, a gray, is accepted for any target.
fn solve(target: f64, chroma: f64, hue: f64) -> Option<[f64; 3]> {
    if let Some(rgb) = accept(target, chroma, hue) {
        return Some(rgb);
    }
    let (mut low, mut high) = (0.0, chroma);
    for _ in 0..STEPS {
        let middle = (low + high) / 2.0;
        if accept(target, middle, hue).is_some() {
            low = middle;
        } else {
            high = middle;
        }
    }
    accept(target, low, hue)
}

/// The color of chroma `chroma` and hue `hue` whose lightness bisection meets
/// `target`, if it is finite, in gamut and within the luminance tolerance.
fn accept(target: f64, chroma: f64, hue: f64) -> Option<[f64; 3]> {
    let at = |lightness| {
        Lch {
            lightness,
            chroma,
            hue,
        }
        .to_linear()
    };
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..STEPS {
        let middle = (low + high) / 2.0;
        if luminance(at(middle)) < target {
            low = middle;
        } else {
            high = middle;
        }
    }
    let rgb = at((low + high) / 2.0);
    if !rgb
        .iter()
        .all(|channel| (-GAMUT_TOLERANCE..=1.0 + GAMUT_TOLERANCE).contains(channel))
    {
        return None;
    }
    let rgb = rgb.map(|channel| channel.clamp(0.0, 1.0));
    ((luminance(rgb) - target).abs() <= LUMINANCE_TOLERANCE).then_some(rgb)
}

/// WCAG 2.x relative luminance of linear sRGB channels, as
/// [`OpaqueColor::relative_luminance`] measures it.
fn luminance([red, green, blue]: [f64; 3]) -> f64 {
    0.2126 * red + 0.7152 * green + 0.0722 * blue
}

/// An Oklch color: Oklab lightness, chroma and hue in radians.
#[derive(Clone, Copy, Debug)]
struct Lch {
    lightness: f64,
    chroma: f64,
    hue: f64,
}
impl Lch {
    fn of(color: Color) -> Self {
        let [red, green, blue, _] = color.channels().map(f64::from);
        Self::from_linear([red, green, blue])
    }
    fn from_linear([red, green, blue]: [f64; 3]) -> Self {
        let lms = [
            0.412_221_470_8 * red + 0.536_332_536_3 * green + 0.051_445_992_9 * blue,
            0.211_903_498_2 * red + 0.680_699_545_1 * green + 0.107_396_956_6 * blue,
            0.088_302_461_9 * red + 0.281_718_837_6 * green + 0.629_978_700_5 * blue,
        ]
        .map(f64::cbrt);
        let [l, m, s] = lms;
        let a = 1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s;
        let b = 0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s;
        Self {
            lightness: 0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
            chroma: a.hypot(b),
            hue: b.atan2(a),
        }
    }
    fn to_linear(self) -> [f64; 3] {
        let (a, b) = (self.chroma * self.hue.cos(), self.chroma * self.hue.sin());
        let l = (self.lightness + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
        let m = (self.lightness - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
        let s = (self.lightness - 0.089_484_177_5 * a - 1.291_485_548_0 * b).powi(3);
        [
            4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
            -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
            -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opaque(red: f32, green: f32, blue: f32) -> OpaqueColor {
        OpaqueColor::try_from(Color::linear_rgb(red, green, blue)).expect("opaque")
    }

    /// Oklab's published reference for sRGB red, and white and black at the
    /// ends of the lightness axis.
    #[test]
    fn oklch_matches_reference_values_and_round_trips() {
        let red = Lch::of(Color::linear_rgb(1.0, 0.0, 0.0));
        let (a, b) = (red.chroma * red.hue.cos(), red.chroma * red.hue.sin());
        assert!((red.lightness - 0.627_955).abs() < 1e-5, "{red:?}");
        assert!((a - 0.224_863).abs() < 1e-5 && (b - 0.125_846).abs() < 1e-5);
        let white = Lch::of(Color::linear_rgb(1.0, 1.0, 1.0));
        assert!((white.lightness - 1.0).abs() < 1e-6 && white.chroma < 1e-6);
        assert_eq!(Lch::of(Color::linear_rgb(0.0, 0.0, 0.0)).lightness, 0.0);
        for color in [
            [0.08, 0.32, 0.85],
            [0.95, 0.48, 0.06],
            [0.018, 0.022, 0.032],
        ] {
            let back = Lch::of(Color::linear_rgb(color[0], color[1], color[2])).to_linear();
            for (channel, expected) in back.iter().zip(color) {
                assert!((channel - f64::from(expected)).abs() < 1e-6, "{back:?}");
            }
        }
    }

    /// Seeding a mode with its own built-in `Primary`, and `Secondary`,
    /// reproduces that mode's built-in colors.
    #[test]
    fn seeding_a_mode_with_its_own_accents_reproduces_it() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let seed = |role| OpaqueColor::try_from(built_in(role, mode)).expect("opaque");
            let primary = seed(ColorRole::Primary);
            for secondary in [None, Some(seed(ColorRole::Secondary))] {
                let colors = generate(mode, primary, secondary);
                for role in COLOR_ROLES {
                    let (got, want) = (colors[role as usize], built_in(role, mode));
                    let close = got
                        .channels()
                        .iter()
                        .zip(want.channels())
                        .all(|(a, b)| (a - b).abs() <= 1e-5);
                    assert!(close, "{mode:?} {role:?}: {got:?} vs {want:?}");
                }
            }
        }
    }

    #[test]
    fn every_seed_class_matches_its_default_source() {
        for role in COLOR_ROLES {
            assert_eq!(
                class(role) == Class::Follower,
                role.follows().is_some(),
                "{role:?}"
            );
        }
    }

    /// The solve lands on its luminance in gamut for targets from black to
    /// white and chromas from gray to beyond any sRGB color, and falls back to
    /// less chroma rather than another hue or luminance.
    #[test]
    fn the_solve_meets_its_luminance_in_gamut_at_the_edges() {
        for target in [0.0, 1e-4, 0.01, 0.18, 0.5, 0.9, 1.0] {
            for chroma in [0.0, 0.05, 0.2, 0.4, 1.0] {
                for degrees in (0..360).step_by(15) {
                    let hue = f64::from(degrees).to_radians();
                    let rgb = solve(target, chroma, hue)
                        .unwrap_or_else(|| panic!("{target} {chroma} {degrees}"));
                    assert!(rgb.iter().all(|c| c.is_finite() && (0.0..=1.0).contains(c)));
                    assert!((luminance(rgb) - target).abs() <= LUMINANCE_TOLERANCE);
                    let result = Lch::from_linear(rgb);
                    assert!(result.chroma <= chroma + 1e-4, "{result:?}");
                    if result.chroma > 0.02 {
                        let turn = (result.hue - hue).rem_euclid(std::f64::consts::TAU);
                        let off = turn.min(std::f64::consts::TAU - turn);
                        assert!(off < 1e-3, "hue moved by {off} at {target} {chroma}");
                    }
                }
            }
        }
    }

    /// Every generated role is placed by the solve, in gamut and at its
    /// built-in luminance and alpha, for seeds across the linear RGB cube and
    /// for directed edge families: very dark saturated colors and grays with
    /// a trace of chroma, whose hue is unstable.
    #[test]
    fn every_generated_role_keeps_its_luminance_for_every_seed_family() {
        const LEVELS: [f32; 7] = [0.0, 0.001, 0.02, 0.1, 0.3, 0.6, 1.0];
        let mut seeds = Vec::new();
        for red in LEVELS {
            for green in LEVELS {
                for blue in LEVELS {
                    seeds.push([red, green, blue]);
                }
            }
        }
        for dark in [1e-4, 1e-3, 5e-3] {
            seeds.extend([
                [dark, 0.0, 0.0],
                [0.0, dark, 0.0],
                [0.0, 0.0, dark],
                [dark, 0.0, dark],
            ]);
        }
        for gray in [0.0_f32, 0.01, 0.2, 0.5, 1.0] {
            for trace in [1e-6, 1e-4] {
                seeds.extend([
                    [(gray + trace).min(1.0), gray, gray],
                    [gray, gray, (gray + trace).min(1.0)],
                ]);
            }
        }
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            for [red, green, blue] in &seeds {
                let primary = opaque(*red, *green, *blue);
                for secondary in [None, Some(opaque(*blue, *red, *green))] {
                    let colors = try_generate(mode, primary, secondary);
                    for role in COLOR_ROLES {
                        let want = built_in(role, mode);
                        let got = colors[role as usize].unwrap_or_else(|| {
                            panic!("{mode:?} {role:?} unplaced for {red} {green} {blue}")
                        });
                        let [r, g, b, a] = got.channels();
                        assert!([r, g, b].iter().all(|c| (0.0..=1.0).contains(c)));
                        assert_eq!(a, want.alpha(), "{role:?} keeps its alpha");
                        let [wr, wg, wb, _] = want.channels();
                        let (got, want) = (
                            luminance([r, g, b].map(f64::from)),
                            luminance([wr, wg, wb].map(f64::from)),
                        );
                        assert!(
                            (got - want).abs() <= 1e-5,
                            "{mode:?} {role:?} luminance {got} vs {want} for {red} {green} {blue}"
                        );
                    }
                }
            }
        }
    }

    /// Black, white and gray seeds have no hue; each gives gray accents and
    /// the same palette, at the built-in luminances.
    #[test]
    fn achromatic_seeds_give_one_gray_palette() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let black = generate(mode, opaque(0.0, 0.0, 0.0), None);
            for seed in [opaque(1.0, 1.0, 1.0), opaque(0.2, 0.2, 0.2)] {
                let other = generate(mode, seed, None);
                for (index, (a, b)) in black.iter().zip(&other).enumerate() {
                    let close = a
                        .channels()
                        .iter()
                        .zip(b.channels())
                        .all(|(x, y)| (x - y).abs() < 1e-5);
                    assert!(close, "{:?}: {a:?} {b:?}", COLOR_ROLES[index]);
                }
            }
            let primary = Lch::of(black[ColorRole::Primary as usize]);
            assert!(primary.chroma < 1e-3, "{primary:?}");
        }
    }
}
