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
                other => Err(format!(
                    "{OVERRIDE_VAR}={other} is not one of auto, blocks, kitty, iterm."
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
        } else {
            Self::Blocks
        }
    }
}

/// Kitty protocol: transmit raw RGB and place it over `cols` x `rows` cells at
/// the top-left. Images alternate between two ids so the new frame is on
/// screen before the old one is deleted, which avoids flicker.
pub fn kitty(frame: &Frame, cols: u16, rows: u16, id: u32) -> String {
    let mut rgb = Vec::with_capacity(frame.pixels().len() * 3);
    for p in frame.pixels() {
        rgb.extend_from_slice(&[p.0, p.1, p.2]);
    }
    let data = base64(&rgb);
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
                "\x1b_Ga=T,f=24,s={},v={},c={cols},r={rows},i={id},p=1,q=2,C=1,m={more};{chunk}\x1b\\",
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

/// Encodes the frame as an RGB PNG. Uses uncompressed deflate blocks: larger
/// than a real compressor, but simple, fast, and dependency-free.
pub fn png(frame: &Frame) -> Vec<u8> {
    let mut raw = Vec::with_capacity((frame.width * 3 + 1) * frame.height);
    for row in frame.pixels().chunks(frame.width.max(1)) {
        raw.push(0); // filter: none
        for p in row {
            raw.extend_from_slice(&[p.0, p.1, p.2]);
        }
    }
    let mut zlib = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (i, block) in blocks.iter().enumerate() {
        zlib.push(u8::from(i + 1 == blocks.len()));
        let len = block.len() as u16;
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    if blocks.is_empty() {
        zlib.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());

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

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in data.chunks(5552) {
        for &byte in chunk {
            a += u32::from(byte);
            b += a;
        }
        a %= 65_521;
        b %= 65_521;
    }
    b << 16 | a
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
        assert!(Graphics::detect(env(&[(OVERRIDE_VAR, "sixel")])).is_err());
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
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
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
        // Raw scanlines are stored verbatim: filter byte then RGB.
        let raw = [0, 10, 20, 30, 10, 20, 30, 10, 20, 30];
        assert!(png.windows(raw.len()).any(|w| w == raw));
    }

    #[test]
    fn kitty_frames_are_chunked_and_swap_ids() {
        let frame = Frame::new(64, 48);
        let out = kitty(&frame, 32, 12, 1);
        assert!(out.starts_with("\x1b[1;1H\x1b_Ga=T,f=24,s=64,v=48,c=32,r=12,i=1,"));
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
}
