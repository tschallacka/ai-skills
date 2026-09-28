// MODE: DEV
// PACKAGE: PROD
//! Builds one frame as exactly `rows` lines of exactly `cols` display
//! cells, independent of the terminal, so it is unit-testable without a
//! live tty. Content (skill names/descriptions) is ASCII-only, so its byte
//! length is its cell width; the border glyphs are the one place that is not
//! true (`BorderSet::UNICODE`'s box-drawing characters are 3 UTF-8 bytes
//! each but still exactly one display column), which is why every width
//! check in this module's own tests counts `.chars()`, not `.len()`.
//!
//! The mascot itself is NOT drawn here: when `layout.mascot_on`, this just
//! reserves its rows as blank cells (a separator line, then `mascot::HEIGHT`
//! blank rows) in the list pane. The actual sprite is painted as a colored
//! overlay at an absolute position after this frame is drawn -- mixing SGR
//! escapes into these strings would break the "every line is exactly
//! `cols` display cells" invariant every test here checks.

use super::layout::Layout;
use super::model::{Focus, PickerState};
use super::text::{overflow, pad, wrap};
use crate::requirements::SkillState;

/// Both bars are chrome, not critical content: at a width where they don't
/// fit on one line, wrapping onto a second line is the graceful fallback,
/// and only a THIRD line -- content wrapping alone still couldn't fit --
/// gets `pad`'s `...` truncation. `layout::compute` needs the resulting
/// line count (at this same `cols`) to reserve the right number of body
/// rows before this ever renders; see its own doc comment.
const BAR_MAX_LINES: usize = 3;

pub(crate) fn title_bar_text(state: &PickerState) -> String {
    format!(
        " AI-SKILLS INSTALLER  {}/{} installed  {} selected ",
        state.installed_count(),
        state.skills.len(),
        state.selected_count()
    )
}

pub(crate) const HINT_TEXT: &str =
    " Up/Dn move  Enter/Space toggle  Tab focus  a all  n none  i install  q quit";

pub(crate) fn title_bar_lines(state: &PickerState, cols: usize) -> Vec<String> {
    overflow(&title_bar_text(state), cols, BAR_MAX_LINES)
}

pub(crate) fn hint_bar_lines(cols: usize) -> Vec<String> {
    overflow(HINT_TEXT, cols, BAR_MAX_LINES)
}

/// The state suffix is appended after the name rather than inserted before
/// it, so an Ok skill's row (the common case, and the only case in most
/// existing frame-shape tests) renders byte-identical to before this state
/// tag existed.
fn list_row(state: &PickerState, index: usize, width: usize) -> String {
    let cursor = if index == state.cursor { '>' } else { ' ' };
    let checkbox = if state.selected[index] { "[x]" } else { "[ ]" };
    let name = &state.skills[index].name;
    let suffix = match state.skills[index].status.state {
        SkillState::Ok => "",
        SkillState::Degraded => " ~",
        SkillState::Blocked => " !",
    };
    pad(&format!("{cursor}{checkbox} {name}{suffix}"), width)
}

/// Name, description, install status, DEPENDENCIES (when requires.tsv named
/// any), and ACTIONS (dependency help, reverify, and the `m` line when the
/// skill offers more than one integration mode).
pub(crate) fn info_lines(state: &PickerState, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let skill = &state.skills[state.cursor];
    lines.push(pad("-".repeat(width).as_str(), width));
    lines.push(pad(&skill.name, width));
    lines.push(pad("", width));
    for line in wrap(&skill.description, width) {
        lines.push(pad(&line, width));
    }
    lines.push(pad("", width));
    lines.push(pad("STATUS", width));
    lines.push(pad(
        &format!(
            "  installed      {}",
            if skill.installed { "yes" } else { "no" }
        ),
        width,
    ));
    lines.push(pad(
        &format!(
            "  state          {}",
            match skill.status.state {
                SkillState::Ok => "ok",
                SkillState::Degraded => "degraded",
                SkillState::Blocked => "blocked",
            }
        ),
        width,
    ));
    if !skill.status.requirements.is_empty() {
        lines.push(pad("", width));
        lines.push(pad("DEPENDENCIES", width));
        for (req, met) in &skill.status.requirements {
            let label = crate::requirements::requirement_label(req);
            let mark = if *met { "ok" } else { "missing" };
            let strength = match req.strength {
                crate::requirements::Strength::Hard => "hard",
                crate::requirements::Strength::Soft => "soft",
            };
            for line in wrap(
                &format!("  {label} ({strength}): {mark} -- {}", req.why),
                width,
            ) {
                lines.push(pad(&line, width));
            }
        }
    }
    // Always listed, usable only when the info pane has focus, but named
    // here regardless so a reader in the list pane already knows what
    // focusing it offers.
    lines.push(pad("", width));
    lines.push(pad("ACTIONS", width));
    lines.push(pad("  d  help me install dependencies", width));
    lines.push(pad("  r  reverify dependencies", width));
    if skill.offered_modes.len() > 1 {
        for line in wrap(
            &format!(
                "  m  integration mode: {}   (cycles: {})",
                skill.mode,
                skill.offered_modes.join(" ")
            ),
            width,
        ) {
            lines.push(pad(&line, width));
        }
    }
    if !state.message.is_empty() {
        lines.push(pad("", width));
        for message in &state.message {
            for line in wrap(message, width) {
                lines.push(pad(&line, width));
            }
        }
    }
    lines
}

