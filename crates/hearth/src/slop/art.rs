//! Baked sprite sets. Scale 1 uses the hand-drawn small sprites; larger scales
//! paint high-detail art from shapes, at exactly `scale` times the small size
//! so hitboxes line up whichever set is on screen.

use std::f32::consts::{PI, TAU};

use super::game::{Game, Pose, SlopKind};
use super::raster::{Bitmap, Paint, Painter, Shape, V2};
use super::render::Rgb;
use super::sprites::{self, Sprite};

/// Distance the runner covers per full stride, in world units.
const STRIDE: f32 = 18.0;
const RUN_FRAMES: usize = 8;

pub struct ArtSet {
    pub scale: usize,
    run: Vec<Bitmap>,
    jump: Bitmap,
    duck: Vec<Bitmap>,
    slopped: Bitmap,
    puddle: [Bitmap; 2],
    hand: Bitmap,
    tower: Bitmap,
    dashes: [Bitmap; 2],
    spark: [Bitmap; 2],
}

impl ArtSet {
    pub fn new(scale: usize) -> Self {
        if scale <= 1 {
            Self::small()
        } else {
            Self::detailed(scale)
        }
    }

    fn small() -> Self {
        let b = sprite_bitmap;
        Self {
            scale: 1,
            run: vec![b(sprites::RUN_A), b(sprites::RUN_B)],
            jump: b(sprites::JUMP),
            duck: vec![b(sprites::DUCK)],
            slopped: b(sprites::SLOPPED),
            puddle: [b(sprites::PUDDLE_A), b(sprites::PUDDLE_B)],
            hand: b(sprites::HAND),
            tower: b(sprites::TOWER),
            dashes: [b(sprites::DASHES_A), b(sprites::DASHES_B)],
            spark: [b(sprites::SPARK_A), b(sprites::SPARK_B)],
        }
    }

    fn detailed(scale: usize) -> Self {
        let run = (0..RUN_FRAMES)
            .map(|i| girl(Pose::Run, i as f32 / RUN_FRAMES as f32, scale))
            .collect();
        let duck = (0..4)
            .map(|i| girl(Pose::Duck, i as f32 / 4.0, scale))
            .collect();
        Self {
            scale,
            run,
            jump: girl(Pose::Jump, 0.0, scale),
            duck,
            slopped: girl(Pose::Slopped, 0.0, scale),
            puddle: [puddle(scale, 0.0), puddle(scale, 1.0)],
            hand: hand(scale),
            tower: tower(scale),
            dashes: [dashes(scale, 0.0), dashes(scale, 1.0)],
            spark: [spark(scale, 1.0), spark(scale, 0.6)],
        }
    }

    pub fn runner(&self, game: &Game) -> &Bitmap {
        let pick = |frames: &[Bitmap]| {
            let step = STRIDE / frames.len() as f32;
            (game.distance / step) as usize % frames.len()
        };
        match game.pose() {
            Pose::Run => &self.run[pick(&self.run)],
            Pose::Duck => &self.duck[pick(&self.duck)],
            Pose::Jump => &self.jump,
            Pose::Slopped => &self.slopped,
        }
    }

    pub fn obstacle(&self, kind: SlopKind, frame: u32) -> &Bitmap {
        let i = (frame % 2) as usize;
        match kind {
            SlopKind::Puddle => &self.puddle[i],
            SlopKind::Hand => &self.hand,
            SlopKind::Tower => &self.tower,
            SlopKind::Dashes => &self.dashes[i],
        }
    }

    pub fn spark(&self, ticks: u32) -> &Bitmap {
        &self.spark[((ticks / 8) % 2) as usize]
    }
}

fn sprite_bitmap(sprite: Sprite) -> Bitmap {
    let (w, h) = (sprites::width(sprite), sprites::height(sprite));
    let px = sprite
        .iter()
        .flat_map(|row| row.bytes())
        .map(|key| sprites::color(key).map_or([0; 4], |c| [c.0, c.1, c.2, 255]))
        .collect();
    Bitmap { w, h, px }
}

