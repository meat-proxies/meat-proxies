//! A tiny shape rasterizer for the high-detail art: signed-distance shapes,
//! soft shading, colored outlines, and 4x4 supersampled anti-aliasing.

use std::ops::{Add, Mul, Sub};

use super::render::Rgb;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct V2(pub f32, pub f32);

impl Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        V2(self.0 + o.0, self.1 + o.1)
    }
}

impl Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        V2(self.0 - o.0, self.1 - o.1)
    }
}

impl Mul<f32> for V2 {
    type Output = V2;
    fn mul(self, k: f32) -> V2 {
        V2(self.0 * k, self.1 * k)
    }
}

impl V2 {
    pub fn dot(self, o: V2) -> f32 {
        self.0 * o.0 + self.1 * o.1
    }

    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// Unit vector at `angle` radians from straight down, turning toward +x.
    pub fn down_at(angle: f32) -> V2 {
        V2(angle.sin(), angle.cos())
    }
}

/// A shape in sprite units (one unit is one pixel of the small art).
#[derive(Clone, Debug)]
pub enum Shape {
    Circle(V2, f32),
    Ellipse(V2, f32, f32),
    /// A segment from `a` to `b` whose radius tapers from the first to the second.
    Capsule(V2, V2, f32, f32),
    Poly(Vec<V2>),
    RoundRect(V2, V2, f32),
    /// Everything on the side of the line through `point` that `normal` points away from.
    HalfPlane(V2, V2),
    Intersect(Box<Shape>, Box<Shape>),
    /// Union that blends the two shapes together over `k` units.
    Smooth(Box<Shape>, Box<Shape>, f32),
}

impl Shape {
    pub fn and(self, other: Shape) -> Shape {
        Shape::Intersect(Box::new(self), Box::new(other))
    }

    pub fn blend(self, other: Shape, k: f32) -> Shape {
        Shape::Smooth(Box::new(self), Box::new(other), k)
    }

    /// Signed distance in units: negative inside.
    pub fn sdf(&self, p: V2) -> f32 {
        match self {
            Shape::Circle(c, r) => (p - *c).len() - r,
            Shape::Ellipse(c, rx, ry) => {
                let q = p - *c;
                (V2(q.0 / rx, q.1 / ry).len() - 1.0) * rx.min(*ry)
            }
            Shape::Capsule(a, b, ra, rb) => {
                let ab = *b - *a;
                let t = ((p - *a).dot(ab) / ab.dot(ab).max(1e-6)).clamp(0.0, 1.0);
                (p - (*a + ab * t)).len() - (ra + (rb - ra) * t)
            }
            Shape::Poly(points) => poly_sdf(points, p),
            Shape::RoundRect(min, max, r) => {
                let half = (*max - *min) * 0.5;
                let q = p - (*min + half);
                let d = V2(q.0.abs() - half.0 + r, q.1.abs() - half.1 + r);
                V2(d.0.max(0.0), d.1.max(0.0)).len() + d.0.max(d.1).min(0.0) - r
            }
            Shape::HalfPlane(point, normal) => (p - *point).dot(*normal) / normal.len(),
            Shape::Intersect(a, b) => a.sdf(p).max(b.sdf(p)),
            Shape::Smooth(a, b, k) => {
                let (da, db) = (a.sdf(p), b.sdf(p));
                let h = (0.5 + 0.5 * (db - da) / k).clamp(0.0, 1.0);
                db + (da - db) * h - k * h * (1.0 - h)
            }
        }
    }

