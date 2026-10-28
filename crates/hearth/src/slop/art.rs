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
const RUN_FRAMES: usize = 10;

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

/// Joint angles for one frame, in radians from straight down, positive toward
/// the front (+x). A limb's lower half turns by `-bend`: positive bends fold
/// knees backward, negative bends fold elbows forward.
struct Rig {
    hip_x: f32,
    lean: f32,
    /// Far leg, then near leg.
    legs: [(f32, f32); 2],
    /// Far arm, then near arm.
    arms: [(f32, f32); 2],
    /// How far the twin tails stream back and up (0 hanging, 1 flying).
    tail_lift: f32,
    tail_sway: f32,
    skirt_flare: f32,
}

// Young-adult proportions in world units: about six heads tall, with legs
// making up half the height.
const THIGH: f32 = 4.1;
const SHIN: f32 = 3.9;
const TORSO: f32 = 4.4;
const UPPER_ARM: f32 = 2.25;
const FOREARM: f32 = 2.05;
/// Ankle to the bottom of the boot sole.
const FOOT_DROP: f32 = 0.62;

fn rig(pose: Pose, phase: f32) -> Rig {
    let t = phase * TAU;
    match pose {
        Pose::Run => {
            let leg = |o: f32| {
                let s = t + o;
                (0.74 * s.sin(), 0.25 + 1.55 * s.cos().max(0.0).powf(0.8))
            };
            let arm = |o: f32| (-0.8 * (t + o).sin(), -1.45);
            Rig {
                hip_x: 6.3,
                lean: 0.2,
                legs: [leg(PI), leg(0.0)],
                arms: [arm(0.0), arm(PI)],
                tail_lift: 0.7,
                tail_sway: (2.0 * t).sin(),
                skirt_flare: 0.55 + 0.35 * (2.0 * t).cos(),
            }
        }
        Pose::Jump => Rig {
            hip_x: 6.4,
            lean: 0.1,
            legs: [(-0.25, 1.9), (1.25, 2.1)],
            arms: [(-1.9, -0.5), (1.15, -1.25)],
            tail_lift: 1.0,
            tail_sway: 0.4,
            skirt_flare: 1.0,
        },
        Pose::Duck => {
            let s = t.sin() * 0.18;
            Rig {
                hip_x: 5.0,
                lean: 1.08,
                legs: [(0.55 + s, 2.45), (1.45 - s, 2.75)],
                arms: [(0.95 + s, 0.35), (1.2 - s, 0.25)],
                tail_lift: 1.0,
                tail_sway: (2.0 * t).sin() * 0.6,
                skirt_flare: 0.9,
            }
        }
        Pose::Slopped => Rig {
            hip_x: 6.8,
            lean: -0.04,
            legs: [(-0.12, 0.15), (0.16, 0.2)],
            arms: [(-0.3, -0.25), (0.35, -0.45)],
            tail_lift: 0.0,
            tail_sway: 0.0,
            skirt_flare: 0.0,
        },
    }
}