// Claude.ai-inspired accents: terracotta, warm cream, charcoal.
const TERRA: Rgb = Rgb(217, 119, 87);
const TERRA_LIGHT: Rgb = Rgb(244, 168, 132);
const TERRA_DARK: Rgb = Rgb(160, 72, 50);
const CREAM: Rgb = Rgb(244, 240, 230);
const CREAM_LIGHT: Rgb = Rgb(255, 254, 250);
const CREAM_DARK: Rgb = Rgb(206, 194, 174);
const CHARCOAL: Rgb = Rgb(62, 58, 56);
const CHARCOAL_LIGHT: Rgb = Rgb(104, 98, 96);
const CHARCOAL_DARK: Rgb = Rgb(32, 30, 32);
const HAIR: Rgb = Rgb(92, 54, 42);
const HAIR_LIGHT: Rgb = Rgb(160, 102, 74);
const HAIR_DARK: Rgb = Rgb(54, 30, 28);
const SKIN: Rgb = Rgb(252, 216, 192);
const SKIN_LIGHT: Rgb = Rgb(255, 236, 222);
const SKIN_DARK: Rgb = Rgb(226, 166, 142);
const EYE: Rgb = Rgb(50, 36, 58);
const IRIS: Rgb = Rgb(150, 84, 62);
const BLUSH: Rgb = Rgb(242, 140, 128);
const WHITE: Rgb = Rgb(255, 255, 255);

fn hair() -> Paint {
    Paint::shaded(HAIR, HAIR_LIGHT, HAIR_DARK)
}
fn skin() -> Paint {
    Paint::shaded(SKIN, SKIN_LIGHT, SKIN_DARK)
}
fn cream() -> Paint {
    Paint::shaded(CREAM, CREAM_LIGHT, CREAM_DARK)
}
fn terra() -> Paint {
    Paint::shaded(TERRA, TERRA_LIGHT, TERRA_DARK)
}
fn charcoal() -> Paint {
    Paint::shaded(CHARCOAL, CHARCOAL_LIGHT, CHARCOAL_DARK)
}

/// Joint angles for one frame. Leg and arm angles are radians from straight
/// down, positive toward the front (+x); bends fold the lower limb backward.
struct Rig {
    lean: f32,
    legs: [(f32, f32); 2],
    arms: [(f32, f32); 2],
    /// How far the twin tails stream back and up (0 hanging, 1 flying).
    tail_lift: f32,
    tail_sway: f32,
    hem_flare: f32,
}

const THIGH: f32 = 2.5;
const SHIN: f32 = 2.4;
const FOOT_R: f32 = 0.62;

fn rig(pose: Pose, phase: f32) -> Rig {
    let t = phase * TAU;
    match pose {
        Pose::Run => {
            let leg = |o: f32| {
                let s = t + o;
                (0.78 * s.sin(), 0.25 + 1.15 * s.cos().max(0.0))
            };
            let arm = |o: f32| (-0.75 * (t + o).sin(), -1.2);
            Rig {
                lean: 0.14,
                legs: [leg(PI), leg(0.0)],
                arms: [arm(0.0), arm(PI)],
                tail_lift: 0.45,
                tail_sway: (2.0 * t).sin(),
                hem_flare: 0.5 + 0.3 * (2.0 * t).cos(),
            }
        }
        Pose::Jump => Rig {
            lean: 0.05,
            legs: [(-0.5, 1.7), (1.0, 1.8)],
            arms: [(-2.2, -0.4), (1.3, 0.5)],
            tail_lift: 1.0,
            tail_sway: 0.3,
            hem_flare: 1.0,
        },
        Pose::Duck => {
            let s = (t).sin() * 0.25;
            Rig {
                lean: 0.8,
                legs: [(0.6 - s, 2.1), (1.1 + s, 2.3)],
                arms: [(1.2 + s, -0.9), (1.6 - s, -0.9)],
                tail_lift: 1.0,
                tail_sway: (2.0 * t).sin() * 0.5,
                hem_flare: 0.8,
            }
        }
        Pose::Slopped => Rig {
            lean: 0.0,
            legs: [(-0.15, 0.1), (0.2, 0.1)],
            arms: [(-0.5, -0.2), (0.6, -0.2)],
            tail_lift: 0.0,
            tail_sway: 0.0,
            hem_flare: 0.0,
        },
    }
}

