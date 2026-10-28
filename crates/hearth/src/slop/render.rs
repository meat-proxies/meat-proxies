//! Draws a game frame into a 24-bit framebuffer, at 1x with the small art or
//! at higher scales with the detailed art, and encodes it for a truecolor
//! terminal two pixels per cell using the upper half block.

use std::fmt::Write as _;

use super::art::ArtSet;
use super::game::{Game, Phase, RUNNER_X};
use super::raster::Bitmap;

/// World units between the bottom of the canvas and the ground line.
pub const GROUND_DEPTH: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn mix(self, other: Rgb, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        let lerp = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
        Rgb(
            lerp(self.0, other.0),
            lerp(self.1, other.1),
            lerp(self.2, other.2),
        )
    }

    pub fn scale(self, k: f32) -> Rgb {
        let f = |c: u8| (f32::from(c) * k).round().clamp(0.0, 255.0) as u8;
        Rgb(f(self.0), f(self.1), f(self.2))
    }
}

pub struct Frame {
    pub width: usize,
    pub height: usize,
    pixels: Vec<Rgb>,
}

impl Frame {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![Rgb(0, 0, 0); width * height],
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Rgb {
        self.pixels[y * self.width + x]
    }

    pub fn pixels(&self) -> &[Rgb] {
        &self.pixels
    }

    #[cfg(test)]
    pub fn fill(&mut self, color: Rgb) {
        self.pixels.fill(color);
    }

    fn set(&mut self, x: i32, y: i32, color: Rgb) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.pixels[y as usize * self.width + x as usize] = color;
        }
    }

    /// Blends `color` over the pixel at `alpha` opacity.
    fn blend(&mut self, x: i32, y: i32, color: Rgb, alpha: f32) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            let i = y as usize * self.width + x as usize;
            self.pixels[i] = self.pixels[i].mix(color, alpha);
        }
    }

    /// Composites a bitmap with its top-left corner at (x, y), clipping at edges.
    pub fn blit(&mut self, bitmap: &Bitmap, x: i32, y: i32) {
        for by in 0..bitmap.h {
            for bx in 0..bitmap.w {
                let [r, g, b, a] = bitmap.get(bx, by);
                if a > 0 {
                    self.blend(
                        x + bx as i32,
                        y + by as i32,
                        Rgb(r, g, b),
                        f32::from(a) / 255.0,
                    );
                }
            }
        }
    }

    /// Encodes the frame as ANSI truecolor text. Only cells that differ from
    /// `previous` are sent, and colors are only re-sent when they change.
    pub fn to_ansi(&self, previous: Option<&Frame>) -> String {
        let previous = previous.filter(|p| p.width == self.width && p.height == self.height);
        let mut out = String::with_capacity(self.width * self.height * 4);
        let mut colors: Option<(Rgb, Rgb)> = None;
        for row in 0..self.height.div_ceil(2) {
            let mut cursor_here = false;
            for x in 0..self.width {
                let cell = self.cell(x, row);
                if previous.is_some_and(|p| p.cell(x, row) == cell) {
                    cursor_here = false;
                    continue;
                }
                if !cursor_here {
                    let _ = write!(out, "\x1b[{};{}H", row + 1, x + 1);
                    cursor_here = true;
                }
                if colors != Some(cell) {
                    let (top, bottom) = cell;
                    let _ = write!(
                        out,
                        "\x1b[38;2;{};{};{};48;2;{};{};{}m",
                        top.0, top.1, top.2, bottom.0, bottom.1, bottom.2
                    );
                    colors = Some(cell);
                }
                out.push('▀');
            }
        }
        out.push_str("\x1b[0m");
        out
    }

    /// The (top, bottom) pixel pair shown by one terminal cell.
    fn cell(&self, x: usize, row: usize) -> (Rgb, Rgb) {
        let top = self.get(x, row * 2);
        let bottom = if row * 2 + 1 < self.height {
            self.get(x, row * 2 + 1)
        } else {
            top
        };
        (top, bottom)
    }
}