/// Knee and ankle offsets from the hip for a leg.
fn leg_joints((a, bend): (f32, f32)) -> (V2, V2) {
    let knee = V2::down_at(a) * THIGH;
    (knee, knee + V2::down_at(a - bend) * SHIN)
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
    let down = up * -1.0;
    let fwd = V2(r.lean.cos(), r.lean.sin());

    // Rest the lowest boot on the sprite's bottom edge.
    let lowest = r
        .legs
        .iter()
        .map(|&leg| leg_joints(leg).1.1)
        .fold(f32::MIN, f32::max);
    let hip = V2(r.hip_x, h as f32 - 0.05 - FOOT_DROP - lowest);
    let waist = hip + up * 1.45;
    let chest = hip + up * 2.85;
    let neck = hip + up * TORSO;
    let shoulder = neck + down * 0.5 + fwd * -0.1;
    // The head stays nearly upright even when the body leans.
    let head = neck + up * 0.5 + V2(0.0, -1.15) + V2(0.3, 0.0);

    let mut p = Painter::new(w, h, scale);

    let far_tail = twin_tail(head + V2(-0.5, -1.4), &r, -1.0);
    let near_tail = twin_tail(head + V2(-1.15, -0.65), &r, 1.0);
    draw_tail(
        &mut p,
        &far_tail,
        Paint::shaded(HAIR_DARK.mix(HAIR, 0.55), HAIR, HAIR_DARK),
    );

    // Far arm and both legs sit behind the dress.
    arm(&mut p, shoulder + fwd * -0.25, r.arms[0], true);
    for (i, &leg) in r.legs.iter().enumerate() {
        let (knee, ankle) = leg_joints(leg);
        let tint = if i == 0 { 0.82 } else { 1.0 };
        draw_leg(
            &mut p,
            hip + fwd * (0.15 * i as f32 - 0.1),
            hip + knee,
            hip + ankle,
            tint,
        );
    }

    // Dress: shaped bodice, flared skirt hanging with gravity.
    let skirt_lean = r.lean * 0.3;
    let sfwd = V2(skirt_lean.cos(), skirt_lean.sin());
    let sdown = V2(-skirt_lean.sin(), skirt_lean.cos());
    let f = r.skirt_flare;
    let hem = 2.15 + 0.15 * f;
    let bodice = Shape::Capsule(waist, neck + down * 0.55, 0.6, 0.72)
        .blend(Shape::Circle(chest + fwd * 0.42 + up * 0.15, 0.56), 0.35)
        .blend(Shape::Capsule(hip, waist, 0.82, 0.6), 0.3);
    let skirt = Shape::Poly(vec![
        waist + fwd * 0.62,
        hip + sfwd * 1.05 + sdown * 0.45,
        hip + sfwd * (1.55 + 0.35 * f) + sdown * hem,
        hip + sfwd * 0.35 + sdown * (hem + 0.3),
        hip + sfwd * (-1.0 - 0.3 * f) + sdown * (hem + 0.15),
        hip + sfwd * (-1.75 - 0.9 * f) + sdown * (hem - 0.25),
        hip + sfwd * -0.95 + sdown * 0.3,
        waist + fwd * -0.62,
    ]);
    let dress = bodice.blend(skirt, 0.25);
    p.draw(&dress, cream());
    // Soft fold shadows down the skirt.
    for (x, len, alpha) in [(-0.9, 1.6, 0.55), (0.1, 1.9, 0.45), (0.9, 1.4, 0.4)] {
        let a = hip + sfwd * x + sdown * 0.6;
        p.draw(
            &Shape::Capsule(a, a + sdown * len + sfwd * (x * 0.35), 0.1, 0.16),
            Paint::Flat(CREAM_DARK, alpha),
        );
    }
    let hem_line = hip + sdown * (hem - 0.42);
    let sup = sdown * -1.0;
    p.draw(
        &dress.clone().and(Shape::HalfPlane(hem_line, sup)),
        terra().no_outline(),
    );
    let trim = Shape::HalfPlane(hem_line, sup).and(Shape::HalfPlane(hem_line + sdown * 0.1, sdown));
    p.draw(&dress.clone().and(trim), Paint::Flat(CREAM_LIGHT, 0.95));
    // Terracotta sash with a bow and streamers at the back.
    let sash =
        Shape::HalfPlane(waist + up * 0.3, up).and(Shape::HalfPlane(waist + down * 0.3, down));
    p.draw(&dress.clone().and(sash), terra().no_outline());
    let bow = waist + fwd * -0.7;
    for (dx, dy) in [(-0.9, 1.7 + 0.4 * f), (-1.5 - 0.5 * f, 1.3 + 0.2 * f)] {
        p.draw(&Shape::Capsule(bow, bow + V2(dx, dy), 0.16, 0.1), terra());
    }
    p.draw(
        &Shape::Poly(vec![bow, bow + V2(-0.85, -0.5), bow + V2(-0.8, 0.45)]),
        terra(),
    );
    p.draw(
        &Shape::Circle(bow, 0.2),
        Paint::shaded(TERRA_DARK, TERRA, TERRA_DARK),
    );
    // Collar piping and a small four-point star brooch.
    p.draw(
        &Shape::Capsule(
            neck + fwd * -0.45 + down * 0.35,
            neck + fwd * 0.5 + down * 0.5,
            0.09,
            0.09,
        ),
        Paint::Flat(TERRA, 1.0),
    );
    star(&mut p, neck + fwd * 0.42 + down * 0.85, 0.24);

    draw_tail(&mut p, &near_tail, hair());

    // Neck and head.
    p.draw(
        &Shape::Capsule(neck + down * 0.2, head + V2(0.05, 0.95), 0.27, 0.25),
        skin(),
    );
    p.draw(&Shape::Ellipse(head + V2(-0.22, -0.12), 1.6, 1.65), hair());
    let jaw = Shape::Poly(vec![
        head + V2(-0.3, 0.8),
        head + V2(1.25, 0.45),
        head + V2(1.08, 1.15),
        head + V2(0.55, 1.58),
        head + V2(-0.25, 1.35),
    ]);
    let face = Shape::Ellipse(head + V2(0.3, 0.02), 1.12, 1.3)
        .blend(jaw, 0.25)
        .and(Shape::HalfPlane(head + V2(-0.6, 0.0), V2(-1.0, 0.0)));
    p.draw(&face, skin().no_outline());
    // Shadow under the jaw onto the neck.
    p.draw(
        &Shape::Ellipse(head + V2(0.1, 1.45), 0.45, 0.14),
        Paint::Flat(SKIN_DARK, 0.6),
    );

    if pose == Pose::Slopped {
        for c in [head + V2(0.62, 0.15), head + V2(0.03, 0.15)] {
            p.draw(
                &Shape::Capsule(c + V2(-0.22, 0.02), c + V2(0.2, 0.08), 0.06, 0.05),
                Paint::Flat(EYE, 1.0),
            );
        }
        p.draw(
            &Shape::Capsule(head + V2(0.72, 1.0), head + V2(0.98, 0.94), 0.06, 0.05),
            Paint::Flat(Rgb(150, 70, 70), 1.0),
        );
    } else {
        eye(&mut p, head + V2(0.72, 0.15), 1.0);
        eye(&mut p, head + V2(0.05, 0.15), 0.82);
        // Lips, a little color.
        p.draw(
            &Shape::Capsule(head + V2(0.86, 1.05), head + V2(1.08, 1.02), 0.06, 0.05),
            Paint::Flat(Rgb(196, 98, 96), 1.0),
        );
        p.draw(
            &Shape::Circle(head + V2(0.96, 1.11), 0.04),
            Paint::Flat(SKIN_LIGHT, 0.8),
        );
    }
    // Nose shading on the profile edge, and blush.
    p.draw(
        &Shape::Capsule(head + V2(1.2, 0.45), head + V2(1.24, 0.68), 0.04, 0.05),
        Paint::Flat(SKIN_DARK, 0.85),
    );
    p.draw(
        &Shape::Ellipse(head + V2(0.65, 0.62), 0.32, 0.12),
        Paint::Flat(BLUSH, 0.4),
    );

    // Bangs sweep across the brow; side locks frame the face.
    for (a, b, ra) in [
        (V2(-0.3, -1.55), V2(0.5, -0.3), 0.38),
        (V2(0.15, -1.6), V2(0.95, -0.45), 0.32),
        (V2(0.55, -1.45), V2(1.3, -0.6), 0.24),
        (V2(-0.55, -1.3), V2(0.05, -0.25), 0.3),
    ] {
        p.draw(
            &Shape::Capsule(head + a, head + b, ra, 0.05),
            hair().no_outline(),
        );
    }
    for (a, b) in [
        (V2(-0.6, -0.8), V2(-0.5, 1.6)),
        (V2(-0.3, -1.0), V2(-0.15, 1.1)),
    ] {
        p.draw(&Shape::Capsule(head + a, head + b, 0.26, 0.06), hair());
    }
    // Glossy highlight bands across the crown.
    for (a, b) in [
        (V2(-1.35, -0.6), V2(-0.65, -1.4)),
        (V2(-0.35, -1.55), V2(0.45, -1.42)),
    ] {
        p.draw(
            &Shape::Capsule(head + a, head + b, 0.12, 0.08),
            Paint::Flat(HAIR_LIGHT, 0.8),
        );
    }
    ribbon(&mut p, far_tail[0]);
    ribbon(&mut p, near_tail[0]);

    // Near arm on top.
    arm(&mut p, shoulder + fwd * 0.05, r.arms[1], false);

    if pose == Pose::Slopped {
        let goo = Paint::shaded(Rgb(186, 172, 134), Rgb(232, 222, 188), Rgb(120, 108, 80));
        let top = head + V2(-0.1, -1.55);
        let blob =
            Shape::Ellipse(top, 1.7, 0.6).blend(Shape::Circle(top + V2(0.5, -0.25), 0.55), 0.35);
        p.draw(&blob, goo);
        for (x, len) in [(-1.1, 1.6), (0.25, 2.1), (1.1, 1.1)] {
            let a = top + V2(x, 0.3);
            p.draw(
                &Shape::Capsule(a, a + V2(0.05, len), 0.22, 0.15),
                goo.no_outline(),
            );
        }
        p.draw(&Shape::Ellipse(shoulder + fwd * 0.3, 0.6, 0.35), goo);
        p.draw(
            &Shape::Circle(top + V2(-0.55, -0.12), 0.16),
            Paint::Flat(CREAM_LIGHT, 0.85),
        );
    }
    p.finish()
}

