// MODE: DEV
// PACKAGE: PROD
//! Pane geometry. Only the "left-bottom" mascot placement exists here (an
//! ordinary terminal, sprite pinned under the list); a "right-top"
//! placement for a hint carousel is out of scope for this slice.

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
/// dropped rather than starving the content a reader came for.
const MASCOT_MIN_LIST_ROWS: usize = 6;

/// The list is sized to its content -- a row is cursor(1) + checkbox(3) +
/// space(1) + name -- so the longest skill name decides the width and
/// nothing truncates.
fn list_width(cols: usize, skill_names: &[&str]) -> usize {
    let longest = skill_names.iter().map(|n| n.len()).max().unwrap_or(0);
    let mut width = (longest + 6).clamp(LIST_MIN_W, LIST_MAX_W);
    let ceiling = cols.saturating_sub(3 + DETAIL_MIN_W);
    if width > ceiling {
        width = ceiling;
    }
    width.max(8)
}

/// Rows: `title_rows` title, 1 top border, body rows, 1 bottom border,
/// `hint_rows` hint. Both are usually 1 -- the title/hint text fits one
/// line at most terminal widths -- but at a narrow enough width either bar
/// wraps onto more (via `text::overflow`), and the body must shrink to make
/// room rather than let the frame grow past `rows`. Callers compute the
/// actual wrapped line count for the title text they are about to show (it
/// varies with the installed/selected counts) and the fixed hint text, at
/// this same `cols`, and pass them in -- `layout` itself does not lay out
/// text, only reserves the rows it will need.
/// Columns: 1 vertical + left + 1 divider + right + 1 vertical.
pub fn compute(
    cols: usize,
    rows: usize,
    skill_names: &[&str],
    color_capable: bool,
    title_rows: usize,
    hint_rows: usize,
) -> Layout {
    let reserved = 2 + title_rows.max(1) + hint_rows.max(1);
    let body_rows = rows.saturating_sub(reserved).max(1);
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
        let layout = compute(40, 24, &["todo"], true, 1, 1);
        assert!(layout.narrow);
        assert_eq!(layout.left_w, layout.right_w);
    }

    #[test]
    fn the_list_pane_widens_for_a_long_name() {
        let layout = compute(120, 24, &["post-implementation-review"], true, 1, 1);
        assert!(!layout.narrow);
        assert!(layout.left_w >= "post-implementation-review".len());
    }

    #[test]
    fn the_list_pane_never_starves_the_detail_pane() {
        let layout = compute(60, 24, &["post-implementation-review"], true, 1, 1);
        assert!(layout.right_w >= DETAIL_MIN_W);
        assert_eq!(layout.left_w + layout.right_w + 3, 60);
    }

    #[test]
    fn body_rows_is_always_at_least_one() {
        let layout = compute(80, 5, &["a"], true, 1, 1);
        assert!(layout.body_rows >= 1);
    }

    #[test]
    fn a_tall_color_capable_terminal_gets_the_mascot() {
        let layout = compute(80, 30, &["a"], true, 1, 1);
        assert!(layout.mascot_on);
        assert_eq!(layout.list_rows, layout.body_rows - MASCOT_RESERVED_ROWS);
    }

    #[test]
    fn a_terminal_with_no_color_never_gets_the_mascot() {
        let layout = compute(80, 30, &["a"], false, 1, 1);
        assert!(!layout.mascot_on);
        assert_eq!(layout.list_rows, layout.body_rows);
    }

    #[test]
    fn a_short_terminal_drops_the_mascot_rather_than_starve_the_list() {
        let layout = compute(80, 20, &["a"], true, 1, 1);
        assert!(!layout.mascot_on);
        assert_eq!(layout.list_rows, layout.body_rows);
    }

    #[test]
    fn narrow_layouts_never_show_the_mascot_even_with_color() {
        let layout = compute(40, 40, &["a"], true, 1, 1);
        assert!(!layout.mascot_on);
    }

    #[test]
    fn a_wrapped_title_or_hint_bar_shrinks_the_body_to_make_room() {
        let base = compute(80, 24, &["a"], true, 1, 1);
        let wrapped_title = compute(80, 24, &["a"], true, 2, 1);
        let wrapped_hint = compute(80, 24, &["a"], true, 1, 3);
        assert_eq!(wrapped_title.body_rows, base.body_rows - 1);
        assert_eq!(wrapped_hint.body_rows, base.body_rows - 2);
        // The frame's total row budget is otherwise unchanged: whatever a
        // wrapped bar costs the body, it does not cost the overall `rows`.
        assert_eq!(wrapped_title.rows, base.rows);
    }

    #[test]
    fn a_zero_title_or_hint_row_count_is_treated_as_at_least_one() {
        // Callers should never pass 0 (both bars always render at least one
        // line), but `compute` does not trust that and reserves a floor of
        // 1 each rather than let a caller's bug hand back extra body rows
        // that a real render would then overflow past.
        let with_zero = compute(80, 24, &["a"], true, 0, 0);
        let with_one = compute(80, 24, &["a"], true, 1, 1);
        assert_eq!(with_zero.body_rows, with_one.body_rows);
    }
}
