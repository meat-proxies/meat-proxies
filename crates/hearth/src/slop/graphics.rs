//! Terminal image output: picks a graphics mode and encodes frames for the
//! Kitty graphics protocol or iTerm2 inline images. Everything else uses
//! half-block text from `render`.

use std::fmt::Write as _;

use super::render::Frame;

/// Environment variable that overrides graphics detection.
pub const OVERRIDE_VAR: &str = "HEARTH_SLOP_GRAPHICS";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Graphics {
    /// Two pixels per cell with upper half blocks; any truecolor terminal.
    Blocks,
    /// Kitty graphics protocol: Kitty, WezTerm, Ghostty.
    Kitty,
    /// iTerm2 inline images (OSC 1337).
    Iterm,
    /// Sixel images with a dithered 252-color palette: Windows Terminal.
    Sixel,
}

impl Graphics {
    /// Picks a mode from environment variables. Unknown terminals, and
    /// multiplexers like tmux that hide the outer terminal, get blocks.
    pub fn detect(var: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        if let Some(choice) = var(OVERRIDE_VAR) {
            return match choice.as_str() {
                "" | "auto" => Ok(Self::sniff(&var)),
                "blocks" => Ok(Self::Blocks),
                "kitty" => Ok(Self::Kitty),
                "iterm" => Ok(Self::Iterm),
                "sixel" => Ok(Self::Sixel),
                other => Err(format!(
                    "{OVERRIDE_VAR}={other} is not one of auto, blocks, kitty, iterm, sixel."
                )),
            };
        }
        Ok(Self::sniff(&var))
    }

    fn sniff(var: &impl Fn(&str) -> Option<String>) -> Self {
        let is = |name: &str, value: &str| var(name).is_some_and(|v| v == value);
        if var("TMUX").is_some() || var("STY").is_some() {
            return Self::Blocks;
        }
        if is("TERM", "xterm-kitty")
            || is("TERM", "xterm-ghostty")
            || is("TERM_PROGRAM", "WezTerm")
            || is("TERM_PROGRAM", "ghostty")
            || var("KITTY_WINDOW_ID").is_some()
        {
            Self::Kitty
        } else if is("TERM_PROGRAM", "iTerm.app") || is("LC_TERMINAL", "iTerm2") {
            Self::Iterm
        } else if var("WT_SESSION").is_some() {
            // Windows Terminal, directly or from WSL (which forwards WT_SESSION).
            Self::Sixel
        } else {
            Self::Blocks
        }
    }
}

/// Kitty protocol: transmit zlib-compressed RGB and place it over `cols` x `rows` cells at
/// the top-left. Images alternate between two ids so the new frame is on
/// screen before the old one is deleted, which avoids flicker.
pub fn kitty(frame: &Frame, cols: u16, rows: u16, id: u32) -> String {
    let mut rgb = Vec::with_capacity(frame.pixels().len() * 3);
    for p in frame.pixels() {
        rgb.extend_from_slice(&[p.0, p.1, p.2]);
    }
    let data = base64(&miniz_oxide::deflate::compress_to_vec_zlib(&rgb, 1));
    let mut out = String::with_capacity(data.len() + 256);
    out.push_str("\x1b[1;1H");
    let chunks: Vec<&str> = data
        .as_bytes()
        .chunks(4096)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect();
    for (i, chunk) in chunks.iter().enumerate() {
        let more = u8::from(i + 1 < chunks.len());
        if i == 0 {
            let _ = write!(
                out,
                "\x1b_Ga=T,f=24,o=z,s={},v={},c={cols},r={rows},i={id},p=1,q=2,C=1,m={more};{chunk}\x1b\\",
                frame.width, frame.height
            );
        } else {
            let _ = write!(out, "\x1b_Gm={more};{chunk}\x1b\\");
        }
    }
    let _ = write!(
        out,
        "\x1b_Ga=d,d=I,i={},q=2\x1b\\",
        if id == 1 { 2 } else { 1 }
    );
    out
}