/// The glyphs every border-drawing function below draws with, resolved once
/// per frame from `layout.unicode_borders` rather than read as a global, the
/// same reason `color_mode` is threaded as a value everywhere else here: a
/// test can force either set with no terminal, and nothing here depends on
/// probing anything live. Every field is exactly one display column in every
/// terminal that renders it at all (plain ASCII, or the box-drawing block
/// U+2500-U+257F) -- picking the wrong set for an incapable terminal is a
/// portability bug, not a width bug, and `detect_utf8_capable` is what
/// guards against that.
pub(crate) struct BorderSet {
    pub(crate) horizontal: char,
    pub(crate) vertical: char,
    pub(crate) corner_tl: char,
    pub(crate) corner_tr: char,
    pub(crate) corner_bl: char,
    pub(crate) corner_br: char,
    /// Where the list pane's top border meets the divider into the info
    /// pane (a T pointing down into the frame).
    pub(crate) divider_top: char,
    /// The same divider's bottom-border counterpart (a T pointing up).
    pub(crate) divider_bottom: char,
}

impl BorderSet {
    pub(crate) const ASCII: BorderSet = BorderSet {
        horizontal: '-',
        vertical: '|',
        corner_tl: '+',
        corner_tr: '+',
        corner_bl: '+',
        corner_br: '+',
        divider_top: '+',
        divider_bottom: '+',
    };
    pub(crate) const UNICODE: BorderSet = BorderSet {
        horizontal: '─',
        vertical: '│',
        corner_tl: '┌',
        corner_tr: '┐',
        corner_bl: '└',
        corner_br: '┘',
        divider_top: '┬',
        divider_bottom: '┴',
    };

    /// `wizard`'s own full-screen steps have no `Layout` of their own (no
    /// skill list, no info pane) but still want the same glyph set a
    /// UTF-8-capable terminal gets everywhere else -- this is the plain
    /// bool-driven half `for_layout` is built from, so both callers pick the
    /// same two sets from one place rather than each hardcoding its own.
    pub(crate) fn for_unicode(unicode: bool) -> &'static BorderSet {
        if unicode {
            &BorderSet::UNICODE
        } else {
            &BorderSet::ASCII
        }
    }

    fn for_layout(layout: &Layout) -> &'static BorderSet {
        BorderSet::for_unicode(layout.unicode_borders)
    }
}

fn top_border(layout: &Layout, focus: Focus) -> String {
    let b = BorderSet::for_layout(layout);
    let list_label = if focus == Focus::List {
        "[SKILLS]"
    } else {
        " SKILLS "
    };
    let info_label = if focus == Focus::Info {
        "[DETAILS]"
    } else {
        " DETAILS "
    };
    format!(
        "{}{}{}{}{}",
        b.corner_tl,
        pad_center_dash(list_label, layout.left_w, b.horizontal),
        b.divider_top,
        pad_center_dash(info_label, layout.right_w, b.horizontal),
        b.corner_tr
    )
}

fn pad_center_dash(label: &str, width: usize, fill: char) -> String {
    if label.len() >= width {
        return label[..width.min(label.len())].to_string();
    }
    format!(
        "{label}{}",
        std::iter::repeat_n(fill, width - label.len()).collect::<String>()
    )
}