/// Paints the runner. Sizes match the small sprites so hitboxes agree.
fn girl(pose: Pose, phase: f32, scale: usize) -> Bitmap {
    let sprite = match pose {
        Pose::Run => sprites::RUN_A,
        Pose::Jump => sprites::JUMP,
        Pose::Duck => sprites::DUCK,
        Pose::Slopped => sprites::SLOPPED,
    };
    let (w, h) = (sprites::width(sprite), sprites::height(sprite));
    let r = rig(pose, phase);
    let up = V2(r.lean.sin(), -r.lean.cos());
    let fwd = V2(r.lean.cos(), r.lean.sin());

    // Place the hip so the lowest foot rests on the sprite's bottom edge.
    let foot = |(a, bend): (f32, f32)| {
        let knee = V2::down_at(a) * THIGH;
        (knee, knee + V2::down_at(a - bend) * SHIN)
    };
    let lowest = r
        .legs
        .iter()
        .map(|&leg| foot(leg).1.1)
        .fold(f32::MIN, f32::max);
    let hip = V2(6.6, h as f32 - FOOT_R - 0.15 - lowest);

    let waist = hip + up * 1.7;
    let chest = hip + up * 3.1;
    let neck = hip + up * 4.1;
    let head = hip + up * 6.7 + fwd * 0.35;
    let shoulder = chest + up * 0.2 + fwd * 0.2;

    let mut p = Painter::new(w, h, scale);

    // Twin tails, far one first. Their ribbons go on after the head.
    let mut ribbons = Vec::with_capacity(2);
    let tail = |side: f32| {
        // The far tail is tied high on the crown and streams back; the near
        // one is tied behind the ear and hangs lower, so both read clearly.
        let (root, path, lift) = if side < 0.0 {
            (
                head + V2(-0.9, -2.9),
                [(-1.7, 1.1), (-3.2, 3.2), (-3.8, 5.8)],
                r.tail_lift * 0.85,
            )
        } else {
            (
                head + V2(-2.4, -0.9),
                [(-1.0, 2.0), (-1.4, 4.8), (-1.0, 7.6)],
                r.tail_lift,
            )
        };
        let sway = r.tail_sway * (0.4 + side * 0.1);
        let pts = [
            root,
            root + V2(path[0].0 - lift * 1.0, path[0].1 - lift * 1.1 + sway * 0.3),
            root + V2(path[1].0 - lift * 2.6, path[1].1 - lift * 2.6 + sway),
            root + V2(path[2].0 - lift * 4.2, path[2].1 - lift * 4.0 + sway * 1.6),
        ];
        (root, pts)
    };
    for side in [-1.0, 1.0] {
        let (root, pts) = tail(side);
        let radii = [0.85, 1.1, 0.85, 0.25];
        let paint = if side < 0.0 {
            Paint::shaded(HAIR_DARK.mix(HAIR, 0.5), HAIR, HAIR_DARK)
        } else {
            hair()
        };
        for i in 0..3 {
            p.draw(
                &Shape::Capsule(pts[i], pts[i + 1], radii[i], radii[i + 1]),
                paint,
            );
        }
        // Highlight streak down the tail.
        p.draw(
            &Shape::Capsule(pts[1] + V2(0.3, -0.2), pts[2] + V2(0.25, -0.1), 0.22, 0.12),
            Paint::Flat(HAIR_LIGHT, 0.6),
        );
        ribbons.push(root + V2(0.15, 0.05));
    }

    // Far arm and both legs sit behind the dress.
    limb(
        &mut p,
        shoulder - fwd * 0.3,
        r.arms[0],
        1.55,
        1.45,
        skin(),
        0.42,
    );
    for &leg in &r.legs {
        let (knee, ankle) = foot(leg);
        let (knee, ankle) = (hip + knee, hip + ankle);
        p.draw(&Shape::Capsule(hip, knee, 0.68, 0.55), charcoal());
        p.draw(&Shape::Capsule(knee, ankle, 0.55, 0.5), charcoal());
        // Boot with a terracotta sole.
        let toe = ankle + V2(0.85, 0.15);
        p.draw(
            &Shape::Capsule(ankle + V2(-0.1, -0.35), toe, 0.62, 0.5),
            Paint::shaded(CHARCOAL_DARK, CHARCOAL_LIGHT, CHARCOAL_DARK),
        );
        p.draw(
            &Shape::Capsule(ankle + V2(-0.3, 0.42), toe + V2(0.2, 0.42), 0.18, 0.18),
            Paint::Flat(TERRA, 1.0),
        );
    }

    // Dress: fitted bodice, flared skirt, terracotta sash and hem.
    let hem_y = 1.4 + 0.2 * r.hem_flare;
    let back = -(2.5 + 0.9 * r.hem_flare);
    let front = 2.3 + 0.3 * r.hem_flare;
    let down = up * -1.0;
    // The skirt mostly hangs with gravity rather than tilting with the body.
    let skirt_lean = r.lean * 0.25;
    let sfwd = V2(skirt_lean.cos(), skirt_lean.sin());
    let sdown = V2(-skirt_lean.sin(), skirt_lean.cos());
    let dress = Shape::Poly(vec![
        neck + fwd * -1.05 + down * 0.1,
        neck + fwd * 1.05 + down * 0.1,
        waist + fwd * 1.0,
        hip + sfwd * 1.6 + sdown * (hem_y * 0.6),
        hip + sfwd * front + sdown * hem_y,
        hip + sfwd * 0.2 + sdown * (hem_y + 0.25),
        hip + sfwd * back + sdown * (hem_y - 0.15),
        hip + sfwd * -1.5 + sdown * 0.2,
        waist + fwd * -1.0,
    ]);
    p.draw(&dress, cream());
    // Inside a band means below its top edge and above its bottom edge.
    let band = |top: V2, bottom: V2| Shape::HalfPlane(top, up).and(Shape::HalfPlane(bottom, down));
    p.draw(
        &dress.clone().and(band(waist + up * 0.5, waist - up * 0.5)),
        terra().no_outline(),
    );
    let hem_line = hip + sdown * (hem_y - 0.85);
    let sup = sdown * -1.0;
    p.draw(
        &dress.clone().and(Shape::HalfPlane(hem_line, sup)),
        terra().no_outline(),
    );
    let trim =
        Shape::HalfPlane(hem_line, sup).and(Shape::HalfPlane(hem_line + sdown * 0.18, sdown));
    p.draw(&dress.clone().and(trim), Paint::Flat(CREAM_LIGHT, 0.9));
    // Pleat lines and the sash bow.
    for k in [-1.0, 0.4] {
        let a = waist + down * 0.5 + fwd * k;
        p.draw(
            &Shape::Capsule(a, a + down * 2.0 + fwd * (k * 0.6), 0.09, 0.09),
            Paint::Flat(CREAM_DARK, 0.8),
        );
    }
    let bow = waist + fwd * -1.1;
    p.draw(
        &Shape::Poly(vec![bow, bow + V2(-1.2, -0.6), bow + V2(-1.1, 0.7)]),
        terra(),
    );
    p.draw(
        &Shape::Circle(bow, 0.35),
        Paint::shaded(TERRA_DARK, TERRA, TERRA_DARK),
    );
    sleeve(&mut p, shoulder - fwd * 0.3, r.arms[0].0);

    // Neck and head.
    p.draw(
        &Shape::Capsule(neck + down * 0.2, neck + up * 1.0, 0.55, 0.55),
        skin(),
    );
    p.draw(&Shape::Circle(head + V2(-0.35, -0.2), 3.35), hair());
    // Only the front of the face shows; the back of the head is hair.
    let face = Shape::Circle(head + V2(0.35, 0.3), 2.95)
        .and(Shape::HalfPlane(head + V2(0.0, -1.0), V2(0.0, -1.0)))
        .and(Shape::HalfPlane(head + V2(-1.1, 0.0), V2(-1.0, 0.0)));
    p.draw(&face, skin().no_outline());
    // Bangs sweep forward over the brow.
    for (a, b, ra) in [
        (V2(-0.4, -2.9), V2(1.3, -0.4), 0.75),
        (V2(0.8, -2.9), V2(2.4, -0.5), 0.6),
        (V2(1.8, -2.4), V2(3.0, -0.6), 0.45),
        (V2(-1.0, -1.0), V2(-0.6, 2.2), 0.7),
    ] {
        p.draw(
            &Shape::Capsule(head + a, head + b, ra, 0.18),
            hair().no_outline(),
        );
    }
    // A glossy highlight arcing across the crown.
    for (a, b) in [
        (V2(-2.4, -1.5), V2(-1.1, -2.6)),
        (V2(-0.7, -2.85), V2(0.6, -2.8)),
    ] {
        p.draw(
            &Shape::Capsule(head + a, head + b, 0.26, 0.2),
            Paint::Flat(HAIR_LIGHT, 0.75),
        );
    }
    for &knot in &ribbons {
        ribbon(&mut p, knot);
    }

    if pose == Pose::Slopped {
        // Squeezed-shut eyes and a wobbly frown.
        for (x, s) in [(1.75, 1.0), (0.1, 0.85)] {
            let e = head + V2(x, 0.55);
            p.draw(
                &Shape::Capsule(e + V2(-0.4 * s, -0.3), e + V2(0.3 * s, 0.0), 0.13, 0.13),
                Paint::Flat(EYE, 1.0),
            );
            p.draw(
                &Shape::Capsule(e + V2(0.3 * s, 0.0), e + V2(-0.4 * s, 0.3), 0.13, 0.13),
                Paint::Flat(EYE, 1.0),
            );
        }
        p.draw(
            &Shape::Capsule(head + V2(1.9, 1.9), head + V2(2.6, 1.75), 0.12, 0.12),
            Paint::Flat(TERRA_DARK, 1.0),
        );
    } else {
        eye(&mut p, head + V2(1.75, 0.45), 1.0);
        eye(&mut p, head + V2(0.1, 0.5), 0.8);
        p.draw(
            &Shape::Capsule(head + V2(2.35, 1.85), head + V2(2.7, 1.75), 0.12, 0.1),
            Paint::Flat(TERRA_DARK, 1.0),
        );
    }
    p.draw(
        &Shape::Ellipse(head + V2(1.7, 1.55), 0.6, 0.28),
        Paint::Flat(BLUSH, 0.55),
    );

    // Near arm on top.
    limb(&mut p, shoulder, r.arms[1], 1.55, 1.45, skin(), 0.45);
    sleeve(&mut p, shoulder + fwd * 0.1, r.arms[1].0);

    if pose == Pose::Slopped {
        let goo = Paint::shaded(Rgb(186, 172, 134), Rgb(230, 220, 186), Rgb(124, 112, 84));
        let top = head + V2(-0.2, -3.2);
        let blob =
            Shape::Ellipse(top, 2.6, 1.0).blend(Shape::Circle(top + V2(0.8, -0.3), 0.9), 0.5);
        p.draw(&blob, goo);
        for (x, len) in [(-1.6, 2.2), (0.4, 3.0), (1.7, 1.5)] {
            let a = top + V2(x, 0.4);
            p.draw(
                &Shape::Capsule(a, a + V2(0.1, len), 0.4, 0.3),
                goo.no_outline(),
            );
        }
        p.draw(&Shape::Ellipse(waist + fwd * 0.4, 1.0, 0.6), goo);
        p.draw(
            &Shape::Circle(top + V2(-0.9, -0.2), 0.3),
            Paint::Flat(CREAM_LIGHT, 0.8),
        );
    }
    p.finish()
}

