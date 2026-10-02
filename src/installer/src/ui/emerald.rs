// MODE: DEV
// PACKAGE: PROD
//! Minecraft-style emeralds raining down the progress screen's idle area,
//! each spinning about its own vertical axis. Drawn like the mascot: one
//! pixel is a solid block two columns wide. Positions are a pure function of
//! elapsed time, so the rain needs no state beyond its clock and survives a
//! resize without resyncing.

use super::mascot::{self, ColorMode};

pub(crate) type Rgb = (u8, u8, u8);

/// One emerald: `O` outline, `H` highlight, `L` light, `M` mid, `D` dark.
const SPRITE: [&str; 7] = [
    "..O..", ".OHO.", "OLMMO", "OHMDO", "OMMDO", ".ODO.", "..O..",
];
const SPRITE_W: usize = 5;
const SPRITE_H: usize = SPRITE.len();
/// Terminal columns per sprite pixel, the same as a mascot pixel.
const PIXEL_COLS: usize = 2;
/// The most emeralds ever falling at once, however wide the pane.
const MAX_DROPS: usize = 16;
/// Terminal columns of pane per falling emerald.
const COLS_PER_DROP: usize = 16;

fn pixel_color(code: char) -> Option<Rgb> {
    match code {
        'O' => Some((0x0b, 0x4d, 0x26)),
        'D' => Some((0x12, 0x88, 0x3e)),
        'M' => Some((0x1d, 0xc4, 0x5c)),
        'L' => Some((0x5c, 0xf0, 0x92)),
        'H' => Some((0xd8, 0xff, 0xe6)),
        _ => None,
    }
}

/// The sprite at spin `phase` (0..1 is a full turn), in pixels: squeezed by
/// |cos| about the vertical axis and mirrored on the far side of the turn.
pub(crate) fn frame(phase: f32) -> Vec<Vec<Option<Rgb>>> {
    let c = (phase * std::f32::consts::TAU).cos();
    let visible = ((SPRITE_W as f32 * c.abs()).round() as usize).clamp(1, SPRITE_W);
    let left = (SPRITE_W - visible) / 2;
    SPRITE
        .iter()
        .map(|row| {
            let src: Vec<char> = row.chars().collect();
            (0..SPRITE_W)
                .map(|x| {
                    let k = x.checked_sub(left).filter(|k| *k < visible)?;
                    let mut s = (k * 2 + 1) * SPRITE_W / (visible * 2);
                    if c < 0.0 {
                        s = SPRITE_W - 1 - s;
                    }
                    pixel_color(src[s])
                })
                .collect()
        })
        .collect()
}

#[derive(Clone, Copy)]
struct Drop {
    column: f32,
    offset: f32,
    speed: f32,
    phase: f32,
    spin: f32,
}

/// The falling emeralds: fixed per-drop parameters plus a clock.
pub(crate) struct Rain {
    drops: [Drop; MAX_DROPS],
    seconds: f32,
}

