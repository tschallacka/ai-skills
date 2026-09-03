// MODE: DEV
// PACKAGE: PROD
use serde::{Deserialize, Serialize};
use unicode_normalization::char::is_combining_mark;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Cursor {
    pub id: u64,
    pub position: Position,
}

pub fn line_lengths(text: &str) -> Vec<usize> {
    let mut lengths: Vec<usize> = text
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).chars().count())
        .collect();
    if lengths.is_empty() {
        lengths.push(0);
    }
    lengths
}

pub fn clamp(text: &str, position: Position) -> Position {
    let lengths = line_lengths(text);
    let line = position.line.max(1).min(lengths.len());
    Position {
        line,
        column: position.column.min(lengths[line - 1]),
    }
}

pub fn home(text: &str, position: Position) -> Position {
    Position {
        line: clamp(text, position).line,
        column: 0,
    }
}
pub fn end(text: &str, position: Position) -> Position {
    let position = clamp(text, position);
    Position {
        line: position.line,
        column: line_lengths(text)[position.line - 1],
    }
}

pub fn page(text: &str, position: Position, lines: usize, direction: i32) -> Position {
    let position = clamp(text, position);
    let target = if direction < 0 {
        position.line.saturating_sub(lines).max(1)
    } else {
        position
            .line
            .saturating_add(lines)
            .min(line_lengths(text).len())
    };
    clamp(
        text,
        Position {
            line: target,
            column: position.column,
        },
    )
}

pub fn next_word(text: &str, position: Position) -> Position {
    move_word(text, clamp(text, position), true)
}
pub fn previous_word(text: &str, position: Position) -> Position {
    move_word(text, clamp(text, position), false)
}

pub fn visual_position(
    text: &str,
    position: Position,
    wrap_width: Option<usize>,
) -> Result<Position, &'static str> {
    let width = wrap_width.ok_or("wrap_width_required")?;
    if width == 0 {
        return Err("wrap_width_required");
    }
    let position = clamp(text, position);
    let lines: Vec<Vec<char>> = text
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).chars().collect())
        .collect();
    let preceding_rows: usize = lines[..position.line - 1]
        .iter()
        .map(|line| line.len().max(1).div_ceil(width))
        .sum();
    Ok(Position {
        line: preceding_rows + position.column / width + 1,
        column: position.column % width,
    })
}

pub fn logical_position(
    text: &str,
    visual: Position,
    wrap_width: Option<usize>,
) -> Result<Position, &'static str> {
    let width = wrap_width.ok_or("wrap_width_required")?;
    if width == 0 || visual.line == 0 {
        return Err("wrap_width_required");
    }
    let lines: Vec<Vec<char>> = text
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).chars().collect())
        .collect();
    let mut row = 1usize;
    for (line_index, chars) in lines.iter().enumerate() {
        let rows = chars.len().max(1).div_ceil(width);
        if visual.line < row + rows {
            return Ok(Position {
                line: line_index + 1,
                column: ((visual.line - row) * width + visual.column).min(chars.len()),
            });
        }
        row += rows;
    }
    Ok(Position {
        line: lines.len().max(1),
        column: lines.last().map_or(0, Vec::len),
    })
}

fn move_word(text: &str, position: Position, forward: bool) -> Position {
    let lines: Vec<Vec<char>> = text
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).chars().collect())
        .collect();
    let mut index = position.column;
    let line = position.line - 1;
    let chars = &lines[line];
    if forward {
        while index < chars.len() && is_word(chars[index]) {
            index += 1;
        }
        while index < chars.len() && !is_word(chars[index]) {
            index += 1;
        }
        Position {
            line: position.line,
            column: index,
        }
    } else {
        index = index.min(chars.len());
        index = index.saturating_sub(1);
        while index > 0 && !is_word(chars[index]) {
            index -= 1;
        }
        while index > 0 && is_word(chars[index - 1]) {
            index -= 1;
        }
        Position {
            line: position.line,
            column: index,
        }
    }
}

fn is_word(character: char) -> bool {
    character.is_alphabetic()
        || character.is_numeric()
        || is_combining_mark(character)
        || character.is_ascii_punctuation() && character == '_'
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_and_crlf_positions_are_scalar_based() {
        let text = "héllo\r\nworld";
        assert_eq!(line_lengths(text), vec![5, 5]);
        assert_eq!(
            end(
                text,
                Position {
                    line: 1,
                    column: 99
                }
            ),
            Position { line: 1, column: 5 }
        );
    }
    #[test]
    fn word_and_page_navigation_clamp() {
        let text = "one, two\nthree";
        assert_eq!(
            next_word(text, Position { line: 1, column: 0 }),
            Position { line: 1, column: 5 }
        );
        assert_eq!(
            previous_word(text, Position { line: 1, column: 7 }),
            Position { line: 1, column: 5 }
        );
        assert_eq!(page(text, Position { line: 1, column: 0 }, 40, -1).line, 1);
    }
    #[test]
    fn wrapping_requires_positive_width() {
        let text = "abcdef";
        assert_eq!(
            visual_position(text, Position { line: 1, column: 4 }, None),
            Err("wrap_width_required")
        );
        assert_eq!(
            visual_position(text, Position { line: 1, column: 4 }, Some(3)).unwrap(),
            Position { line: 2, column: 1 }
        );
        let text = "abcde\nf";
        assert_eq!(
            visual_position(text, Position { line: 2, column: 0 }, Some(3)).unwrap(),
            Position { line: 3, column: 0 }
        );
        assert_eq!(
            logical_position(text, Position { line: 2, column: 1 }, Some(3)).unwrap(),
            Position { line: 1, column: 4 }
        );
    }
}
