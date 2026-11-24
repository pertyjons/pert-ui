// Scalar CPU sampler and f64 reference share the mathematical contract and
// differ in arithmetic precision. GPU implementation is independent WGSL.
use bytemuck::{Pod, Zeroable};
use num_traits::Float;

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Params {
    pub geom: [f32; 4],
    pub misc: [f32; 4],
    pub mode: [u32; 4],
    pub options: [u32; 4],
}
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Stop {
    pub color: [f32; 4],
    pub data: [f32; 4],
}

pub fn encode<F: Float>(x: F) -> F {
    let n = |x: f64| F::from(x).unwrap();
    if x <= n(0.0031308) {
        n(12.92) * x
    } else {
        n(1.055) * x.powf(n(1. / 2.4)) - n(0.055)
    }
}
pub fn decode<F: Float>(x: F) -> F {
    let n = |x: f64| F::from(x).unwrap();
    if x <= n(0.04045) {
        x / n(12.92)
    } else {
        ((x + n(0.055)) / n(1.055)).powf(n(2.4))
    }
}
// Paired encode/decode samples, linearly interpolated in their input domain.
pub fn transfers() -> &'static [[f32; 2]] {
    static LUT: std::sync::OnceLock<Vec<[f32; 2]>> = std::sync::OnceLock::new();
    LUT.get_or_init(|| {
        (0..=4096)
            .map(|i| {
                let x = f64::from(i) / 4096.;
                [encode(x) as f32, decode(x) as f32]
            })
            .collect()
    })
}
fn transfer<F: Float>(x: F, channel: usize) -> F {
    let t = x.max(F::zero()).min(F::one()) * F::from(4096).unwrap();
    let i = t.to_usize().unwrap();
    let lut = transfers();
    let a = F::from(lut[i][channel]).unwrap();
    let b = F::from(lut[(i + 1).min(4096)][channel]).unwrap();
    a + (b - a) * (t - t.floor())
}
fn matrix<F: Float>(m: [[f64; 3]; 3], v: [F; 3]) -> [F; 3] {
    m.map(|row| (0..3).fold(F::zero(), |sum, i| sum + F::from(row[i]).unwrap() * v[i]))
}
fn lab(rgb: [f64; 3]) -> [f64; 3] {
    let lms = matrix(
        [
            [0.4122214708, 0.5363325363, 0.0514459929],
            [0.2119034982, 0.6806995451, 0.1073969566],
            [0.0883024619, 0.2817188376, 0.6299787005],
        ],
        rgb,
    );
    matrix(
        [
            [0.2104542553, 0.793617785, -0.0040720468],
            [1.9779984951, -2.428592205, 0.4505937099],
            [0.0259040371, 0.7827717662, -0.808675766],
        ],
        lms.map(f64::cbrt),
    )
}
fn rgb<F: Float>(v: [F; 3]) -> [F; 3] {
    let lms = matrix(
        [
            [1., 0.3963377774, 0.2158037573],
            [1., -0.1055613458, -0.0638541728],
            [1., -0.0894841775, -1.291485548],
        ],
        v,
    );
    matrix(
        [
            [4.0767416621, -3.3077115913, 0.2309699292],
            [-1.2684380046, 2.6097574011, -0.3413193965],
            [-0.0041960863, -0.7034186147, 1.707614701],
        ],
        lms.map(|x| x * x * x),
    )
    .map(|x| x.max(F::zero()).min(F::one()))
}
pub fn stop(at: f32, rgba: [f64; 4], space: u32) -> Stop {
    let [r, g, b, a] = rgba;
    let v = match space {
        1 => [r, g, b].map(encode),
        2 => lab([r, g, b]),
        _ => [r, g, b],
    };
    Stop {
        color: [
            (v[0] * a) as f32,
            (v[1] * a) as f32,
            (v[2] * a) as f32,
            a as f32,
        ],
        data: [at, 0., 0., 0.],
    }
}
pub fn ramp<F: Float>(t: F, stops: &[Stop], space: u32) -> [F; 4] {
    let f = |x: f32| F::from(x).unwrap();
    let mut a = stops[0];
    let mut c = a.color.map(f);
    for b in &stops[1..] {
        if t < f(b.data[0]) {
            if t > f(a.data[0]) {
                let u = (t - f(a.data[0])) / (f(b.data[0]) - f(a.data[0]));
                c = std::array::from_fn(|i| c[i] + u * (f(b.color[i]) - c[i]));
            }
            break;
        }
        a = *b;
        c = a.color.map(f);
    }
    let [r, g, b, alpha] = c;
    if alpha == F::zero() {
        return [F::zero(); 4];
    }
    let v = [r / alpha, g / alpha, b / alpha];
    let [r, g, b] = match space {
        1 => v.map(decode),
        2 => rgb(v),
        _ => v,
    };
    [r, g, b, alpha]
}
pub fn sample<F: Float>(p: &Params, stops: &[Stop], x: u32, y: u32, bg: f64) -> [u8; 4] {
    let n = |x: f64| F::from(x).unwrap();
    let pos = |i: u32| n(f64::from(i) + 0.5);
    let [ox, oy, ax, ay] = p.geom.map(|v| n(f64::from(v)));
    let dx = pos(x) - ox;
    let dy = pos(y) - oy;
    let mut t = match p.mode[0] {
        1 => {
            let inner = n(p.misc[0].into());
            (((dx / ax) * (dx / ax) + (dy / ay) * (dy / ay)).sqrt() - inner) / (F::one() - inner)
        }
        2 if dx == F::zero() && dy == F::zero() => F::zero(),
        2 => {
            let a = dy.atan2(dx) * n(180. / std::f64::consts::PI) - n(p.misc[1].into());
            (a - n(360.) * (a / n(360.)).floor()) / n(p.misc[2].into())
        }
        _ => (dx * ax + dy * ay) / (ax * ax + ay * ay),
    };
    let work = p.options[1];
    let bg_alpha = if bg < 0. { 0. } else { 1. };
    let bg = n(bg.max(0.));
    let byte = |v: F| {
        (v.max(F::zero()).min(F::one()) * n(255.))
            .round()
            .to_u8()
            .unwrap()
    };
    if p.mode[0] == 0 && p.mode[1] == 1 {
        let k = t.round();
        t = ((dx - k * ax) * ax + (dy - k * ay) * ay) / (ax * ax + ay * ay);
    }
    if work == 1 || work == 2 {
        t = F::one() - dy / ay;
    }
    t = match p.mode[1] {
        1 => t - t.floor(),
        2 => F::one() - (F::one() - (t - n(2.) * (t / n(2.)).floor())).abs(),
        _ => t.max(F::zero()).min(F::one()),
    };
    let mut c = ramp(t, stops, p.mode[2]);
    if p.options[0] != 0 && c[3] > F::zero() {
        let mut rank = 0;
        for bit in 0..3 {
            let v = (y >> bit) & 1;
            rank = 4 * rank + 2 * (((x >> bit) & 1) ^ v) + v;
        }
        for ch in &mut c[..3] {
            let table = p.options[0] == 2;
            let e = if table { transfer(*ch, 0) } else { encode(*ch) };
            let e = (e + n(((f64::from(rank) + 0.5) / 64. - 0.5) / 255.))
                .max(F::zero())
                .min(F::one());
            *ch = if table { transfer(e, 1) } else { decode(e) };
        }
    }
    let mut alpha = c[3];
    let mut src = [c[0] * alpha, c[1] * alpha, c[2] * alpha];
    if work == 3 {
        let border = (n(2.75) - pos(y)).max(F::zero()).min(F::one()) * n(0.6);
        alpha = border + alpha * (F::one() - border);
        src =
            std::array::from_fn(|i| border * n([0.1, 0.2, 0.5][i]) + (F::one() - border) * src[i]);
    }
    let [r, g, b] = src.map(|v| byte(encode(v + (F::one() - alpha) * bg)));
    [r, g, b, byte(alpha + (F::one() - alpha) * n(bg_alpha))]
}