fn bottom_border(layout: &Layout) -> String {
    let b = BorderSet::for_layout(layout);
    format!(
        "{}{}{}{}{}",
        b.corner_bl,
        std::iter::repeat_n(b.horizontal, layout.left_w).collect::<String>(),
        b.divider_bottom,
        std::iter::repeat_n(b.horizontal, layout.right_w).collect::<String>(),
        b.corner_br
    )
}

pub fn render_frame(state: &PickerState, layout: &Layout) -> Vec<String> {
    let mut out = Vec::with_capacity(layout.rows);
    out.extend(title_bar_lines(state, layout.cols));

    if layout.narrow {
        render_narrow(state, layout, &mut out);
    } else {
        render_wide(state, layout, &mut out);
    }
    out.extend(hint_bar_lines(layout.cols));
    out
}

/// One list-pane cell for body row `body`: a skill row while `body` is
/// inside `layout.list_rows`, then (only when `layout.mascot_on`) one
/// separator line and `mascot::HEIGHT` blank rows the overlay paints over,
/// then blank padding for whatever body rows remain.
fn list_cell(state: &PickerState, layout: &Layout, body: usize) -> String {
    if body < layout.list_rows {
        return if state.scroll + body < state.skills.len() {
            list_row(state, state.scroll + body, layout.left_w)
        } else {
            pad("", layout.left_w)
        };
    }
    if layout.mascot_on && body == layout.list_rows {
        let fill = BorderSet::for_layout(layout).horizontal;
        return std::iter::repeat_n(fill, layout.left_w).collect();
    }
    pad("", layout.left_w)
}

fn render_wide(state: &PickerState, layout: &Layout, out: &mut Vec<String>) {
    let b = BorderSet::for_layout(layout);
    out.push(top_border(layout, state.focus));
    let info = info_lines(state, layout.right_w);
    for body in 0..layout.body_rows {
        let info_index = body + state.info_scroll;
        let info_cell = info
            .get(info_index)
            .cloned()
            .unwrap_or_else(|| pad("", layout.right_w));
        out.push(format!(
            "{}{}{}{info_cell}{}",
            b.vertical,
            list_cell(state, layout, body),
            b.vertical,
            b.vertical
        ));
    }
    out.push(bottom_border(layout));
}