fn eye(p: &mut Painter, c: V2, s: f32) {
    p.draw(&Shape::Ellipse(c, 0.5 * s, 0.82 * s), Paint::Flat(EYE, 1.0));
    p.draw(
        &Shape::Ellipse(c + V2(0.0, 0.3 * s), 0.36 * s, 0.4 * s),
        Paint::Flat(IRIS, 1.0),
    );
    p.draw(
        &Shape::Circle(c + V2(0.12 * s, -0.35 * s), 0.2 * s),
        Paint::Flat(WHITE, 1.0),
    );
    // Lashes flick out toward the back of the head.
    p.draw(
        &Shape::Capsule(
            c + V2(0.55 * s, -0.75 * s),
            c + V2(-0.7 * s, -0.7 * s),
            0.16,
            0.12,
        ),
        Paint::Flat(EYE, 1.0),
    );
    p.draw(
        &Shape::Capsule(
            c + V2(-0.55 * s, -0.7 * s),
            c + V2(-0.95 * s, -0.4 * s),
            0.12,
            0.06,
        ),
        Paint::Flat(EYE, 1.0),
    );
}

/// A short puffed sleeve with a terracotta cuff, pointing along the arm.
fn sleeve(p: &mut Painter, shoulder: V2, arm_angle: f32) {
    let dir = V2::down_at(arm_angle);
    let across = V2(dir.1, -dir.0);
    p.draw(
        &Shape::Capsule(shoulder - dir * 0.15, shoulder + dir * 0.45, 0.72, 0.62),
        cream(),
    );
    let cuff = shoulder + dir * 0.9;
    p.draw(
        &Shape::Capsule(cuff - across * 0.5, cuff + across * 0.5, 0.2, 0.2),
        Paint::Flat(TERRA, 1.0),
    );
}

