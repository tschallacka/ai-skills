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
    /// Whether `render` should draw box-drawing borders (`┌─┐│└┘├┤┬┴`)
    /// instead of the plain-ASCII fallback (`+-|`). Carried on `Layout`
    /// rather than read fresh by `render` so a test can force either border
    /// set without touching the terminal-capability probe, the same reason
    /// `color_capable` is a parameter here rather than a global.
    pub unicode_borders: bool,
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
    unicode_borders: bool,
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
            unicode_borders,
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
        unicode_borders,
    }
}

/// What a mouse click landed on, from `hit_test` below.
#[derive(Debug, PartialEq, Eq)]
pub enum ClickTarget {
    /// A row inside the visible list, given as an index into the CURRENTLY
    /// SCROLLED view (0 is whatever row is topmost right now, not
    /// necessarily skill 0) -- the same thing `state.scroll + row` already
    /// means everywhere else `list_rows` is used, so a caller adds
    /// `state.scroll` to reach an absolute skill index, exactly as
    /// `list_cell` does. `col` is the 0-based content column within the
    /// row (past the pane's own leading border), matching what
    /// `render::list_row` actually draws: column 0 is the cursor marker,
    /// 1..4 the `[x]`/`[ ]` checkbox, the rest the skill's name -- needed so
    /// a click can be tested against the checkbox specifically rather than
    /// "landed somewhere in this row" (B388: clicking anywhere in the row
    /// used to toggle selection, which fought with using a click to move
    /// focus into DETAILS without changing what was checked).
    ListRow { row: usize, col: usize },
    /// A cell inside the details/info pane (only reachable at all in a wide
    /// layout, or a narrow one currently showing it), given as the 0-based
    /// row and column WITHIN THE PANE'S OWN CONTENT -- `row` is relative to
    /// the currently scrolled view the same way `ListRow`'s is (a caller
    /// adds `state.info_scroll` to reach an absolute line index into
    /// `render::info_lines`), and `col` is the content column past the
    /// pane's own leading border, matching what `render::info_layout`'s
    /// button/toggle column ranges are computed against. Needed so a click
    /// can be tested against the ACTIONS buttons and the integration-mode
    /// toggle's own segments, not just "landed somewhere in this pane".
    Info { row: usize, col: usize },
}