/// Control points for a twin tail tied at `root`; `side` picks far (-1) or near (1).
fn twin_tail(root: V2, r: &Rig, side: f32) -> [V2; 6] {
    // The far tail flies higher and flatter so the two never stack up.
    let (path, lift) = if side < 0.0 {
        (
            [
                (0.0, 0.0),
                (-1.3, 0.4),
                (-2.8, 1.4),
                (-4.0, 2.8),
                (-4.6, 4.2),
                (-4.7, 5.2),
            ],
            r.tail_lift * 0.6 + 0.2,
        )
    } else {
        (
            [
                (0.0, 0.0),
                (-1.0, 1.0),
                (-2.0, 2.9),
                (-2.7, 4.9),
                (-2.9, 6.7),
                (-2.6, 7.9),
            ],
            r.tail_lift,
        )
    };
    let sway = r.tail_sway * (0.35 + side * 0.08);
    let mut pts = [root; 6];
    for (i, (x, y)) in path.into_iter().enumerate() {
        let k = i as f32 / 5.0;
        pts[i] = root
            + V2(
                x - lift * 3.0 * k,
                y - lift * 4.6 * k * k.sqrt() + sway * 1.8 * k * k,
            );
    }
    pts
}

fn draw_tail(p: &mut Painter, pts: &[V2; 6], paint: Paint) {
    let radii = [0.36, 0.52, 0.5, 0.42, 0.3, 0.1];
    // One blended shape, so the shading flows down the whole tail without seams,
    // with the tip splitting into separate locks.
    let mut tail = Shape::Capsule(pts[0], pts[1], radii[0], radii[1]);
    for i in 1..4 {
        tail = tail.blend(
            Shape::Capsule(pts[i], pts[i + 1], radii[i], radii[i + 1]),
            0.3,
        );
    }
    for (dx, dy, r) in [(-0.3, 0.2, 0.2), (0.25, 0.35, 0.16), (0.0, -0.1, 0.22)] {
        tail = tail.blend(Shape::Capsule(pts[4], pts[5] + V2(dx, dy), r, 0.04), 0.08);
    }
    p.draw(&tail, paint);
    // Strand highlights running down the tail.
    for (o, alpha) in [(0.2, 0.75), (-0.15, 0.45)] {
        for i in 1..4 {
            let a = pts[i] + V2(o, -0.05);
            let b = pts[i + 1] + V2(o * 0.8, -0.05);
            p.draw(
                &Shape::Capsule(a, b, 0.06, 0.04),
                Paint::Flat(HAIR_LIGHT, alpha),
            );
        }
    }
}