/// Removes every Kitty image this program placed.
pub const KITTY_CLEAR: &str = "\x1b_Ga=d,d=A,q=2\x1b\\";

/// iTerm2 inline image, stretched over `cols` x `rows` cells at the top-left.
pub fn iterm(frame: &Frame, cols: u16, rows: u16) -> String {
    let png = png(frame);
    format!(
        "\x1b[1;1H\x1b]1337;File=inline=1;size={};width={cols};height={rows};preserveAspectRatio=0:{}\x07",
        png.len(),
        base64(&png)
    )
}

/// Levels per channel in the fixed Sixel palette: 6 x 7 x 6 = 252 colors.
const SIXEL_LEVELS: [usize; 3] = [6, 7, 6];

/// Windows Terminal lays Sixel out on a 10 x 20 pixel virtual cell.
pub const SIXEL_CELL: (usize, usize) = (10, 20);

/// Sixel image at the top-left, each frame pixel drawn `upscale` times bigger.
/// Colors are ordered-dithered onto a fixed palette so gradients stay smooth.
pub fn sixel(frame: &Frame, upscale: usize) -> String {
    const BAYER: [[f32; 4]; 4] = [
        [0.0, 8.0, 2.0, 10.0],
        [12.0, 4.0, 14.0, 6.0],
        [3.0, 11.0, 1.0, 9.0],
        [15.0, 7.0, 13.0, 5.0],
    ];
    let (w, h) = (frame.width, frame.height);
    let index: Vec<u8> = frame
        .pixels()
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let threshold = BAYER[(i / w.max(1)) % 4][i % w.max(1) % 4] / 16.0 - 0.5;
            let level = |c: u8, n: usize| {
                let v = f32::from(c) / 255.0 * (n - 1) as f32 + threshold;
                v.round().clamp(0.0, (n - 1) as f32) as usize
            };
            let [lr, lg, lb] = SIXEL_LEVELS;
            ((level(p.0, lr) * lg + level(p.1, lg)) * lb + level(p.2, lb)) as u8
        })
        .collect();
    let (out_w, out_h) = (w * upscale, h * upscale);

    let mut out = String::with_capacity(out_w * out_h / 2);
    let _ = write!(out, "\x1b[1;1H\x1bP0;1;0q\"1;1;{out_w};{out_h}");
    let [lr, lg, lb] = SIXEL_LEVELS;
    for i in 0..lr * lg * lb {
        let pct = |v: usize, n: usize| v * 100 / (n - 1);
        let _ = write!(
            out,
            "#{i};2;{};{};{}",
            pct(i / (lg * lb), lr),
            pct(i / lb % lg, lg),
            pct(i % lb, lb)
        );
    }
    // Per band, collect each color's six-row bit pattern at source width in
    // one pass, then emit only the colors that appear, run-length encoded.
    let colors = lr * lg * lb;
    let mut bits = vec![0u8; colors * w];
    let mut seen = vec![false; colors];
    let mut used = Vec::with_capacity(colors);
    for band in (0..out_h).step_by(6) {
        let rows = (out_h - band).min(6);
        for dy in 0..rows {
            let row = &index[(band + dy) / upscale * w..][..w];
            for (x, &c) in row.iter().enumerate() {
                let c = usize::from(c);
                if !seen[c] {
                    seen[c] = true;
                    used.push(c);
                }
                bits[c * w + x] |= 1 << dy;
            }
        }
        used.sort_unstable();
        for (n, &color) in used.iter().enumerate() {
            if n > 0 {
                out.push('$');
            }
            let _ = write!(out, "#{color}");
            let line = &mut bits[color * w..(color + 1) * w];
            let mut run: (u8, usize) = (line[0], 0);
            for &b in line.iter() {
                if b == run.0 {
                    run.1 += upscale;
                } else {
                    push_run(&mut out, run);
                    run = (b, upscale);
                }
            }
            // Trailing empty columns need not be sent.
            if run.0 != 0 {
                push_run(&mut out, run);
            }
            line.fill(0);
            seen[color] = false;
        }
        used.clear();
        out.push('-');
    }
    out.push_str("\x1b\\");
    out
}