fn ribbon(p: &mut Painter, c: V2) {
    p.draw(
        &Shape::Poly(vec![c, c + V2(-1.4, -1.2), c + V2(-1.5, 0.6)]),
        terra(),
    );
    p.draw(
        &Shape::Poly(vec![c, c + V2(1.2, -1.2), c + V2(1.35, 0.5)]),
        terra(),
    );
    p.draw(
        &Shape::Circle(c, 0.38),
        Paint::shaded(TERRA_DARK, TERRA, TERRA_DARK),
    );
}

fn limb(
    p: &mut Painter,
    root: V2,
    (a, bend): (f32, f32),
    upper: f32,
    lower: f32,
    paint: Paint,
    r: f32,
) {
    let joint = root + V2::down_at(a) * upper;
    let end = joint + V2::down_at(a - bend) * lower;
    p.draw(&Shape::Capsule(root, joint, r, r * 0.9), paint);
    p.draw(&Shape::Capsule(joint, end, r * 0.9, r), paint);
}

fn size(sprite: Sprite) -> (usize, usize) {
    (sprites::width(sprite), sprites::height(sprite))
}

/// Lukewarm filler content, pooling and wobbling.
fn puddle(scale: usize, wobble: f32) -> Bitmap {
    let (w, h) = size(sprites::PUDDLE_A);
    let mut p = Painter::new(w, h, scale);
    let wb = wobble * 0.35;
    let goo = Shape::Ellipse(V2(10.0, 5.4), 9.4, 1.5)
        .blend(
            Shape::Ellipse(V2(8.0, 3.6 + wb * 0.5), 5.0 - wb, 2.3 + wb * 0.6),
            1.2,
        )
        .blend(Shape::Ellipse(V2(14.2, 4.3), 2.8 + wb, 1.4), 1.0);
    p.draw(
        &goo,
        Paint::shaded(Rgb(184, 168, 130), Rgb(232, 222, 188), Rgb(118, 104, 78)),
    );
    p.draw(
        &Shape::Ellipse(V2(6.2, 2.7 + wb * 0.4), 1.6, 0.5),
        Paint::Flat(WHITE, 0.55),
    );
    p.draw(&Shape::Circle(V2(11.8, 3.9), 0.35), Paint::Flat(WHITE, 0.6));
    for (x, y) in [(4.0, 5.3), (12.5, 5.6), (16.0, 4.8)] {
        p.draw(
            &Shape::Circle(V2(x, y), 0.45),
            Paint::shaded(Rgb(122, 168, 92), Rgb(170, 210, 130), Rgb(80, 120, 60)).no_outline(),
        );
    }
    // Vacant little eyes.
    for x in [7.6, 9.4] {
        p.draw(
            &Shape::Ellipse(V2(x, 3.8 + wb * 0.3), 0.32, 0.45),
            Paint::Flat(Rgb(64, 54, 40), 1.0),
        );
    }
    // A bubble of generic text drifting off.
    let by = 0.3 - wobble * 0.25;
    p.draw(
        &Shape::RoundRect(V2(15.3, by + 0.2), V2(19.6, by + 2.3), 0.6),
        Paint::shaded(CREAM, CREAM_LIGHT, CREAM_DARK),
    );
    for (i, len) in [3.0, 2.2].into_iter().enumerate() {
        let y = by + 0.85 + i as f32 * 0.75;
        p.draw(
            &Shape::Capsule(V2(16.0, y), V2(16.0 + len, y), 0.16, 0.16),
            Paint::Flat(Rgb(150, 140, 120), 1.0),
        );
    }
    p.finish()
}

