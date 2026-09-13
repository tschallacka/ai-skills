// MODE: DEV
// PACKAGE: PROD
//! Pane geometry -- ported in spirit from installer/src/35-ui-model.sh's
//! iui_layout/iui_list_width/iui_head_geometry. Only the "left-bottom"
//! mascot placement is ported (an ordinary terminal, sprite pinned under
//! the list); "right-top" existed to make room for the hint carousel,
//! which this slice does not have.

pub struct Layout {
    pub cols: usize,
    pub rows: usize,
    pub narrow: bool,
    pub left_w: usize,
    pub right_w: usize,
    pub body_rows: usize,
    /// True when there is room for the sprite under the list: a wide
    /// (non-narrow) layout, a color-capable terminal, and enough spare rows
    /// that the list itself keeps a real floor.
    pub mascot_on: bool,
    /// The list's own row budget: equal to `body_rows` unless the mascot
    /// claims the bottom `MASCOT_RESERVED_ROWS` of it.
    pub list_rows: usize,
}

const LIST_MIN_W: usize = 22;
const LIST_MAX_W: usize = 46;
const DETAIL_MIN_W: usize = 30;
/// The sprite's own 16 rows (mascot::HEIGHT) plus one separator line above it.
const MASCOT_RESERVED_ROWS: usize = 17;
/// The list must keep at least this many rows of its own, or the mascot is
/// dropped rather than starving the content a reader came for -- same
/// reasoning as install.sh's IUI_HEAD_MIN_LIST_ROWS.
const MASCOT_MIN_LIST_ROWS: usize = 6;

/// The list is sized to its content -- a row is cursor(1) + checkbox(3) +
/// space(1) + name -- so the longest skill name decides the width and
/// nothing truncates, same rule as iui_list_width.
fn list_width(cols: usize, skill_names: &[&str]) -> usize {
    let longest = skill_names.iter().map(|n| n.len()).max().unwrap_or(0);
    let mut width = (longest + 6).clamp(LIST_MIN_W, LIST_MAX_W);
    let ceiling = cols.saturating_sub(3 + DETAIL_MIN_W);
    if width > ceiling {
        width = ceiling;
    }
    width.max(8)
}

/// Rows: 1 title, 1 top border, body rows, 1 bottom border, 1 hint.
/// Columns: 1 vertical + left + 1 divider + right + 1 vertical.
pub fn compute(cols: usize, rows: usize, skill_names: &[&str], color_capable: bool) -> Layout {
    let body_rows = rows.saturating_sub(4).max(1);
    if cols < 56 {
        let left_w = (cols.saturating_sub(2)).max(8);
        return Layout {
            cols,
            rows,
            narrow: true,
            left_w,
            right_w: left_w,
            body_rows,
            mascot_on: false,
            list_rows: body_rows,
        };
    }
    let left_w = list_width(cols, skill_names);
    let right_w = cols.saturating_sub(left_w + 3);
    let mascot_on = color_capable && body_rows >= MASCOT_RESERVED_ROWS + MASCOT_MIN_LIST_ROWS;
    let list_rows = if mascot_on {
        body_rows - MASCOT_RESERVED_ROWS
    } else {
        body_rows
    };
    Layout {
        cols,
        rows,
        narrow: false,
        left_w,
        right_w,
        body_rows,
        mascot_on,
        list_rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_narrow_terminal_uses_one_pane() {
        let layout = compute(40, 24, &["todo"], true);
        assert!(layout.narrow);
        assert_eq!(layout.left_w, layout.right_w);
    }

    #[test]
    fn the_list_pane_widens_for_a_long_name() {
        let layout = compute(120, 24, &["post-implementation-review"], true);
        assert!(!layout.narrow);
        assert!(layout.left_w >= "post-implementation-review".len());
    }

    #[test]
    fn the_list_pane_never_starves_the_detail_pane() {
        let layout = compute(60, 24, &["post-implementation-review"], true);
        assert!(layout.right_w >= DETAIL_MIN_W);
        assert_eq!(layout.left_w + layout.right_w + 3, 60);
    }

    #[test]
    fn body_rows_is_always_at_least_one() {
        let layout = compute(80, 5, &["a"], true);
        assert!(layout.body_rows >= 1);
    }

    #[test]
    fn a_tall_color_capable_terminal_gets_the_mascot() {
        let layout = compute(80, 30, &["a"], true);
        assert!(layout.mascot_on);
        assert_eq!(layout.list_rows, layout.body_rows - MASCOT_RESERVED_ROWS);
    }

    #[test]
    fn a_terminal_with_no_color_never_gets_the_mascot() {
        let layout = compute(80, 30, &["a"], false);
        assert!(!layout.mascot_on);
        assert_eq!(layout.list_rows, layout.body_rows);
    }

    #[test]
    fn a_short_terminal_drops_the_mascot_rather_than_starve_the_list() {
        let layout = compute(80, 20, &["a"], true);
        assert!(!layout.mascot_on);
        assert_eq!(layout.list_rows, layout.body_rows);
    }

    #[test]
    fn narrow_layouts_never_show_the_mascot_even_with_color() {
        let layout = compute(40, 40, &["a"], true);
        assert!(!layout.mascot_on);
    }
}