fn render_narrow(state: &PickerState, layout: &Layout, out: &mut Vec<String>) {
    let b = BorderSet::for_layout(layout);
    let label = if state.focus == Focus::Info {
        "[DETAILS]"
    } else {
        "[SKILLS]"
    };
    out.push(format!(
        "{}{}{}",
        b.corner_tl,
        pad_center_dash(label, layout.left_w, b.horizontal),
        b.corner_tr
    ));
    let info = if state.focus == Focus::Info {
        info_lines(state, layout.left_w)
    } else {
        Vec::new()
    };
    for body in 0..layout.body_rows {
        let cell = if state.focus == Focus::Info {
            info.get(body + state.info_scroll)
                .cloned()
                .unwrap_or_else(|| pad("", layout.left_w))
        } else if state.scroll + body < state.skills.len() {
            list_row(state, state.scroll + body, layout.left_w)
        } else {
            pad("", layout.left_w)
        };
        out.push(format!("{}{cell}{}", b.vertical, b.vertical));
    }
    out.push(format!(
        "{}{}{}",
        b.corner_bl,
        std::iter::repeat_n(b.horizontal, layout.left_w).collect::<String>(),
        b.corner_br
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::requirements::SkillStatus;
    use crate::ui::layout;
    use crate::ui::model::SkillEntry;

    fn skills(names: &[&str]) -> Vec<SkillEntry> {
        names
            .iter()
            .map(|n| SkillEntry {
                name: n.to_string(),
                description: format!("{n} does things."),
                installed: false,
                status: SkillStatus {
                    state: SkillState::Ok,
                    blocker: None,
                    requirements: Vec::new(),
                },
                offered_modes: Vec::new(),
                mode: "skill".to_string(),
            })
            .collect()
    }

    /// `layout::compute` needs the ACTUAL number of lines the title/hint
    /// bars will take at this `cols` (they vary with both the width and,
    /// for the title, the state's own counts) -- derived here exactly the
    /// way `render_frame` derives them, so a test's `Layout` always agrees
    /// with what `render_frame` actually produces at the same inputs.
    fn layout_for(cols: usize, rows: usize, state: &PickerState, color_capable: bool) -> Layout {
        layout_for_borders(cols, rows, state, color_capable, false)
    }

    fn layout_for_borders(
        cols: usize,
        rows: usize,
        state: &PickerState,
        color_capable: bool,
        unicode_borders: bool,
    ) -> Layout {
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let title_rows = title_bar_lines(state, cols).len();
        let hint_rows = hint_bar_lines(cols).len();
        layout::compute(
            cols,
            rows,
            &names,
            color_capable,
            title_rows,
            hint_rows,
            unicode_borders,
        )
    }

    #[test]
    fn every_line_is_exactly_the_terminal_width() {
        let state = PickerState::new(skills(&["todo", "bug-report"]));
        let layout = layout_for(80, 24, &state, true);
        let frame = render_frame(&state, &layout);
        assert_eq!(frame.len(), layout.rows);
        for line in &frame {
            assert_eq!(line.chars().count(), layout.cols, "line was: {line:?}");
        }
    }

    /// The same invariant as above, but with the box-drawing border set,
    /// where a border cell is 3 UTF-8 bytes yet still exactly one display
    /// column -- `.chars().count()`, not `.len()`, is what must hold here.
    #[test]
    fn unicode_borders_still_measure_one_column_per_glyph() {
        let state = PickerState::new(skills(&["todo", "bug-report"]));
        let layout = layout_for_borders(80, 24, &state, true, true);
        let frame = render_frame(&state, &layout);
        assert_eq!(frame.len(), layout.rows);
        for line in &frame {
            assert_eq!(line.chars().count(), layout.cols, "line was: {line:?}");
        }
        // And the byte length is now genuinely longer than the column count
        // on a bordered row -- proving this test would have caught a
        // regression back to raw `.len()`, not just passed by coincidence.
        let bordered_row = &frame[1];
        assert!(
            bordered_row.len() > bordered_row.chars().count(),
            "expected multi-byte border glyphs on a bordered row: {bordered_row:?}"
        );
    }

    #[test]
    fn unicode_borders_draw_the_box_drawing_glyphs_ascii_never_does() {
        let state = PickerState::new(skills(&["todo"]));
        let layout = layout_for_borders(80, 24, &state, true, true);
        let frame = render_frame(&state, &layout);
        let top = &frame[1];
        assert!(top.starts_with('┌'), "top border was: {top:?}");
        assert!(top.ends_with('┐'), "top border was: {top:?}");
        assert!(top.contains('┬'), "top border was: {top:?}");
        // the hint bar is the true last line; the border sits one above it
        let bottom = &frame[frame.len() - 2];
        assert!(bottom.starts_with('└'), "bottom border was: {bottom:?}");
        assert!(bottom.ends_with('┘'), "bottom border was: {bottom:?}");
        assert!(bottom.contains('┴'), "bottom border was: {bottom:?}");
    }

    #[test]
    fn ascii_borders_never_draw_a_box_drawing_glyph() {
        let state = PickerState::new(skills(&["todo"]));
        let layout = layout_for(80, 24, &state, true);
        let frame = render_frame(&state, &layout);
        for line in &frame {
            assert!(
                !line.contains(['┌', '┐', '└', '┘', '─', '│', '┬', '┴']),
                "ASCII mode drew a box-drawing glyph: {line:?}"
            );
        }
    }

    #[test]
    fn the_cursor_row_carries_the_marker() {
        let mut state = PickerState::new(skills(&["todo", "bug-report"]));
        state.cursor = 1;
        let layout = layout_for(80, 24, &state, true);
        let title_rows = title_bar_lines(&state, layout.cols).len();
        let frame = render_frame(&state, &layout);
        // title bar(s), then the top border, then the second skill row.
        let body_line = &frame[title_rows + 1 + 1];
        assert!(body_line.contains(">[x] bug-report"));
    }

    #[test]
    fn a_deselected_skill_shows_an_empty_checkbox() {
        let mut state = PickerState::new(skills(&["todo"]));
        state.toggle(0);
        let layout = layout_for(80, 24, &state, true);
        let title_rows = title_bar_lines(&state, layout.cols).len();
        let frame = render_frame(&state, &layout);
        assert!(frame[title_rows + 1].contains("[ ] todo"));
    }

    #[test]
    fn a_narrow_terminal_renders_one_pane_with_no_pipe_divider() {
        let state = PickerState::new(skills(&["todo"]));
        let layout = layout_for(40, 24, &state, true);
        let title_rows = title_bar_lines(&state, layout.cols).len();
        let hint_rows = hint_bar_lines(layout.cols).len();
        let frame = render_frame(&state, &layout);
        // Everything between the title bar's own line(s) + its top border,
        // and the hint bar's own line(s) + its bottom border, is body.
        let start = title_rows + 1;
        let end = frame.len() - hint_rows - 1;
        for line in &frame[start..end] {
            assert_eq!(line.matches('|').count(), 2, "line was: {line:?}");
        }
    }

    #[test]
    fn the_title_bar_reports_selected_and_installed_counts() {
        let mut state = PickerState::new(skills(&["a", "b", "c"]));
        state.toggle(0);
        state.skills[1].installed = true;
        let layout = layout_for(80, 24, &state, true);
        let frame = render_frame(&state, &layout);
        assert!(frame[0].contains("1/3 installed"));
        assert!(frame[0].contains("2 selected"));
    }

    #[test]
    fn wrap_hyphenates_a_token_wider_than_the_pane() {
        let lines = wrap("supercalifragilisticexpialidocious", 10);
        assert!(lines.len() > 1);
        assert!(lines[0].ends_with('-'));
    }

    #[test]
    fn wrap_breaks_on_whole_words_when_it_can() {
        let lines = wrap("one two three four", 8);
        for line in &lines {
            assert!(!line.contains("  "));
        }
        assert_eq!(lines.join(" "), "one two three four");
    }

    #[test]
    fn a_tall_terminal_reserves_blank_rows_for_the_mascot() {
        let state = PickerState::new(skills(&["todo"]));
        let layout = layout_for(80, 30, &state, true);
        assert!(layout.mascot_on);
        let title_rows = title_bar_lines(&state, layout.cols).len();
        let frame = render_frame(&state, &layout);
        // Every line still comes out exactly `cols` wide even with the
        // mascot's rows reserved -- the invariant every other test checks.
        for line in &frame {
            assert_eq!(line.len(), layout.cols);
        }
        let separator_row = title_rows + 1 + layout.list_rows;
        assert!(frame[separator_row].contains("-------"));
    }

    #[test]
    fn a_bar_that_does_not_fit_one_line_wraps_instead_of_truncating() {
        // At 20 columns neither bar fits on one line; both must wrap
        // rather than cut, and neither may show the truncation marker
        // unless even wrapped they still overrun BAR_MAX_LINES lines.
        let state = PickerState::new(skills(&["todo"]));
        let title_lines = title_bar_lines(&state, 20);
        let hint_lines = hint_bar_lines(20);
        assert!(title_lines.len() > 1, "title did not wrap: {title_lines:?}");
        assert!(hint_lines.len() > 1, "hint did not wrap: {hint_lines:?}");
        for line in &title_lines {
            assert_eq!(line.len(), 20);
        }
        for line in &hint_lines {
            assert_eq!(line.len(), 20);
        }
    }

    #[test]
    fn a_bar_truncation_marker_if_any_lands_only_on_the_last_line() {
        // An extreme width where even BAR_MAX_LINES of wrapping cannot fit
        // the hint text: the cut, if it happens, must be confined to the
        // final line.
        let hint_lines = hint_bar_lines(6);
        assert_eq!(hint_lines.len(), BAR_MAX_LINES);
        for line in &hint_lines[..hint_lines.len() - 1] {
            assert!(!line.contains("..."), "cut too early: {line:?}");
        }
    }

    #[test]
    fn a_skill_offering_more_than_one_mode_shows_the_mode_line() {
        let mut list = skills(&["ai-text-editor"]);
        list[0].offered_modes = vec!["skill".to_string(), "mcp".to_string()];
        list[0].mode = "mcp".to_string();
        let state = PickerState::new(list);
        let width = 60;
        let lines = info_lines(&state, width);
        assert!(lines.iter().any(|l| l.contains("ACTIONS")));
        assert!(lines
            .iter()
            .any(|l| l.contains("integration mode: mcp") && l.contains("cycles: skill mcp")));
    }

    #[test]
    fn a_skill_with_one_mode_still_shows_actions_but_no_mode_line() {
        let state = PickerState::new(skills(&["todo"]));
        let lines = info_lines(&state, 60);
        assert!(lines.iter().any(|l| l.contains("ACTIONS")));
        assert!(lines
            .iter()
            .any(|l| l.contains("help me install dependencies")));
        assert!(lines.iter().any(|l| l.contains("reverify dependencies")));
        assert!(!lines.iter().any(|l| l.contains("integration mode")));
    }
}