/// Six fingers and an unsettling sheen.
fn hand(scale: usize) -> Bitmap {
    let (w, h) = size(sprites::HAND);
    let mut p = Painter::new(w, h, scale);
    let flesh = Paint::shaded(Rgb(238, 190, 168), Rgb(255, 226, 210), Rgb(190, 128, 112));
    let tips = [5.4, 2.6, 1.3, 1.2, 2.5, 4.4];
    for (i, tip) in tips.into_iter().enumerate() {
        let x = 1.9 + i as f32 * 1.98;
        p.draw(
            &Shape::Capsule(V2(x, tip), V2(x + 0.1, 10.5), 0.86, 0.95),
            flesh,
        );
        p.draw(
            &Shape::RoundRect(V2(x - 0.5, tip - 0.5), V2(x + 0.5, tip + 0.6), 0.4),
            Paint::shaded(Rgb(250, 220, 214), WHITE, Rgb(214, 170, 160)),
        );
        for y in [tip + 2.4, tip + 4.0] {
            p.draw(
                &Shape::Capsule(V2(x - 0.4, y), V2(x + 0.4, y), 0.08, 0.08),
                Paint::Flat(Rgb(196, 134, 118), 0.8),
            );
        }
    }
    let palm = Shape::RoundRect(V2(0.9, 9.0), V2(13.1, 14.6), 2.0)
        .blend(Shape::RoundRect(V2(3.2, 12.0), V2(10.8, 18.0), 1.0), 1.0);
    p.draw(&palm, flesh);
    p.draw(
        &Shape::Ellipse(V2(9.0, 10.8), 2.0, 0.7),
        Paint::Flat(WHITE, 0.35),
    );
    p.draw(
        &Shape::RoundRect(V2(3.0, 16.6), V2(11.0, 18.0), 0.3),
        Paint::Flat(Rgb(120, 92, 84), 0.5),
    );
    p.finish()
}

