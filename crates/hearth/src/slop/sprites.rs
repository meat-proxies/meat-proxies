//! Pixel art for Slop Runner. Each sprite is a grid of palette keys; `.` is transparent.

use super::render::Rgb;

pub type Sprite = &'static [&'static str];

/// Maps a palette key to its 24-bit color. Unknown keys are transparent.
pub fn color(key: u8) -> Option<Rgb> {
    Some(match key {
        // Outline and runner.
        b'k' => Rgb(43, 26, 22),
        b'h' => Rgb(112, 58, 40),
        b'H' => Rgb(166, 96, 62),
        b's' => Rgb(246, 206, 176),
        b'S' => Rgb(222, 164, 134),
        b'e' => Rgb(38, 30, 52),
        b'r' => Rgb(240, 140, 130),
        b'o' => Rgb(217, 119, 87),
        b'O' => Rgb(170, 82, 56),
        b'L' => Rgb(248, 170, 134),
        b'w' => Rgb(244, 240, 230),
        b'W' => Rgb(206, 194, 174),
        b'p' => Rgb(62, 58, 56),
        b'P' => Rgb(104, 98, 96),
        b'b' => Rgb(32, 30, 32),
        b'g' => Rgb(255, 205, 96),
        b'G' => Rgb(255, 240, 170),
        // Slop goo.
        b'm' => Rgb(150, 138, 104),
        b'M' => Rgb(196, 184, 146),
        b'n' => Rgb(102, 92, 70),
        b'q' => Rgb(236, 228, 206),
        b'x' => Rgb(126, 170, 96),
        // Uncanny hand.
        b'f' => Rgb(232, 180, 160),
        b'F' => Rgb(196, 134, 118),
        b'i' => Rgb(250, 226, 214),
        // Listicle tower.
        b'R' => Rgb(222, 62, 70),
        b'y' => Rgb(250, 208, 64),
        b'u' => Rgb(64, 140, 230),
        b'U' => Rgb(40, 92, 170),
        b'z' => Rgb(255, 255, 255),
        // Em-dash swarm.
        b'c' => Rgb(118, 220, 236),
        b'C' => Rgb(206, 246, 252),
        b'd' => Rgb(58, 120, 150),
        _ => return None,
    })
}

pub const RUN_A: Sprite = &[
    "......kkkkk...",
    ".kk..khHHhhk..",
    "kook.khhhhhhk.",
    "kookkhHhhhhhhk",
    "kkhhkhhhhhhhhk",
    "khhkoohhksssek",
    "khhkookhksssek",
    "khhkhhhhksssk.",
    "khhhkhhhksrsk.",
    ".khhkkkkkSsk..",
    ".khhkwwwwwwk..",
    "..khkwwwwwwksk",
    "..kkoooooookk.",
    "...kwwwwwwwk..",
    "..kwwwwwwwWWk.",
    "..kooooooooOk.",
    "...kppk.kppk..",
    "..kbbbk..kbbbk",
];

pub const RUN_B: Sprite = &[
    "......kkkkk...",
    ".kk..khHHhhk..",
    "kook.khhhhhhk.",
    "kookkhHhhhhhhk",
    "kkhhkhhhhhhhhk",
    ".khkoohhksssek",
    "khhkookhksssek",
    "khhkhhhhksssk.",
    ".khhkhhhksrsk.",
    ".khhkkkkkSsk..",
    "..khkwwwwwwk..",
    "..kkkwwwwwwk..",
    "..kkoooooooksk",
    "...kwwwwwwwkk.",
    "..kwwwwwwwWWk.",
    "..kooooooooOk.",
    "....kpppPk....",
    "...kbbbkbbbk..",
];

pub const JUMP: Sprite = &[
    "kk....kkkkk...",
    "kookkkhHHhhk..",
    "kookhhhhhhhhk.",
    ".kkhhhhHhhhhhk",
    "khhhhoohhhhhhk",
    "kkkkkookksssek",
    ".khhhhhhksssek",
    "..kkkhhhksrsk.",
    ".ksk.kkkkSsk..",
    "..kkkwwwwwwksk",
    "....koooooookk",
    "...kwwwwwwwk..",
    "..kwwwwwwwWWk.",
    "..kooooooooOk.",
    "..kppPkkkppk..",
    "..kbbbk.kbbbk.",
    "...kkk...kkk..",
];

pub const DUCK: Sprite = &[
    "............kkkkk.",
    "..kk.......khHHhhk",
    ".kook.kkk.khHhhhhk",
    "kkookkhhhkhhhhhhhk",
    "khhhhhhhhhkoohsssk",
    "kkkhhhhhhkkookssek",
    "..kkkwwwwwkhhksrsk",
    ".kwwwoooooookkkkk.",
    "kwwwwwwwwWWksk....",
    "kooooooOkpppkk....",
    ".kbbbk..kbbbk.....",
];

pub const SLOPPED: Sprite = &[
    "......kkkkk...",
    ".kk..kmMMmmk..",
    "kook.kmqMmmmk.",
    "kookkhmmhmhmhk",
    "kkhhkhmhhhmhhk",
    "khhkoohhksksek",
    "khhkookhkskskk",
    "khhkhhhhksssk.",
    "khhhkhhhksssk.",
    ".khhkkkkkmmk..",
    ".khhkwwwmmwk..",
    "..khkwwmMmwksk",
    "..kkooommoook.",
    "...kwwwmmwwk..",
    "..kwwwwwwwWWk.",
    "..kooooooooOk.",
    "...kppk.kppk..",
    "..kbbbk..kbbbk",
];