impl Rain {
    pub(crate) fn new(seed: u64) -> Rain {
        let mut state = seed | 1;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 40) as f32 / (1u64 << 24) as f32
        };
        let drops = std::array::from_fn(|i| Drop {
            column: (i as f32 + 0.15 + 0.7 * next()) / MAX_DROPS as f32,
            offset: next(),
            speed: 3.0 + 4.0 * next(),
            phase: next(),
            spin: 0.25 + 0.5 * next(),
        });
        Rain {
            drops,
            seconds: 0.0,
        }
    }

    pub(crate) fn advance(&mut self, seconds: f32) {
        self.seconds += seconds;
    }

    /// A `height` x `width` grid of terminal cells, `Some` where an emerald
    /// pixel covers that cell.
    pub(crate) fn paint(&self, width: usize, height: usize) -> Vec<Vec<Option<Rgb>>> {
        let mut grid = vec![vec![None; width]; height];
        let sprite_cols = SPRITE_W * PIXEL_COLS;
        if width < sprite_cols || height == 0 {
            return grid;
        }
        let count = (width / COLS_PER_DROP).clamp(1, MAX_DROPS);
        let cycle = (height + SPRITE_H + 2) as f32;
        let mut drops: Vec<&Drop> = self.drops.iter().collect();
        drops.sort_by(|a, b| a.column.total_cmp(&b.column));
        let stride = MAX_DROPS / count;
        for (n, drop) in drops.iter().step_by(stride).take(count).enumerate() {
            let slot = (width - sprite_cols) as f32 / count as f32;
            let left = ((n as f32 + drop.column.fract()) * slot) as usize;
            let fallen = (drop.offset * cycle + drop.speed * self.seconds) % cycle;
            let top = fallen as isize - SPRITE_H as isize;
            let sprite = frame(drop.phase + drop.spin * self.seconds);
            for (dy, row) in sprite.iter().enumerate() {
                let y = top + dy as isize;
                if y < 0 || y as usize >= height {
                    continue;
                }
                for (dx, pixel) in row.iter().enumerate() {
                    if let Some(rgb) = pixel {
                        for c in 0..PIXEL_COLS {
                            let x = left + dx * PIXEL_COLS + c;
                            if x < width {
                                grid[y as usize][x] = Some(*rgb);
                            }
                        }
                    }
                }
            }
        }
        grid
    }
}

/// One grid row as a terminal line: exactly `cells.len()` columns, colored
/// cells drawn as solid blocks (`#` without UTF-8), the rest blank.
pub(crate) fn render_row(cells: &[Option<Rgb>], mode: ColorMode, unicode: bool) -> String {
    let glyph = if unicode { '\u{2588}' } else { '#' };
    let mut out = String::new();
    let mut current: Option<Rgb> = None;
    for cell in cells {
        match cell {
            Some(rgb) => {
                if current != Some(*rgb) {
                    out.push_str(&mascot::fg_sgr(mode, *rgb));
                    current = Some(*rgb);
                }
                out.push(glyph);
            }
            None => {
                if current.is_some() {
                    out.push_str("\x1b[0m");
                    current = None;
                }
                out.push(' ');
            }
        }
    }
    if current.is_some() {
        out.push_str("\x1b[0m");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visible_width(frame: &[Vec<Option<Rgb>>]) -> usize {
        (0..SPRITE_W)
            .filter(|x| frame.iter().any(|row| row[*x].is_some()))
            .count()
    }

    #[test]
    fn an_emerald_spins_from_face_on_to_edge_on_and_back_mirrored() {
        let face = frame(0.0);
        let edge = frame(0.25);
        let back = frame(0.5);
        assert_eq!(visible_width(&face), SPRITE_W);
        assert_eq!(visible_width(&edge), 1);
        assert_eq!(visible_width(&back), SPRITE_W);
        // Seen from behind, the highlight is on the other side.
        assert_eq!(face[3][1], pixel_color('H'));
        assert_eq!(back[3][3], pixel_color('H'));
    }

    #[test]
    fn the_rain_fills_its_area_exactly_and_moves_with_time() {
        let mut rain = Rain::new(7);
        let before = rain.paint(120, 30);
        assert_eq!(before.len(), 30);
        assert!(before.iter().all(|row| row.len() == 120));
        assert!(
            before.iter().flatten().any(|c| c.is_some()),
            "no emerald drawn"
        );
        rain.advance(0.5);
        assert_ne!(before, rain.paint(120, 30), "the rain did not move");
    }

    #[test]
    fn a_rendered_row_is_exactly_its_width_in_columns() {
        let rain = Rain::new(3);
        for row in rain.paint(80, 20) {
            let line = render_row(&row, ColorMode::TrueColor, true);
            let mut cols = 0;
            let mut chars = line.chars();
            while let Some(c) = chars.next() {
                if c == '\u{1b}' {
                    chars.by_ref().find(|c| c.is_ascii_alphabetic());
                } else {
                    cols += 1;
                }
            }
            assert_eq!(cols, 80, "{line:?}");
        }
    }
}