/// Screen row of the ground line for a canvas of this height and scale.
pub fn ground_line(height: usize, scale: usize) -> i32 {
    height.saturating_sub(GROUND_DEPTH * scale) as i32
}

pub fn draw(game: &Game, art: &ArtSet, width: usize, height: usize) -> Frame {
    let s = art.scale;
    let mut frame = Frame::new(width, height);
    let palette = palette(game);
    sky(&mut frame, &palette);
    sun(&mut frame, &palette, s);
    clouds(&mut frame, &palette, game.distance * 0.06, s);
    skyline(&mut frame, game.distance * 0.15, palette.far, 22, 41, s);
    skyline(&mut frame, game.distance * 0.4, palette.near, 13, 97, s);
    ground(&mut frame, game.distance, s);
    if matches!(game.phase, Phase::Over { .. }) {
        // Desaturate the world, but not the sprites, while the runner is slopped.
        for pixel in &mut frame.pixels {
            let grey = ((u16::from(pixel.0) + u16::from(pixel.1) + u16::from(pixel.2)) / 3) as u8;
            *pixel = pixel.mix(Rgb(grey, grey, grey), 0.5);
        }
    }

    let place = |bitmap: &Bitmap, x: f32, y: f32| origin(bitmap, x, y, s, height);
    for spark in &game.sparks {
        let bitmap = art.spark(game.ticks);
        let (x, y) = place(bitmap, spark.x, spark.y);
        frame.blit(bitmap, x, y);
    }
    for obstacle in &game.obstacles {
        let bitmap = art.obstacle(obstacle.kind, game.frame());
        let (x, y) = place(bitmap, obstacle.x, obstacle.y());
        frame.blit(bitmap, x, y);
    }
    let runner = art.runner(game);
    let (x, y) = place(runner, RUNNER_X, game.runner_y);
    frame.blit(runner, x, y);
    frame
}

/// Screen position of a bitmap's top-left corner for a world position, where
/// `y` is the height of its bottom edge above the ground.
fn origin(bitmap: &Bitmap, x: f32, y: f32, scale: usize, height: usize) -> (i32, i32) {
    let s = scale as f32;
    (
        (x * s).round() as i32,
        ground_line(height, scale) - (y * s).round() as i32 - bitmap.h as i32,
    )
}

/// Colors for the current time of day.
struct Palette {
    top: Rgb,
    horizon: Rgb,
    sun: Rgb,
    cloud: Rgb,
    far: Rgb,
    near: Rgb,
}

/// The sky drifts from afternoon to sunset to night as the score climbs.
fn palette(game: &Game) -> Palette {
    const TIMES: [[Rgb; 6]; 3] = [
        [
            Rgb(92, 158, 226),
            Rgb(250, 214, 180),
            Rgb(255, 246, 214),
            Rgb(255, 255, 255),
            Rgb(58, 52, 92),
            Rgb(86, 66, 110),
        ],
        [
            Rgb(122, 78, 140),
            Rgb(255, 150, 96),
            Rgb(255, 176, 104),
            Rgb(255, 214, 196),
            Rgb(70, 44, 82),
            Rgb(104, 58, 92),
        ],
        [
            Rgb(18, 20, 48),
            Rgb(70, 56, 112),
            Rgb(232, 232, 250),
            Rgb(120, 116, 160),
            Rgb(30, 28, 60),
            Rgb(48, 40, 82),
        ],
    ];
    let cycle = (game.score() as f32 / 400.0) % 3.0;
    let (from, to, t) = (cycle as usize, (cycle as usize + 1) % 3, cycle.fract());
    let c = |i: usize| TIMES[from][i].mix(TIMES[to][i], t);
    Palette {
        top: c(0),
        horizon: c(1),
        sun: c(2),
        cloud: c(3),
        far: c(4),
        near: c(5),
    }
}

fn sky(frame: &mut Frame, palette: &Palette) {
    let horizon = frame.height.max(1) as f32 * 0.85;
    for y in 0..frame.height {
        let color = palette.top.mix(palette.horizon, y as f32 / horizon);
        frame.pixels[y * frame.width..(y + 1) * frame.width].fill(color);
    }
}