/// A puddle of generic beige content, still warm from the generator.
pub const PUDDLE_A: Sprite = &[
    "......nnnnn.........",
    "....nnmMMMmnn..q....",
    "..nnmMqqMMmmmnn.....",
    ".nmMMMqMMMmmxmmn..q.",
    "nmMMMMMMmmmmmmmmmnn.",
    "nmmmmmmmmmxmmmmmmmmn",
    ".nnnnnnnnnnnnnnnnnn.",
];

pub const PUDDLE_B: Sprite = &[
    ".....nnnnnn.......q.",
    "...nnmMMMMmnn.......",
    "..nmMqqMMMmmmnn..q..",
    ".nmMMqqMMMmmmxmn....",
    "nmMMMMMMMmmmmmmmmnn.",
    "nmmmmmxmmmmmmmmmmmmn",
    ".nnnnnnnnnnnnnnnnnn.",
];

/// Six fingers, reaching up out of a generated image.
pub const HAND: Sprite = &[
    ".....kkkk.....",
    "...kkiFiFkk...",
    "..kiFfFfFiFk..",
    "..kfFfFfFfFkk.",
    ".kkfFfFfFfFiFk",
    "kiFfFfFfFfFfFk",
    "kfFfFfFfFfFfFk",
    "kfFfFfFfFfFfFk",
    "kfFfFfFfFfFfFk",
    "kfFfFfFfFfFfFk",
    "kfffifffiffFFk",
    "kffffffffffFFk",
    "kffffffffffFFk",
    "kffffffffffFFk",
    "..kffffffFFk..",
    "..kffffffFFk..",
    "..kffffffFFk..",
    "..kkkkkkkkkk..",
];

/// Three stacked listicles. Number seven will shock you.
pub const TOWER: Sprite = &[
    "....kkkk....",
    "...kyyyyk...",
    "kkkkkyykkkkk",
    "kRRRRRRRRRRk",
    "kRzRRzzzRzRk",
    "kRzRRRzRRzRk",
    "kRRRRRRRRRRk",
    "kkkkkkkkkkkk",
    "kyyyyyyyyyyk",
    "kyzzzyyzyzyk",
    "kyyyyyyyyyyk",
    "kkkkkkkkkkkk",
    "kuuuuuuuuuuk",
    "kuzzuzzzuzuk",
    "kuuuuuuuuuuk",
    "kUUUUUUUUUUk",
    "kkkkkkkkkkkk",
];

/// A swarm of em dashes, flying in formation.
pub const DASHES_A: Sprite = &[
    "..........dd.........",
    ".dCCCCCCd...ddd......",
    ".dccccccd..dCCCCCCd..",
    "..ddddd....dccccccd..",
    ".....dCCCCCCd.ddddd..",
    ".....dccccccd........",
];

pub const DASHES_B: Sprite = &[
    ".dCCCCCCd............",
    ".dccccccd..dCCCCCCd..",
    "...ddd.....dccccccd..",
    "......dd....dddd.....",
    ".....dCCCCCCd........",
    ".....dccccccd........",
];

pub const SPARK_A: Sprite = &[
    "..g..", //
    ".gGg.", //
    "gGGGg", //
    ".gGg.", //
    "..g..", //
];

pub const SPARK_B: Sprite = &[
    ".....", //
    "..g..", //
    ".gGg.", //
    "..g..", //
    ".....", //
];

pub fn width(sprite: Sprite) -> usize {
    sprite.first().map_or(0, |row| row.len())
}

pub fn height(sprite: Sprite) -> usize {
    sprite.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: &[(&str, Sprite)] = &[
        ("RUN_A", RUN_A),
        ("RUN_B", RUN_B),
        ("JUMP", JUMP),
        ("DUCK", DUCK),
        ("SLOPPED", SLOPPED),
        ("PUDDLE_A", PUDDLE_A),
        ("PUDDLE_B", PUDDLE_B),
        ("HAND", HAND),
        ("TOWER", TOWER),
        ("DASHES_A", DASHES_A),
        ("DASHES_B", DASHES_B),
        ("SPARK_A", SPARK_A),
        ("SPARK_B", SPARK_B),
    ];

    #[test]
    fn sprites_are_rectangular_and_use_known_colors() {
        for (name, sprite) in ALL {
            assert!(!sprite.is_empty(), "{name} is empty");
            for (y, row) in sprite.iter().enumerate() {
                assert_eq!(
                    row.len(),
                    width(sprite),
                    "{name} row {y} has the wrong width"
                );
                for key in row.bytes().filter(|&key| key != b'.') {
                    assert!(
                        color(key).is_some(),
                        "{name} row {y} uses unknown key {}",
                        key as char
                    );
                }
            }
        }
    }

    #[test]
    fn animation_frames_share_a_size() {
        for (a, b) in [
            (RUN_A, RUN_B),
            (PUDDLE_A, PUDDLE_B),
            (DASHES_A, DASHES_B),
            (SPARK_A, SPARK_B),
        ] {
            assert_eq!((width(a), height(a)), (width(b), height(b)));
        }
    }
}