    /// A box (in units) that contains the shape; `None` means unbounded.
    fn bounds(&self) -> Option<(V2, V2)> {
        let grow = |c: V2, rx: f32, ry: f32| Some((V2(c.0 - rx, c.1 - ry), V2(c.0 + rx, c.1 + ry)));
        match self {
            Shape::Circle(c, r) => grow(*c, *r, *r),
            Shape::Ellipse(c, rx, ry) => grow(*c, *rx, *ry),
            Shape::Capsule(a, b, ra, rb) => {
                let r = ra.max(*rb);
                Some((
                    V2(a.0.min(b.0) - r, a.1.min(b.1) - r),
                    V2(a.0.max(b.0) + r, a.1.max(b.1) + r),
                ))
            }
            Shape::Poly(points) => points.iter().fold(None, |acc, &p| match acc {
                None => Some((p, p)),
                Some((lo, hi)) => Some((
                    V2(lo.0.min(p.0), lo.1.min(p.1)),
                    V2(hi.0.max(p.0), hi.1.max(p.1)),
                )),
            }),
            Shape::RoundRect(min, max, _) => Some((*min, *max)),
            Shape::HalfPlane(..) => None,
            Shape::Intersect(a, b) => match (a.bounds(), b.bounds()) {
                (Some(x), None) | (None, Some(x)) => Some(x),
                (Some((alo, ahi)), Some((blo, bhi))) => Some((
                    V2(alo.0.max(blo.0), alo.1.max(blo.1)),
                    V2(ahi.0.min(bhi.0), ahi.1.min(bhi.1)),
                )),
                (None, None) => None,
            },
            Shape::Smooth(a, b, k) => match (a.bounds(), b.bounds()) {
                (Some((alo, ahi)), Some((blo, bhi))) => Some((
                    V2(alo.0.min(blo.0) - k, alo.1.min(blo.1) - k),
                    V2(ahi.0.max(bhi.0) + k, ahi.1.max(bhi.1) + k),
                )),
                _ => None,
            },
        }
    }

    /// Outward surface direction, estimated from the distance field.
    fn normal(&self, p: V2) -> V2 {
        let e = 0.05;
        let n = V2(
            self.sdf(p + V2(e, 0.0)) - self.sdf(p - V2(e, 0.0)),
            self.sdf(p + V2(0.0, e)) - self.sdf(p - V2(0.0, e)),
        );
        let len = n.len();
        if len > 1e-6 {
            n * (1.0 / len)
        } else {
            V2(0.0, -1.0)
        }
    }
}

fn poly_sdf(points: &[V2], p: V2) -> f32 {
    let Some(&first) = points.first() else {
        return f32::MAX;
    };
    let mut d = (p - first).dot(p - first);
    let mut sign = 1.0;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let (vi, vj) = (points[i], points[j]);
        let e = vj - vi;
        let w = p - vi;
        let b = w - e * (w.dot(e) / e.dot(e).max(1e-6)).clamp(0.0, 1.0);
        d = d.min(b.dot(b));
        let c = (p.1 >= vi.1, p.1 < vj.1, e.0 * w.1 > e.1 * w.0);
        if (c.0 && c.1 && c.2) || (!c.0 && !c.1 && !c.2) {
            sign = -sign;
        }
        j = i;
    }
    sign * d.sqrt()
}

/// How a shape is filled.
#[derive(Clone, Copy, Debug)]
pub enum Paint {
    /// Soft three-tone shading lit from the upper right, with a colored outline.
    Shaded {
        base: Rgb,
        light: Rgb,
        dark: Rgb,
        outline: bool,
    },
    /// One color at the given opacity, with no outline.
    Flat(Rgb, f32),
    /// A soft halo that fades out over `radius` units beyond the shape.
    Glow(Rgb, f32, f32),
}

impl Paint {
    pub fn shaded(base: Rgb, light: Rgb, dark: Rgb) -> Paint {
        Paint::Shaded {
            base,
            light,
            dark,
            outline: true,
        }
    }

    pub fn no_outline(self) -> Paint {
        match self {
            Paint::Shaded {
                base, light, dark, ..
            } => Paint::Shaded {
                base,
                light,
                dark,
                outline: false,
            },
            other => other,
        }
    }
}

/// Deep plum the outlines lean toward, so they read as colored "selout" lines
/// rather than flat black.
const INK: Rgb = Rgb(44, 22, 34);
/// Cool bounce light from the sky behind the figure.
const RIM: Rgb = Rgb(176, 200, 255);
/// Tone steps on each side of the base color.
const BANDS: f32 = 3.0;
const LIGHT_DIR: V2 = V2(0.45, -0.89);
/// Anti-aliasing samples per pixel side; fewer are needed once pixels are small.
fn supersample(scale: usize) -> usize {
    if scale >= 4 { 3 } else { 4 }
}