fn sun(frame: &mut Frame, palette: &Palette, s: usize) {
    let s = s as f32;
    let (cx, cy) = (frame.width as f32 * 0.78, frame.height as f32 * 0.24);
    let (radius, glow) = (4.5 * s, 13.0 * s);
    let (x0, x1) = ((cx - glow).max(0.0) as i32, (cx + glow) as i32);
    let (y0, y1) = ((cy - glow).max(0.0) as i32, (cy + glow) as i32);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            if d < radius + 0.5 {
                frame.blend(x, y, palette.sun, (radius + 0.5 - d).clamp(0.0, 1.0));
            } else if d < glow {
                frame.blend(x, y, palette.sun, 0.35 * (1.0 - d / glow).powi(2));
            }
        }
    }
}

fn hash(n: u32) -> u32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^ (x >> 12)
}

/// Soft puffy clouds, shaded underneath and anti-aliased at the edges.
fn clouds(frame: &mut Frame, palette: &Palette, offset: f32, s: usize) {
    const SPACING: f32 = 46.0;
    let sf = s as f32;
    let units_wide = frame.width as f32 / sf;
    let first = (offset / SPACING).floor() as i32 - 1;
    let last = ((offset + units_wide) / SPACING).ceil() as i32 + 1;
    let sky_units = frame.height as f32 / sf * 0.45;
    for slot in first..=last {
        let h = hash(slot as u32 ^ 0xC10D);
        if h.is_multiple_of(3) {
            continue;
        }
        let size = 0.7 + (h >> 8 & 255) as f32 / 255.0 * 0.6;
        let cx = slot as f32 * SPACING + (h >> 16 & 31) as f32 - offset;
        let cy = 4.0 + (h >> 4 & 255) as f32 / 255.0 * (sky_units - 4.0).max(1.0);
        let puffs = [
            (0.0, 0.0, 3.2),
            (-3.6, 1.1, 2.3),
            (3.4, 0.9, 2.5),
            (1.2, -1.4, 2.2),
        ];
        let reach = 7.5 * size;
        let (x0, x1) = (((cx - reach) * sf) as i32, ((cx + reach) * sf) as i32);
        let (y0, y1) = (((cy - reach) * sf) as i32, ((cy + reach) * sf) as i32);
        let base = palette.cloud.mix(palette.top, 0.08);
        let shade = palette
            .cloud
            .mix(palette.horizon, 0.45)
            .mix(palette.top, 0.2);
        for y in y0.max(0)..=y1 {
            for x in x0.max(0)..=x1 {
                let (ux, uy) = ((x as f32 + 0.5) / sf, (y as f32 + 0.5) / sf);
                let d = puffs
                    .iter()
                    .map(|&(px, py, r)| {
                        ((ux - cx - px * size).powi(2) + (uy - cy - py * size).powi(2)).sqrt()
                            - r * size
                    })
                    .fold(f32::MAX, f32::min);
                if d < 0.0 {
                    let alpha = (-d * sf).clamp(0.0, 1.0) * 0.88;
                    let under = ((uy - cy) / (2.5 * size) + 0.3).clamp(0.0, 1.0);
                    frame.blend(x, y, base.mix(shade, under), alpha);
                }
            }
        }
    }
}