/// Maps a 1-based (col, row) mouse report onto what it landed on, or `None`
/// for chrome (a border, a divider, the title/hint bars) that has nothing to
/// click. `title_rows` is the caller's own already-computed line count for
/// the title bar at this frame's width (the same value `compute` itself took
/// to reserve body rows), since that is what fixes where the top border --
/// and so the body -- actually starts on screen. `info_focused` matters only
/// in a narrow layout, which shows one pane at a time and swaps which one a
/// body row actually is; a wide layout ignores it, since both panes are
/// always on screen together there.
pub fn hit_test(
    layout: &Layout,
    title_rows: usize,
    info_focused: bool,
    col: u16,
    row: u16,
) -> Option<ClickTarget> {
    if col == 0 || row == 0 {
        return None; // 1-based; 0 never names a real cell
    }
    let col = (col - 1) as usize;
    let row = (row - 1) as usize;
    let body_start = title_rows + 1; // past the title bar(s) and the top border
    if row < body_start {
        return None;
    }
    let body_row = row - body_start;
    if body_row >= layout.body_rows {
        return None;
    }

    if layout.narrow {
        // One pane, framed by the two vertical bars at col 0 and col
        // left_w + 1; content is the columns strictly between them.
        if col == 0 || col > layout.left_w {
            return None;
        }
        return if info_focused {
            Some(ClickTarget::Info {
                row: body_row,
                col: col - 1,
            })
        } else if body_row < layout.list_rows {
            Some(ClickTarget::ListRow {
                row: body_row,
                col: col - 1,
            })
        } else {
            None
        };
    }

    // Wide: | list | info |, three vertical bars at col 0, col left_w + 1,
    // and col left_w + right_w + 2.
    if col == 0 {
        return None;
    }
    let divider = layout.left_w + 1;
    if col < divider {
        return if body_row < layout.list_rows {
            Some(ClickTarget::ListRow {
                row: body_row,
                col: col - 1,
            })
        } else {
            None
        };
    }
    if col == divider {
        return None;
    }
    let right_edge = divider + 1 + layout.right_w;
    if col < right_edge {
        return Some(ClickTarget::Info {
            row: body_row,
            col: col - (divider + 1),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_narrow_terminal_uses_one_pane() {
        let layout = compute(40, 24, &["todo"], true, 1, 1, false);
        assert!(layout.narrow);
        assert_eq!(layout.left_w, layout.right_w);
    }

    #[test]
    fn the_list_pane_widens_for_a_long_name() {
        let layout = compute(120, 24, &["post-implementation-review"], true, 1, 1, false);
        assert!(!layout.narrow);
        assert!(layout.left_w >= "post-implementation-review".len());
    }

    #[test]
    fn the_list_pane_never_starves_the_detail_pane() {
        let layout = compute(60, 24, &["post-implementation-review"], true, 1, 1, false);
        assert!(layout.right_w >= DETAIL_MIN_W);
        assert_eq!(layout.left_w + layout.right_w + 3, 60);
    }

    #[test]
    fn body_rows_is_always_at_least_one() {
        let layout = compute(80, 5, &["a"], true, 1, 1, false);
        assert!(layout.body_rows >= 1);
    }

    #[test]
    fn a_tall_color_capable_terminal_gets_the_mascot() {
        let layout = compute(80, 30, &["a"], true, 1, 1, false);
        assert!(layout.mascot_on);
        assert_eq!(layout.list_rows, layout.body_rows - MASCOT_RESERVED_ROWS);
    }

    #[test]
    fn a_terminal_with_no_color_never_gets_the_mascot() {
        let layout = compute(80, 30, &["a"], false, 1, 1, false);
        assert!(!layout.mascot_on);
        assert_eq!(layout.list_rows, layout.body_rows);
    }

    #[test]
    fn a_short_terminal_drops_the_mascot_rather_than_starve_the_list() {
        let layout = compute(80, 20, &["a"], true, 1, 1, false);
        assert!(!layout.mascot_on);
        assert_eq!(layout.list_rows, layout.body_rows);
    }

    #[test]
    fn narrow_layouts_never_show_the_mascot_even_with_color() {
        let layout = compute(40, 40, &["a"], true, 1, 1, false);
        assert!(!layout.mascot_on);
    }

    #[test]
    fn a_wrapped_title_or_hint_bar_shrinks_the_body_to_make_room() {
        let base = compute(80, 24, &["a"], true, 1, 1, false);
        let wrapped_title = compute(80, 24, &["a"], true, 2, 1, false);
        let wrapped_hint = compute(80, 24, &["a"], true, 1, 3, false);
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
        let with_zero = compute(80, 24, &["a"], true, 0, 0, false);
        let with_one = compute(80, 24, &["a"], true, 1, 1, false);
        assert_eq!(with_zero.body_rows, with_one.body_rows);
    }

    #[test]
    fn unicode_borders_is_carried_through_unchanged_in_every_shape() {
        // Both the narrow (cols < 56) and wide early-return paths set the
        // field from the same parameter -- covering both return points, not
        // just one, is the point of asserting it twice here.
        let narrow = compute(40, 24, &["a"], true, 1, 1, true);
        assert!(narrow.unicode_borders);
        let wide = compute(80, 24, &["a"], true, 1, 1, true);
        assert!(wide.unicode_borders);
        let ascii = compute(80, 24, &["a"], true, 1, 1, false);
        assert!(!ascii.unicode_borders);
    }

    // hit_test: a wide layout (80x24, title_rows=1) has left_w=22, right_w=55
    // -- computed the same way render.rs's own frame is, so these column
    // numbers are exactly where a real click on that real frame would land.
    // Body row 0 sits at absolute (1-based) row 3: row 1 is the title bar,
    // row 2 is the top border, row 3 is the first body row.

    #[test]
    fn a_click_on_the_left_border_hits_nothing() {
        let layout = compute(80, 24, &["todo", "bug-report"], true, 1, 1, false);
        assert_eq!(hit_test(&layout, 1, false, 1, 3), None);
    }

    #[test]
    fn a_click_inside_the_list_pane_names_its_row() {
        let layout = compute(80, 24, &["todo", "bug-report"], true, 1, 1, false);
        assert_eq!(
            hit_test(&layout, 1, false, 2, 3),
            Some(ClickTarget::ListRow { row: 0, col: 0 })
        );
        // the last column still inside the list pane (left_w = 22)
        assert_eq!(
            hit_test(&layout, 1, false, 23, 3),
            Some(ClickTarget::ListRow { row: 0, col: 21 })
        );
    }

    #[test]
    fn a_click_on_the_divider_hits_nothing() {
        let layout = compute(80, 24, &["todo", "bug-report"], true, 1, 1, false);
        assert_eq!(hit_test(&layout, 1, false, 24, 3), None);
    }

    #[test]
    fn a_click_inside_the_info_pane_hits_info() {
        let layout = compute(80, 24, &["todo", "bug-report"], true, 1, 1, false);
        // col 25 is the divider(24) + 1 -- the info pane's own first content
        // column, so col 0 within the pane.
        assert_eq!(
            hit_test(&layout, 1, false, 25, 3),
            Some(ClickTarget::Info { row: 0, col: 0 })
        );
        // the last column still inside the info pane (right_w = 55)
        assert_eq!(
            hit_test(&layout, 1, false, 79, 3),
            Some(ClickTarget::Info { row: 0, col: 54 })
        );
    }

    #[test]
    fn an_info_click_reports_the_row_relative_to_the_scrolled_view() {
        let layout = compute(80, 24, &["todo", "bug-report"], true, 1, 1, false);
        // row 6 is body row 3 (row - body_start, body_start = title_rows(1) + 1)
        assert_eq!(
            hit_test(&layout, 1, false, 25, 6),
            Some(ClickTarget::Info { row: 3, col: 0 })
        );
    }

    #[test]
    fn a_click_on_the_right_border_hits_nothing() {
        let layout = compute(80, 24, &["todo", "bug-report"], true, 1, 1, false);
        assert_eq!(hit_test(&layout, 1, false, 80, 3), None);
    }

    #[test]
    fn a_click_above_or_below_the_body_hits_nothing() {
        let layout = compute(80, 24, &["todo", "bug-report"], true, 1, 1, false);
        assert_eq!(hit_test(&layout, 1, false, 2, 1), None); // the title bar
        assert_eq!(hit_test(&layout, 1, false, 2, 2), None); // the top border
        assert_eq!(hit_test(&layout, 1, false, 2, 100), None); // well past the frame
    }

    #[test]
    fn a_click_at_col_or_row_zero_hits_nothing() {
        // 1-based coordinates: 0 never names a real cell in either axis.
        let layout = compute(80, 24, &["todo", "bug-report"], true, 1, 1, false);
        assert_eq!(hit_test(&layout, 1, false, 0, 3), None);
        assert_eq!(hit_test(&layout, 1, false, 2, 0), None);
    }

    #[test]
    fn a_second_body_row_names_list_row_one() {
        let layout = compute(80, 24, &["todo", "bug-report"], true, 1, 1, false);
        assert_eq!(
            hit_test(&layout, 1, false, 2, 4),
            Some(ClickTarget::ListRow { row: 1, col: 0 })
        );
    }

    #[test]
    fn narrow_layout_follows_info_focused_to_pick_the_pane() {
        let layout = compute(40, 24, &["todo"], true, 1, 1, false);
        // The same body cell is the list when the list has focus, and
        // Info once focus moves there -- narrow shows one pane at a time.
        assert_eq!(
            hit_test(&layout, 1, false, 2, 3),
            Some(ClickTarget::ListRow { row: 0, col: 0 })
        );
        assert_eq!(
            hit_test(&layout, 1, true, 2, 3),
            Some(ClickTarget::Info { row: 0, col: 0 })
        );
    }
}