/// Straight-alpha RGBA pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Bitmap {
    pub w: usize,
    pub h: usize,
    pub px: Vec<[u8; 4]>,
}

impl Bitmap {
    pub fn get(&self, x: usize, y: usize) -> [u8; 4] {
        self.px[y * self.w + x]
    }
}

pub struct Painter {
    scale: usize,
    ss: usize,
    w: usize,
    h: usize,
    /// Premultiplied RGBA at supersampled resolution.
    acc: Vec<[f32; 4]>,
}

impl Painter {
    /// A canvas `w` x `h` units big, drawn at `scale` pixels per unit.
    pub fn new(w: usize, h: usize, scale: usize) -> Self {
        let ss = supersample(scale);
        let (sw, sh) = (w * scale * ss, h * scale * ss);
        Self {
            scale,
            ss,
            w: sw,
            h: sh,
            acc: vec![[0.0; 4]; sw * sh],
        }
    }

    pub fn draw(&mut self, shape: &Shape, paint: Paint) {
        let units_per_sample = 1.0 / (self.scale * self.ss) as f32;
        // One output pixel wide, whatever the scale.
        let outline = 1.0 / self.scale as f32;
        let reach = match paint {
            Paint::Glow(_, radius, _) => radius,
            Paint::Shaded { outline: true, .. } => outline,
            _ => 0.0,
        };
        let (lo, hi) = shape.bounds().unwrap_or((V2(-1e3, -1e3), V2(1e3, 1e3)));
        let to_sample = |u: f32| u / units_per_sample;
        let x0 = to_sample(lo.0 - reach).floor().max(0.0) as usize;
        let y0 = to_sample(lo.1 - reach).floor().max(0.0) as usize;
        let x1 = (to_sample(hi.0 + reach).ceil().max(0.0) as usize).min(self.w);
        let y1 = (to_sample(hi.1 + reach).ceil().max(0.0) as usize).min(self.h);
        for sy in y0..y1 {
            for sx in x0..x1 {
                let p = V2(
                    (sx as f32 + 0.5) * units_per_sample,
                    (sy as f32 + 0.5) * units_per_sample,
                );
                let d = shape.sdf(p);
                let Some((color, alpha)) = shade(shape, paint, p, d, outline) else {
                    continue;
                };
                let dst = &mut self.acc[sy * self.w + sx];
                for (i, channel) in [color.0, color.1, color.2].into_iter().enumerate() {
                    dst[i] = f32::from(channel) / 255.0 * alpha + dst[i] * (1.0 - alpha);
                }
                dst[3] = alpha + dst[3] * (1.0 - alpha);
            }
        }
    }

    /// Averages each 4x4 block of samples into one anti-aliased pixel.
    pub fn finish(self) -> Bitmap {
        let ss = self.ss;
        let (w, h) = (self.w / ss, self.h / ss);
        let mut px = Vec::with_capacity(w * h);
        let n = (ss * ss) as f32;
        for y in 0..h {
            for x in 0..w {
                let mut sum = [0.0f32; 4];
                for sy in 0..ss {
                    for sx in 0..ss {
                        let s = self.acc[(y * ss + sy) * self.w + x * ss + sx];
                        for i in 0..4 {
                            sum[i] += s[i];
                        }
                    }
                }
                let a = sum[3] / n;
                let un = |c: f32| {
                    if a > 0.0 {
                        ((c / n) / a * 255.0).round().clamp(0.0, 255.0) as u8
                    } else {
                        0
                    }
                };
                px.push([
                    un(sum[0]),
                    un(sum[1]),
                    un(sum[2]),
                    (a * 255.0).round() as u8,
                ]);
            }
        }
        Bitmap { w, h, px }
    }
}