/// Three stacked listicles. Number seven will shock you.
fn tower(scale: usize) -> Bitmap {
    let (w, h) = size(sprites::TOWER);
    let mut p = Painter::new(w, h, scale);
    let boxes = [
        (
            3.0,
            7.2,
            (Rgb(222, 62, 70), Rgb(255, 120, 120), Rgb(150, 30, 44)),
        ),
        (
            7.4,
            11.6,
            (Rgb(250, 204, 64), Rgb(255, 240, 150), Rgb(196, 140, 30)),
        ),
        (
            11.8,
            16.6,
            (Rgb(64, 140, 230), Rgb(130, 196, 255), Rgb(36, 80, 160)),
        ),
    ];
    for (i, (top, bottom, (base, light, dark))) in boxes.into_iter().enumerate() {
        p.draw(
            &Shape::RoundRect(V2(0.5, top), V2(11.5, bottom), 0.7),
            Paint::shaded(base, light, dark),
        );
        let mid = (top + bottom) / 2.0;
        // A bold "!" badge and lines of breathless text.
        p.draw(
            &Shape::Circle(V2(2.4, mid), 1.05),
            Paint::shaded(WHITE, WHITE, Rgb(220, 220, 230)).no_outline(),
        );
        p.draw(
            &Shape::Capsule(V2(2.4, mid - 0.6), V2(2.4, mid + 0.15), 0.2, 0.15),
            Paint::Flat(dark, 1.0),
        );
        p.draw(
            &Shape::Circle(V2(2.4, mid + 0.55), 0.18),
            Paint::Flat(dark, 1.0),
        );
        let lines = [(mid - 0.7, 7.0 - i as f32), (mid + 0.6, 5.0 + i as f32)];
        for (y, len) in lines {
            p.draw(
                &Shape::Capsule(V2(4.2, y), V2(4.2 + len * 0.9, y), 0.28, 0.28),
                Paint::Flat(WHITE, 0.92),
            );
        }
    }
    // A gold "#1" rosette on top.
    p.draw(
        &Shape::Circle(V2(6.0, 1.7), 1.55),
        Paint::shaded(Rgb(255, 205, 80), Rgb(255, 244, 170), Rgb(196, 130, 30)),
    );
    p.draw(
        &Shape::Circle(V2(6.0, 1.7), 0.7),
        Paint::Flat(Rgb(255, 236, 150), 1.0),
    );
    p.finish()
}

/// A swarm of em dashes, flying in formation with motion trails.
fn dashes(scale: usize, beat: f32) -> Bitmap {
    let (w, h) = size(sprites::DASHES_A);
    let mut p = Painter::new(w, h, scale);
    let dash = Paint::shaded(Rgb(110, 214, 236), Rgb(226, 250, 255), Rgb(40, 120, 156));
    let layout = [(2.0, 1.6), (12.2, 2.4), (6.4, 4.4)];
    for (i, (x, y)) in layout.into_iter().enumerate() {
        let bob = if i % 2 == 0 {
            beat * 0.5
        } else {
            (1.0 - beat) * 0.5
        };
        let y = y + bob - 0.25;
        for (k, alpha) in [(2.2, 0.18), (1.1, 0.35)] {
            p.draw(
                &Shape::Capsule(V2(x + k, y), V2(x + k + 5.0, y), 0.55, 0.55),
                Paint::Flat(Rgb(150, 230, 246), alpha),
            );
        }
        p.draw(&Shape::Capsule(V2(x, y), V2(x + 5.6, y), 0.68, 0.68), dash);
        p.draw(
            &Shape::Capsule(V2(x + 0.6, y - 0.3), V2(x + 4.0, y - 0.3), 0.14, 0.14),
            Paint::Flat(WHITE, 0.85),
        );
    }
    p.finish()
}

