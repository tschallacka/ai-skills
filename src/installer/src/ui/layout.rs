// MODE: DEV
// PACKAGE: PROD
//! Pane geometry -- ported in spirit from installer/src/35-ui-model.sh's
//! iui_layout/iui_list_width. No sprite geometry (iui_head_geometry): this
//! slice has no mascot to make room for.

pub struct Layout {
    pub cols: usize,
    pub rows: usize,
    pub narrow: bool,
    pub left_w: usize,
    pub right_w: usize,
    pub body_rows: usize,
}

const LIST_MIN_W: usize = 22;
const LIST_MAX_W: usize = 46;
const DETAIL_MIN_W: usize = 30;

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
pub fn compute(cols: usize, rows: usize, skill_names: &[&str]) -> Layout {
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
        };
    }
    let left_w = list_width(cols, skill_names);
    let right_w = cols.saturating_sub(left_w + 3);
    Layout {
        cols,
        rows,
        narrow: false,
        left_w,
        right_w,
        body_rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_narrow_terminal_uses_one_pane() {
        let layout = compute(40, 24, &["todo"]);
        assert!(layout.narrow);
        assert_eq!(layout.left_w, layout.right_w);
    }

    #[test]
    fn the_list_pane_widens_for_a_long_name() {
        let layout = compute(120, 24, &["post-implementation-review"]);
        assert!(!layout.narrow);
        assert!(layout.left_w >= "post-implementation-review".len());
    }

    #[test]
    fn the_list_pane_never_starves_the_detail_pane() {
        let layout = compute(60, 24, &["post-implementation-review"]);
        assert!(layout.right_w >= DETAIL_MIN_W);
        assert_eq!(layout.left_w + layout.right_w + 3, 60);
    }

    #[test]
    fn body_rows_is_always_at_least_one() {
        let layout = compute(80, 5, &["a"]);
        assert!(layout.body_rows >= 1);
    }
}