fn push_run(out: &mut String, (bits, count): (u8, usize)) {
    let c = char::from(63 + bits);
    if count > 3 {
        let _ = write!(out, "!{count}{c}");
    } else {
        out.extend(std::iter::repeat_n(c, count));
    }
}

pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &b)| n | u32::from(b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Encodes the frame as an RGB PNG.
pub fn png(frame: &Frame) -> Vec<u8> {
    let mut raw = Vec::with_capacity((frame.width * 3 + 1) * frame.height);
    for row in frame.pixels().chunks(frame.width.max(1)) {
        raw.push(0); // filter: none
        for p in row {
            raw.extend_from_slice(&[p.0, p.1, p.2]);
        }
    }
    let zlib = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 1);

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&(frame.width as u32).to_be_bytes());
    ihdr.extend_from_slice(&(frame.height as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit RGB

    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    for (kind, data) in [
        (b"IHDR", ihdr.as_slice()),
        (b"IDAT", zlib.as_slice()),
        (b"IEND", &[][..]),
    ] {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let start = out.len();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let crc = crc32(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
    }
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                crc >> 1 ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slop::render::Rgb;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn detects_image_capable_terminals() {
        let cases: &[(&[(&str, &str)], Graphics)] = &[
            (&[("TERM", "xterm-kitty")], Graphics::Kitty),
            (&[("TERM_PROGRAM", "WezTerm")], Graphics::Kitty),
            (&[("TERM", "xterm-ghostty")], Graphics::Kitty),
            (&[("TERM_PROGRAM", "iTerm.app")], Graphics::Iterm),
            (&[("TERM", "xterm-256color")], Graphics::Blocks),
            (&[("WT_SESSION", "8c2c0bb4-...")], Graphics::Sixel),
            (&[], Graphics::Blocks),
            // tmux hides the outer terminal, so do not risk image escapes.
            (
                &[("TERM", "xterm-kitty"), ("TMUX", "/tmp/tmux-0/default,1,0")],
                Graphics::Blocks,
            ),
        ];
        for (vars, expected) in cases {
            assert_eq!(Graphics::detect(env(vars)), Ok(*expected), "{vars:?}");
        }
    }

    #[test]
    fn override_wins_and_rejects_typos() {
        assert_eq!(
            Graphics::detect(env(&[(OVERRIDE_VAR, "blocks"), ("TERM", "xterm-kitty")])),
            Ok(Graphics::Blocks)
        );
        assert_eq!(
            Graphics::detect(env(&[(OVERRIDE_VAR, "kitty")])),
            Ok(Graphics::Kitty)
        );
        assert_eq!(
            Graphics::detect(env(&[(OVERRIDE_VAR, "auto"), ("LC_TERMINAL", "iTerm2")])),
            Ok(Graphics::Iterm)
        );
        assert_eq!(
            Graphics::detect(env(&[(OVERRIDE_VAR, "sixel")])),
            Ok(Graphics::Sixel)
        );
        assert!(Graphics::detect(env(&[(OVERRIDE_VAR, "sixels")])).is_err());
    }

    #[test]
    fn base64_matches_rfc_4648_vectors() {
        for (input, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(input.as_bytes()), expected);
        }
    }

    #[test]
    fn checksums_match_known_values() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn png_has_valid_structure() {
        let mut frame = Frame::new(3, 2);
        frame.fill(Rgb(10, 20, 30));
        let png = png(&frame);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(u32::from_be_bytes([png[16], png[17], png[18], png[19]]), 3);
        assert_eq!(u32::from_be_bytes([png[20], png[21], png[22], png[23]]), 2);
        assert_eq!(
            crc32(&png[12..29]),
            u32::from_be_bytes([png[29], png[30], png[31], png[32]])
        );
        assert!(png.ends_with(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82]));
        // The image data inflates back to the scanlines: filter byte then RGB.
        let len = u32::from_be_bytes([png[33], png[34], png[35], png[36]]) as usize;
        assert_eq!(&png[37..41], b"IDAT");
        let raw =
            miniz_oxide::inflate::decompress_to_vec_zlib(&png[41..41 + len]).expect("valid zlib");
        assert_eq!(raw, [0, 10, 20, 30, 10, 20, 30, 10, 20, 30].repeat(2));
    }

    #[test]
    fn kitty_frames_are_chunked_and_swap_ids() {
        let frame = Frame::new(64, 48);
        let out = kitty(&frame, 32, 12, 1);
        assert!(out.starts_with("\x1b[1;1H\x1b_Ga=T,f=24,o=z,s=64,v=48,c=32,r=12,i=1,"));
        for chunk in out.split("\x1b_G").skip(1) {
            let payload = chunk
                .split_once(';')
                .map_or("", |(_, p)| p)
                .trim_end_matches("\x1b\\");
            assert!(payload.len() <= 4096);
        }
        assert_eq!(out.matches("m=0;").count(), 1, "exactly one final chunk");
        assert!(
            out.ends_with("\x1b_Ga=d,d=I,i=2,q=2\x1b\\"),
            "deletes the previous frame"
        );
    }

    #[test]
    fn iterm_frames_carry_a_png() {
        let frame = Frame::new(4, 4);
        let out = iterm(&frame, 2, 1);
        assert!(out.contains("1337;File=inline=1;"));
        assert!(out.contains("width=2;height=1;preserveAspectRatio=0:iVBORw0KGgo"));
        assert!(out.ends_with('\x07'));
    }

    #[test]
    fn kitty_payload_is_zlib_of_the_raw_pixels() {
        let mut frame = Frame::new(8, 4);
        frame.fill(Rgb(1, 2, 3));
        let out = kitty(&frame, 4, 2, 1);
        let payload = out
            .split_once("m=0;")
            .map(|(_, p)| p)
            .and_then(|p| p.split_once('\x1b'))
            .map(|(p, _)| p);
        let bytes = base64_decode(payload.expect("payload"));
        let raw = miniz_oxide::inflate::decompress_to_vec_zlib(&bytes).expect("valid zlib");
        assert_eq!(raw, [1, 2, 3].repeat(32));
    }

    #[test]
    fn sixel_frames_cover_the_upscaled_image() {
        let mut frame = Frame::new(4, 3);
        frame.fill(Rgb(255, 255, 255));
        let out = sixel(&frame, 2);
        assert!(out.starts_with("\x1b[1;1H\x1bP0;1;0q\"1;1;8;6#0;2;0;0;0"));
        assert!(out.ends_with("-\x1b\\"));
        // 6 rows fit one band; pure white is the last palette entry, every column set.
        let body = out
            .rsplit_once("#251;2;100;100;100")
            .map(|(_, b)| b)
            .expect("palette");
        assert_eq!(body, "#251!8~-\x1b\\");
    }

    #[test]
    fn sixel_dithering_mixes_neighbouring_palette_colors() {
        let mut frame = Frame::new(16, 6);
        frame.fill(Rgb(128, 128, 128));
        let out = sixel(&frame, 1);
        let body = out
            .rsplit_once("#251;2;100;100;100")
            .map(|(_, b)| b)
            .expect("palette");
        assert!(
            body.matches('#').count() >= 2,
            "mid grey should dither between levels: {body}"
        );
    }

    fn base64_decode(text: &str) -> Vec<u8> {
        let value = |c: u8| match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            _ => 63,
        };
        let digits: Vec<u8> = text.bytes().filter(|&c| c != b'=').map(value).collect();
        digits
            .chunks(4)
            .flat_map(|c| {
                let n = c
                    .iter()
                    .enumerate()
                    .fold(0u32, |n, (i, &d)| n | u32::from(d) << (18 - 6 * i));
                (0..c.len() - 1).map(move |i| (n >> (16 - 8 * i)) as u8)
            })
            .collect()
    }
}