fn draw_leg(p: &mut Painter, hip: V2, knee: V2, ankle: V2, tint: f32) {
    let tights = Paint::shaded(
        CHARCOAL.scale(tint),
        CHARCOAL_LIGHT.scale(tint),
        CHARCOAL_DARK,
    );
    let shin = ankle - knee;
    let calf = knee + shin * 0.3 + V2(-shin.1, shin.0) * (-0.06);
    let leg = Shape::Capsule(hip, knee, 0.56, 0.34)
        .blend(Shape::Capsule(knee, ankle, 0.33, 0.2), 0.15)
        .blend(Shape::Circle(calf, 0.34), 0.35);
    p.draw(&leg, tights);
    // Ankle boot: shaft up the shin, rounded toe, terracotta sole.
    let shaft = ankle - shin * (1.2 / SHIN);
    let toe = ankle + V2(0.95, 0.3);
    let boot = Shape::Capsule(shaft, ankle, 0.34, 0.3)
        .blend(Shape::Capsule(ankle + V2(-0.15, 0.1), toe, 0.32, 0.22), 0.2);
    p.draw(
        &boot,
        Paint::shaded(CHARCOAL_DARK, CHARCOAL_LIGHT, Rgb(20, 18, 22)),
    );
    p.draw(
        &Shape::Capsule(ankle + V2(-0.35, 0.5), toe + V2(0.15, 0.22), 0.09, 0.09),
        Paint::Flat(TERRA, 1.0),
    );
}