/// Server-rack silhouettes scrolling at `offset` units, up to `max_height` units tall.
fn skyline(frame: &mut Frame, offset: f32, color: Rgb, max_height: u32, salt: u32, s: usize) {
    if s >= 3 {
        return detailed_skyline(frame, offset, color, max_height, salt, s);
    }
    let s = s as i32;
    let ground = ground_line(frame.height, s as usize);
    let lit = color.mix(Rgb(255, 210, 130), 0.55);
    let edge = color.scale(1.25);
    let offset_px = (offset * s as f32) as i32;
    for x in 0..frame.width as i32 {
        let world = x + offset_px;
        let (unit, sub) = (world.div_euclid(s), world.rem_euclid(s));
        let block = unit.div_euclid(9) as u32;
        let tall = (4 + hash(block.wrapping_add(salt)) % max_height) as i32 * s;
        let col = unit.rem_euclid(9);
        if col == 8 {
            continue;
        }
        let bits = hash(block ^ salt.rotate_left(7));
        for y in (ground - tall).max(0)..ground {
            let row = (ground - y - 1) / s;
            let window =
                col % 3 == 1 && row % 3 == 1 && row + 2 < tall / s && (bits >> (row % 24)) & 1 == 1;
            let color = if window {
                lit
            } else if s > 1 && col == 0 && sub == 0 {
                edge
            } else {
                color
            };
            frame.set(x, y, color);
        }
        if s > 1 && bits.is_multiple_of(5) && col == 4 && sub == 0 {
            for y in (ground - tall - 2 * s).max(0)..(ground - tall) {
                frame.set(x, y, color);
            }
            frame.set(x, ground - tall - 2 * s - 1, Rgb(255, 120, 96));
        }
    }
}

/// The same skyline with sub-unit detail: shaded facades, lit edges, roof
/// ledges, and framed windows with glass reflections or warm lamplight.
fn detailed_skyline(
    frame: &mut Frame,
    offset: f32,
    color: Rgb,
    max_height: u32,
    salt: u32,
    s: usize,
) {
    let sf = s as f32;
    let ground = ground_line(frame.height, s);
    let offset_px = offset * sf;
    let light = color.mix(Rgb(255, 226, 190), 0.3);
    let shadow = color.scale(0.72);
    for x in 0..frame.width as i32 {
        let u = (x as f32 + 0.5 + offset_px) / sf;
        let block = u.div_euclid(9.0) as i32 as u32;
        let local = u.rem_euclid(9.0);
        if local >= 8.0 {
            continue;
        }
        let tall = (4 + hash(block.wrapping_add(salt)) % max_height) as f32;
        let bits = hash(block ^ salt.rotate_left(7));
        let top = ground - (tall * sf) as i32;
        for y in top.max(0)..ground {
            let v = (ground - y) as f32 / sf; // units above the ground
            // Sky light falls on the top of the facade; it darkens toward the street.
            let mut c = color.scale(0.82 + 0.3 * (v / tall));
            if local < 0.4 {
                c = light;
            } else if local > 7.65 {
                c = shadow;
            }
            if tall - v < 0.5 {
                c = light.mix(Rgb(255, 255, 255), 0.15);
            } else if tall - v < 0.75 {
                c = shadow;
            }
            let (wx, wy) = (local.rem_euclid(3.0), v.rem_euclid(3.0));
            let (row, column) = ((v / 3.0) as u32, (local / 3.0) as u32);
            if (0.85..2.15).contains(&wx)
                && (0.85..2.15).contains(&wy)
                && v + 1.5 < tall
                && local < 7.6
            {
                let lit = (bits.rotate_left(row * 3 + column) & 3) == 0;
                let inner = (0.95..2.05).contains(&wx) && (0.95..2.05).contains(&wy);
                c = if !inner {
                    shadow.scale(0.8)
                } else if lit {
                    // Warm lamplight, brightest at the top of the pane.
                    Rgb(255, 214, 140).mix(Rgb(240, 150, 80), (2.05 - wy) / 1.1)
                } else {
                    // Dark glass with a diagonal sky reflection.
                    let streak = ((wx - wy + 3.0).rem_euclid(1.1) < 0.22) as u8 as f32;
                    color
                        .scale(0.6)
                        .mix(Rgb(150, 170, 220), 0.25 + 0.25 * streak)
                };
            }
            frame.set(x, y, c);
        }
        // Thin rooftop antennas with a warning light.
        if bits.is_multiple_of(5) && (4.0..4.0 + 1.0 / sf).contains(&local) {
            let mast = (2.4 * sf) as i32;
            for y in (top - mast).max(0)..top {
                frame.set(x, y, shadow);
            }
            for dy in 0..2 {
                frame.set(x, top - mast - dy, Rgb(255, 110, 90));
            }
        }
    }
}