fn spark(scale: usize, size_k: f32) -> Bitmap {
    let (w, h) = size(sprites::SPARK_A);
    let mut p = Painter::new(w, h, scale);
    let c = V2(2.5, 2.5);
    let (outer, inner) = (2.4 * size_k, 0.75 * size_k);
    let star: Vec<V2> = (0..8)
        .map(|i| {
            let a = i as f32 * PI / 4.0;
            let r = if i % 2 == 0 { outer } else { inner };
            c + V2(a.sin(), -a.cos()) * r
        })
        .collect();
    let shape = Shape::Poly(star);
    p.draw(&shape, Paint::Glow(Rgb(255, 214, 110), 1.3, 0.45));
    p.draw(
        &shape,
        Paint::shaded(Rgb(255, 208, 90), Rgb(255, 250, 210), Rgb(214, 140, 40)).no_outline(),
    );
    p.draw(&Shape::Circle(c, 0.5 * size_k), Paint::Flat(WHITE, 0.9));
    p.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn every_bitmap(art: &ArtSet) -> Vec<(&'static str, &Bitmap, Sprite)> {
        let mut all = Vec::new();
        for frame in &art.run {
            all.push(("run", frame, sprites::RUN_A));
        }
        for frame in &art.duck {
            all.push(("duck", frame, sprites::DUCK));
        }
        all.push(("jump", &art.jump, sprites::JUMP));
        all.push(("slopped", &art.slopped, sprites::SLOPPED));
        all.push(("puddle", &art.puddle[0], sprites::PUDDLE_A));
        all.push(("puddle", &art.puddle[1], sprites::PUDDLE_B));
        all.push(("hand", &art.hand, sprites::HAND));
        all.push(("tower", &art.tower, sprites::TOWER));
        all.push(("dashes", &art.dashes[0], sprites::DASHES_A));
        all.push(("dashes", &art.dashes[1], sprites::DASHES_B));
        all.push(("spark", &art.spark[0], sprites::SPARK_A));
        all.push(("spark", &art.spark[1], sprites::SPARK_B));
        all
    }

    #[test]
    fn every_set_matches_the_hitbox_sizes() {
        for scale in [1, 2, 3] {
            let art = ArtSet::new(scale);
            for (name, bitmap, sprite) in every_bitmap(&art) {
                let (w, h) = size(sprite);
                assert_eq!(
                    (bitmap.w, bitmap.h),
                    (w * scale, h * scale),
                    "{name} at scale {scale}"
                );
                assert!(
                    bitmap.px.iter().any(|p| p[3] > 0),
                    "{name} at scale {scale} is blank"
                );
            }
        }
    }

    #[test]
    fn detailed_runner_is_smooth_and_uses_rich_color() {
        let art = ArtSet::new(2);
        let frame = &art.run[0];
        let colors: HashSet<_> = frame
            .px
            .iter()
            .filter(|p| p[3] == 255)
            .map(|p| [p[0], p[1], p[2]])
            .collect();
        assert!(colors.len() > 120, "only {} colors", colors.len());
        assert!(
            frame.px.iter().any(|p| p[3] > 0 && p[3] < 255),
            "edges should be anti-aliased"
        );
    }

    #[test]
    fn runner_wears_claude_accents() {
        let art = ArtSet::new(2);
        let opaque = || art.run[0].px.iter().filter(|p| p[3] == 255);
        // Shading shifts the exact color, so match terracotta by hue: a warm
        // orange that is redder than skin and more saturated than blush.
        let terracotta = opaque()
            .filter(|p| {
                let (r, g, b) = (i32::from(p[0]), i32::from(p[1]), i32::from(p[2]));
                r >= 150 && r - g >= 60 && g - b >= 20
            })
            .count();
        assert!(terracotta > 20, "terracotta: {terracotta} pixels");
        // Warm off-white (the dress) and neutral dark grey (tights and boots).
        let cream = opaque()
            .filter(|p| p[..3].iter().all(|&c| c >= 165) && p[0] - p[2] <= 45)
            .count();
        let charcoal = opaque()
            .filter(|p| {
                let (lo, hi) = (
                    *p[..3].iter().min().unwrap_or(&0),
                    *p[..3].iter().max().unwrap_or(&0),
                );
                (25..=115).contains(&lo) && hi <= 125 && hi - lo <= 15
            })
            .count();
        assert!(cream > 20, "cream: {cream} pixels");
        assert!(charcoal > 20, "charcoal: {charcoal} pixels");
    }

    #[test]
    fn run_cycle_animates_and_keeps_a_foot_on_the_ground() {
        let art = ArtSet::new(2);
        let distinct: HashSet<_> = art.run.iter().map(|f| f.px.clone()).collect();
        assert_eq!(distinct.len(), RUN_FRAMES);
        for (i, frame) in art.run.iter().enumerate() {
            let lowest = (0..frame.h)
                .rev()
                .find(|&y| (0..frame.w).any(|x| frame.get(x, y)[3] > 128));
            assert!(
                lowest.is_some_and(|y| y + 3 >= frame.h),
                "frame {i} floats: lowest row {lowest:?}"
            );
        }
    }

    #[test]
    fn small_set_is_the_hand_drawn_art() {
        let art = ArtSet::new(1);
        assert_eq!(art.run.len(), 2);
        assert_eq!(art.run[0], sprite_bitmap(sprites::RUN_A));
    }

    /// Writes PNG previews of every sprite: `HEARTH_SLOP_PREVIEW=dir cargo test -- --ignored`.
    #[test]
    #[ignore = "writes preview images for art iteration"]
    fn write_previews() -> std::io::Result<()> {
        let Some(dir) = std::env::var_os("HEARTH_SLOP_PREVIEW") else {
            return Ok(());
        };
        for scale in [1, 2, 4] {
            let art = ArtSet::new(scale);
            let all = every_bitmap(&art);
            let gap = 2 * scale;
            let w: usize = all.iter().map(|(_, b, _)| b.w + gap).sum::<usize>() + gap;
            let h = all.iter().map(|(_, b, _)| b.h).max().unwrap_or(0) + 2 * gap;
            let mut frame = super::super::render::Frame::new(w, h);
            frame.fill(Rgb(46, 48, 66));
            let mut x = gap;
            for (_, bitmap, _) in all {
                frame.blit(bitmap, x as i32, (h - gap - bitmap.h) as i32);
                x += bitmap.w + gap;
            }
            let png = super::super::graphics::png(&frame);
            std::fs::write(
                std::path::Path::new(&dir).join(format!("art-{scale}x.png")),
                png,
            )?;
        }
        Ok(())
    }
}