/// An arm with a puffed sleeve; `far` arms are drawn a shade darker.
fn arm(p: &mut Painter, shoulder: V2, (a, bend): (f32, f32), far: bool) {
    let elbow = shoulder + V2::down_at(a) * UPPER_ARM;
    let wrist = elbow + V2::down_at(a - bend) * FOREARM;
    let paint = if far {
        Paint::shaded(SKIN.mix(SKIN_DARK, 0.4), SKIN, SKIN_DARK)
    } else {
        skin()
    };
    let limb = Shape::Capsule(shoulder, elbow, 0.27, 0.2)
        .blend(Shape::Capsule(elbow, wrist, 0.2, 0.15), 0.1);
    p.draw(&limb, paint);
    p.draw(
        &Shape::Circle(wrist + V2::down_at(a - bend) * 0.18, 0.2),
        paint,
    );
    let dir = V2::down_at(a);
    let across = V2(dir.1, -dir.0);
    let sleeve = if far {
        Paint::shaded(CREAM_DARK, CREAM, CREAM_DARK.mix(CHARCOAL, 0.2))
    } else {
        cream()
    };
    p.draw(
        &Shape::Capsule(shoulder - dir * 0.1, shoulder + dir * 0.55, 0.45, 0.4),
        sleeve,
    );
    let cuff = shoulder + dir * 0.82;
    p.draw(
        &Shape::Capsule(cuff - across * 0.36, cuff + across * 0.36, 0.1, 0.1),
        Paint::Flat(TERRA, 1.0),
    );
}