/// Grass over textured earth, with pebbles, scrolling with the run.
fn ground(frame: &mut Frame, distance: f32, s: usize) {
    if s >= 3 {
        return detailed_ground(frame, distance, s);
    }
    let si = s as i32;
    let line = ground_line(frame.height, s);
    let offset = (distance * s as f32) as i32;
    let grass = Rgb(120, 176, 88);
    let grass_light = Rgb(176, 214, 112);
    for x in 0..frame.width as i32 {
        let world = x + offset;
        let h = hash(world as u32);
        let blades = if s > 1 {
            (h % (si as u32 + 1)) as i32
        } else {
            0
        };
        for y in (line - blades).max(0)..line + si {
            frame.set(
                x,
                y,
                if y < line + si / 2 {
                    grass_light
                } else {
                    grass
                },
            );
        }
        for y in line + si..frame.height as i32 {
            let depth = (y - line) as f32 / (GROUND_DEPTH * s) as f32;
            let mut color = Rgb(150, 96, 68).mix(Rgb(86, 52, 44), depth);
            if s > 1 {
                let noise = (hash(world as u32 ^ (y as u32) << 16) & 15) as f32 / 15.0;
                color = color.scale(0.94 + noise * 0.1);
            }
            let (ux, uy) = (world.div_euclid(si), (y - line) / si);
            let cell = hash(ux.div_euclid(3) as u32 ^ (uy as u32).wrapping_mul(0x51ED));
            if cell.is_multiple_of(7) && ux.rem_euclid(3) == 1 && uy >= 2 {
                color = if s > 1 && (y - line) % si == 0 {
                    Rgb(226, 186, 140)
                } else {
                    Rgb(198, 154, 112)
                };
            }
            frame.set(x, y, color);
        }
    }
}

/// Ground with sub-unit detail: individual grass blades, layered soil, and
/// round shaded pebbles.
fn detailed_ground(frame: &mut Frame, distance: f32, s: usize) {
    let sf = s as f32;
    let line = ground_line(frame.height, s);
    let offset = (distance * sf) as i32;
    for x in 0..frame.width as i32 {
        let world = x + offset;
        let h = hash(world as u32);
        // Blades of varying height and shade rise above the turf.
        let blade = (h % (s as u32 * 3 / 2 + 1)) as i32;
        let tint = (h >> 8 & 7) as f32 / 7.0;
        for y in (line - blade).max(0)..line {
            let t = (line - y) as f32 / (blade.max(1) as f32);
            frame.set(
                x,
                y,
                Rgb(104, 160, 76).mix(Rgb(196, 226, 128), t * (0.6 + 0.4 * tint)),
            );
        }
        let turf = s as i32;
        for y in line..line + turf {
            let t = (y - line) as f32 / turf as f32;
            frame.set(x, y, Rgb(150, 200, 100).mix(Rgb(86, 132, 64), t));
        }
        for y in line + turf..frame.height as i32 {
            let v = (y - line) as f32 / sf;
            let depth = v / GROUND_DEPTH as f32;
            let noise = (hash(world as u32 ^ (y as u32) << 16) & 15) as f32 / 15.0;
            let strata = (v * 1.7 + (world as f32 / sf * 0.15).sin() * 0.4).sin() * 0.04;
            let mut color = Rgb(156, 102, 72)
                .mix(Rgb(84, 50, 42), depth)
                .scale(0.93 + noise * 0.1 + strata);
            if let Some(shade) = pebble(world as f32 / sf, v) {
                color = shade;
            }
            frame.set(x, y, color);
        }
    }
}