fn shade(shape: &Shape, paint: Paint, p: V2, d: f32, outline: f32) -> Option<(Rgb, f32)> {
    match paint {
        Paint::Flat(color, alpha) => (d < 0.0).then_some((color, alpha)),
        Paint::Glow(color, radius, alpha) => {
            (d < radius).then(|| (color, alpha * (1.0 - d.max(0.0) / radius).powi(2)))
        }
        Paint::Shaded {
            base,
            light,
            dark,
            outline: has_outline,
        } => {
            let n = shape.normal(p);
            let facing = n.dot(LIGHT_DIR);
            if d >= 0.0 {
                // Selective outlines: softer on the lit side, deep on the shadow side.
                let ink = dark.mix(INK, 0.45 + 0.4 * (-facing).max(0.0));
                return (has_outline && d < outline).then_some((ink, 1.0));
            }
            // Shade strongly near the edge and stay flat inside, then snap to
            // a few clean tone bands the way hand-shaded sprites are lit.
            let edge = 1.0 - (-d / 0.7).clamp(0.0, 1.0);
            let lit = facing * (0.3 + 0.7 * edge);
            let lit = (lit * BANDS).round() / BANDS;
            let mut color = if lit >= 0.0 {
                base.mix(light, lit)
            } else {
                base.mix(dark, -lit)
            };
            // A cool rim light on edges facing away from the key light.
            let back = (-facing - 0.35).max(0.0);
            if back > 0.0 && -d < 0.25 {
                color = color.mix(RIM, (back * 0.9).min(0.4));
            }
            Some((color, 1.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_are_negative_inside_and_positive_outside() {
        let shapes = [
            Shape::Circle(V2(5.0, 5.0), 2.0),
            Shape::Ellipse(V2(5.0, 5.0), 3.0, 1.5),
            Shape::Capsule(V2(3.0, 5.0), V2(7.0, 5.0), 1.0, 0.5),
            Shape::Poly(vec![V2(3.0, 3.0), V2(7.0, 3.0), V2(5.0, 7.0)]),
            Shape::RoundRect(V2(3.0, 3.0), V2(7.0, 7.0), 1.0),
        ];
        for shape in &shapes {
            assert!(
                shape.sdf(V2(5.0, 4.9)) < 0.0,
                "{shape:?} should contain the center"
            );
            assert!(
                shape.sdf(V2(20.0, 20.0)) > 0.0,
                "{shape:?} should not contain a far point"
            );
        }
        let ring =
            Shape::Circle(V2(5.0, 5.0), 3.0).and(Shape::HalfPlane(V2(5.0, 5.0), V2(0.0, -1.0)));
        assert!(ring.sdf(V2(5.0, 3.0)) > 0.0, "upper half is cut away");
        assert!(ring.sdf(V2(5.0, 7.0)) < 0.0);
    }

    #[test]
    fn edges_are_anti_aliased() {
        let mut painter = Painter::new(8, 8, 2);
        painter.draw(
            &Shape::Circle(V2(4.0, 4.0), 3.0),
            Paint::Flat(Rgb(200, 100, 50), 1.0),
        );
        let bitmap = painter.finish();
        assert_eq!((bitmap.w, bitmap.h), (16, 16));
        assert_eq!(bitmap.get(8, 8), [200, 100, 50, 255]);
        assert_eq!(bitmap.get(0, 0)[3], 0);
        assert!(
            bitmap.px.iter().any(|p| p[3] > 20 && p[3] < 235),
            "edge pixels should be partly covered"
        );
    }

    #[test]
    fn shading_lights_the_upper_right() {
        let mut painter = Painter::new(10, 10, 2);
        let paint = Paint::shaded(Rgb(128, 128, 128), Rgb(255, 255, 255), Rgb(0, 0, 0));
        painter.draw(&Shape::Circle(V2(5.0, 5.0), 4.0), paint);
        let bitmap = painter.finish();
        let lit = bitmap.get(13, 5)[0];
        let shadow = bitmap.get(6, 15)[0];
        assert!(lit > 150 && shadow < 100, "lit {lit}, shadow {shadow}");
    }
}