/// A 3/4-view eye: white, amber iris, pupil, catchlight, lash line and brow.
fn eye(p: &mut Painter, c: V2, s: f32) {
    p.draw(
        &Shape::Ellipse(c, 0.3 * s, 0.22),
        Paint::Flat(Rgb(250, 246, 242), 1.0),
    );
    p.draw(
        &Shape::Ellipse(c + V2(0.06 * s, 0.02), 0.17 * s, 0.22),
        Paint::Flat(IRIS, 1.0),
    );
    p.draw(
        &Shape::Ellipse(c + V2(0.06 * s, -0.04), 0.1 * s, 0.13),
        Paint::Flat(EYE, 1.0),
    );
    p.draw(
        &Shape::Circle(c + V2(0.1 * s, -0.1), 0.06),
        Paint::Flat(WHITE, 1.0),
    );
    p.draw(
        &Shape::Capsule(
            c + V2(-0.33 * s, -0.15),
            c + V2(0.32 * s, -0.23),
            0.065,
            0.055,
        ),
        Paint::Flat(EYE, 1.0),
    );
    p.draw(
        &Shape::Capsule(
            c + V2(-0.33 * s, -0.15),
            c + V2(-0.46 * s, -0.25),
            0.05,
            0.02,
        ),
        Paint::Flat(EYE, 1.0),
    );
    p.draw(
        &Shape::Capsule(
            c + V2(-0.26 * s, -0.48),
            c + V2(0.3 * s, -0.52),
            0.045,
            0.04,
        ),
        Paint::Flat(HAIR_DARK, 0.9),
    );
}

fn ribbon(p: &mut Painter, c: V2) {
    let paint = terra();
    p.draw(
        &Shape::Poly(vec![c, c + V2(-0.7, -0.55), c + V2(-0.75, 0.3)]),
        paint,
    );
    p.draw(
        &Shape::Poly(vec![c, c + V2(0.6, -0.6), c + V2(0.7, 0.2)]),
        paint,
    );
    p.draw(&Shape::Capsule(c, c + V2(-0.25, 0.95), 0.12, 0.08), paint);
    p.draw(
        &Shape::Circle(c, 0.2),
        Paint::shaded(TERRA_DARK, TERRA, TERRA_DARK),
    );
}

/// A small four-point star, a nod to the sparkle in Claude's colors.
fn star(p: &mut Painter, c: V2, r: f32) {
    let points: Vec<V2> = (0..8)
        .map(|i| {
            let a = i as f32 * PI / 4.0;
            let len = if i % 2 == 0 { r } else { r * 0.32 };
            c + V2(a.sin(), -a.cos()) * len
        })
        .collect();
    p.draw(
        &Shape::Poly(points),
        Paint::shaded(TERRA, TERRA_LIGHT, TERRA_DARK).no_outline(),
    );
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
        let art = ArtSet::new(5);
        let opaque = || art.run[0].px.iter().filter(|p| p[3] == 255);
        // Shading shifts the exact color, so match terracotta by hue: a warm
        // orange that is redder than skin and more saturated than blush.
        let terracotta = opaque()
            .filter(|p| {
                let (r, g, b) = (i32::from(p[0]), i32::from(p[1]), i32::from(p[2]));
                r >= 150 && r - g >= 60 && g - b >= 20
            })
            .count();
        assert!(terracotta > 40, "terracotta: {terracotta} pixels");
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
        assert!(cream > 150, "cream: {cream} pixels");
        assert!(charcoal > 100, "charcoal: {charcoal} pixels");
    }

    #[test]
    fn runner_has_adult_proportions() {
        // Standing height: hair-top of the head down to the boot soles.
        let head = 2.0 * 1.65;
        let height = head + 0.5 + TORSO + THIGH + SHIN + FOOT_DROP;
        assert!(
            (THIGH + SHIN) / height >= 0.45,
            "legs are {:.0}% of height",
            100.0 * (THIGH + SHIN) / height
        );
        assert!(
            head / height <= 0.21,
            "head is {:.0}% of height",
            100.0 * head / height
        );
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
        for scale in [1, 2, 5] {
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