/// The color of a pebble at this ground position (units), if one is there.
fn pebble(u: f32, v: f32) -> Option<Rgb> {
    let (cx, cy) = (u.div_euclid(3.0), v.div_euclid(2.0));
    let h = hash((cx as i32 as u32).wrapping_mul(0x9E37) ^ (cy as i32 as u32).wrapping_mul(0x51ED));
    if !h.is_multiple_of(3) || cy < 1.0 {
        return None;
    }
    let center = (
        cx * 3.0 + 0.8 + (h >> 4 & 15) as f32 / 15.0 * 1.4,
        cy * 2.0 + 0.6 + (h >> 8 & 15) as f32 / 15.0 * 0.8,
    );
    let (rx, ry) = (
        0.32 + (h >> 12 & 7) as f32 / 7.0 * 0.3,
        0.22 + (h >> 15 & 7) as f32 / 7.0 * 0.16,
    );
    let (dx, dy) = ((u - center.0) / rx, (v - center.1) / ry);
    let d = (dx * dx + dy * dy).sqrt();
    if d > 1.0 {
        return None;
    }
    if d > 0.82 {
        return Some(Rgb(98, 66, 52));
    }
    // Lit from the upper right.
    let light = (dx * 0.5 - dy * 0.85).clamp(-1.0, 1.0);
    Some(Rgb(186, 150, 118).mix(
        if light > 0.0 {
            Rgb(236, 210, 176)
        } else {
            Rgb(130, 96, 76)
        },
        light.abs() * 0.8,
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::slop::game::{Input, SlopKind};

    const W: usize = 96;
    const H: usize = 46;

    fn quiet() -> Game {
        let mut game = Game::new(W, 3);
        game.place(SlopKind::Hand, 70.0);
        game
    }

    #[test]
    fn runner_stands_on_the_ground_line_at_every_scale() {
        let game = quiet();
        for scale in [1, 2] {
            let art = ArtSet::new(scale);
            let (w, h) = (W * scale, H * scale);
            let frame = draw(&game, &art, w, h);
            let bitmap = art.runner(&game);
            let (x0, y0) = origin(bitmap, RUNNER_X, 0.0, scale, h);
            assert_eq!(y0 + bitmap.h as i32, ground_line(h, scale), "scale {scale}");
            // The lowest fully opaque runner pixel is on screen, untouched by the background.
            let (dx, dy) = (0..bitmap.h)
                .rev()
                .find_map(|y| {
                    (0..bitmap.w)
                        .find(|&x| bitmap.get(x, y)[3] == 255)
                        .map(|x| (x, y))
                })
                .expect("opaque pixel");
            let [r, g, b, _] = bitmap.get(dx, dy);
            assert_eq!(
                frame.get((x0 + dx as i32) as usize, (y0 + dy as i32) as usize),
                Rgb(r, g, b)
            );
        }
    }

    #[test]
    fn jumping_moves_the_runner_up_the_screen() {
        let mut game = quiet();
        let art = ArtSet::new(2);
        let standing = origin(art.runner(&game), RUNNER_X, game.runner_y, 2, H * 2).1;
        game.input(Input::Jump);
        for _ in 0..10 {
            game.tick();
        }
        let jumping = origin(art.runner(&game), RUNNER_X, game.runner_y, 2, H * 2).1;
        assert!(
            jumping < standing - 20,
            "standing {standing}, jumping {jumping}"
        );
    }

    #[test]
    fn obstacles_are_drawn_where_the_game_says() {
        let game = quiet();
        for scale in [1, 2] {
            let art = ArtSet::new(scale);
            let frame = draw(&game, &art, W * scale, H * scale);
            let bitmap = art.obstacle(SlopKind::Hand, 0);
            let (x0, y0) = origin(bitmap, 70.0, 0.0, scale, H * scale);
            let (cx, cy) = (bitmap.w / 2, bitmap.h * 3 / 4);
            let [r, g, b, a] = bitmap.get(cx, cy);
            assert_eq!(a, 255);
            assert_eq!(
                frame.get(x0 as usize + cx, y0 as usize + cy),
                Rgb(r, g, b),
                "scale {scale}"
            );
        }
    }

    #[test]
    fn detailed_scenes_use_far_more_colors() {
        let game = quiet();
        let count = |scale: usize| {
            let frame = draw(&game, &ArtSet::new(scale), W * scale, H * scale);
            frame.pixels().iter().collect::<HashSet<_>>().len()
        };
        let (small, detailed) = (count(1), count(2));
        assert!(
            detailed > small * 3 && detailed > 1000,
            "1x {small} colors, 2x {detailed}"
        );
    }

    #[test]
    fn full_detail_background_adds_texture() {
        let count = |s: usize| {
            let mut frame = Frame::new(W * s, H * s);
            frame.fill(Rgb(90, 150, 220));
            skyline(&mut frame, 37.0, Rgb(58, 52, 92), 22, 41, s);
            ground(&mut frame, 500.0, s);
            frame.pixels().iter().collect::<HashSet<_>>().len()
        };
        let (plain, detailed) = (count(2), count(5));
        assert!(detailed > plain * 3, "2x {plain} colors, 5x {detailed}");
        assert!((0..400).any(|i| pebble(i as f32 * 0.37, 3.0 + (i % 7) as f32).is_some()));
    }

    #[test]
    fn bitmaps_clip_at_screen_edges_without_panicking() {
        let art = ArtSet::new(2);
        let mut frame = Frame::new(8, 8);
        frame.blit(art.obstacle(SlopKind::Tower, 0), -6, -6);
        frame.blit(art.obstacle(SlopKind::Tower, 0), 5, 5);
    }

    #[test]
    fn blit_blends_partial_alpha() {
        let mut frame = Frame::new(1, 1);
        frame.fill(Rgb(0, 0, 0));
        frame.blit(
            &Bitmap {
                w: 1,
                h: 1,
                px: vec![[200, 100, 50, 128]],
            },
            0,
            0,
        );
        assert_eq!(frame.get(0, 0), Rgb(100, 50, 25));
    }

    #[test]
    fn ansi_encoding_uses_half_blocks_and_truecolor() {
        let mut frame = Frame::new(3, 4);
        frame.set(0, 0, Rgb(1, 2, 3));
        frame.set(0, 1, Rgb(4, 5, 6));
        let ansi = frame.to_ansi(None);
        assert_eq!(ansi.matches('▀').count(), 6, "3 columns x 2 cell rows");
        assert!(ansi.starts_with("\x1b[1;1H\x1b[38;2;1;2;3;48;2;4;5;6m▀"));
        assert!(ansi.ends_with("\x1b[0m"));
        // Repeated colors are not re-sent.
        assert_eq!(ansi.matches("38;2;0;0;0;48;2;0;0;0m").count(), 1);
    }

    #[test]
    fn ansi_diff_sends_only_changed_cells() {
        let before = Frame::new(4, 4);
        let mut after = Frame::new(4, 4);
        after.set(2, 3, Rgb(9, 9, 9));
        let diff = after.to_ansi(Some(&before));
        assert_eq!(diff.matches('▀').count(), 1);
        assert!(diff.starts_with("\x1b[2;3H"), "{diff:?}");
        assert_eq!(after.to_ansi(Some(&after)), "\x1b[0m");
        // A resized previous frame forces a full redraw.
        assert_eq!(
            after.to_ansi(Some(&Frame::new(2, 2))).matches('▀').count(),
            8
        );
    }

    #[test]
    fn world_greys_out_on_game_over() {
        let saturation = |frame: &Frame| {
            let total: u32 = frame
                .pixels()
                .iter()
                .map(|p| u32::from(p.0.max(p.1).max(p.2) - p.0.min(p.1).min(p.2)))
                .sum();
            total as f32 / frame.pixels().len() as f32
        };
        let art = ArtSet::new(1);
        let mut game = Game::new(W, 3);
        game.place(SlopKind::Puddle, RUNNER_X + 4.0);
        let running = saturation(&draw(&game, &art, W, H));
        for _ in 0..30 {
            game.tick();
        }
        assert!(matches!(game.phase, Phase::Over { .. }));
        let over = saturation(&draw(&game, &art, W, H));
        assert!(over < running * 0.7, "running {running}, over {over}");
    }
}
