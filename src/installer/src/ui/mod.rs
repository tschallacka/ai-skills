// MODE: DEV
// PACKAGE: PROD
//! The full-screen skill picker's event loop. Ties layout, model, render,
//! input, terminal and the mascot together; see each submodule's own doc
//! comment for what it deliberately leaves out ("right-top" mascot
//! placement is the one still missing; mouse clicks are handled below via
//! `layout::hit_test`).

pub mod buttons;
pub mod input;
pub mod layout;
pub mod mascot;
pub mod model;
pub mod render;
pub mod terminal;
pub mod text;
pub mod uninstall_picker;
pub mod wizard;

use input::Key;
use mascot::{ColorMode, EyeAnimator};
use model::{Focus, PickerState, SkillEntry};
use std::path::Path;

/// `None` when fd 0 is not a tty (the caller's cue to fall back to a plain
/// listing) or the user quit (q/Esc/Ctrl-C/EOF); `Some(names_and_modes)` in
/// original skill order once `i` confirms, the mode being whatever `m`
/// last cycled it to (or the run's already-resolved default, if `m` was
/// never pressed for that skill). `source_root` is only used by `r`
/// (reverify).
pub fn run_picker(skills: Vec<SkillEntry>, source_root: &Path) -> Option<Vec<(String, String)>> {
    run_picker_with(skills, source_root, PickerState::new)
}

/// Like `run_picker`, but only a skill this run's own `SkillEntry.installed`
/// already reports true starts selected -- `ui::wizard`'s "use previous
/// settings?" step accepted, so the picker opens showing exactly what is
/// already at the target root rather than everything checked.
pub fn run_picker_preselecting_installed(
    skills: Vec<SkillEntry>,
    source_root: &Path,
) -> Option<Vec<(String, String)>> {
    run_picker_with(skills, source_root, PickerState::new_preselecting_installed)
}

fn run_picker_with(
    skills: Vec<SkillEntry>,
    source_root: &Path,
    make_state: impl FnOnce(Vec<SkillEntry>) -> PickerState,
) -> Option<Vec<(String, String)>> {
    if !terminal::is_tty() {
        return None;
    }
    let mut state = make_state(skills);
    let saved = terminal::enter();
    let rx = terminal::spawn_reader();
    // Probed once: the picker redraws on every keypress and tick, and a
    // per-frame `tput` shellout would spawn a process on every redraw.
    let color_mode = mascot::detect_color_mode();
    let unicode_borders = mascot::detect_utf8_capable();
    let mut eyes = EyeAnimator::new();

    loop {
        let (cols, rows) = terminal::size();
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let title_rows = render::title_bar_lines(&state, cols).len();
        let hint_rows = render::hint_bar_lines(cols).len();
        let layout = layout::compute(
            cols,
            rows,
            &names,
            color_mode != ColorMode::None,
            title_rows,
            hint_rows,
            unicode_borders,
        );
        state.clamp_scroll(layout.body_rows);
        clamp_info_scroll(&mut state, &layout);
        terminal::draw(&render::render_frame(&state, &layout, color_mode));
        if layout.mascot_on {
            draw_mascot(&layout, color_mode, eyes.current(), unicode_borders);
        }

        match input::read_key(&rx) {
            Key::Tick => {
                eyes.advance();
                continue;
            }
            Key::Eof => {
                state.done = true;
                state.confirmed = false;
            }
            key => handle_key(&mut state, key, &layout, title_rows, hint_rows, source_root),
        }
        if state.done {
            break;
        }
    }

    terminal::leave(&saved);
    if state.confirmed {
        Some(state.selected_with_modes())
    } else {
        None
    }
}

/// The sprite sits under the list, left-aligned two columns in (past the
/// pane's leading `|`), starting right below the separator render.rs left
/// blank at body row `layout.list_rows` -- title(1) + top border(1) + that
/// separator's own row + 1 = `list_rows + 4` in absolute terminal rows.
pub(crate) fn draw_mascot(
    layout: &layout::Layout,
    mode: ColorMode,
    eye: mascot::EyeState,
    unicode: bool,
) {
    draw_mascot_at(layout.list_rows + 4, 2, mode, eye, unicode);
}

/// Like `draw_mascot`, but the caller supplies the absolute (row, col)
/// itself instead of deriving it from a list-based `Layout` -- the wizard's
/// `InstallOrUninstall` screen has no list pane to pin the sprite under, and
/// centers it at the top instead.
pub(crate) fn draw_mascot_at(
    row: usize,
    col: usize,
    mode: ColorMode,
    eye: mascot::EyeState,
    unicode: bool,
) {
    let lines: Vec<String> = (0..mascot::HEIGHT)
        .map(|r| mascot::head_line(mode, r, eye, unicode))
        .collect();
    terminal::draw_overlay(row, col, &lines);
}

/// The info pane's own line count depends on the current skill's
/// description length, so its max scroll is recomputed every frame rather
/// than tracked as separate state.
fn clamp_info_scroll(state: &mut PickerState, layout: &layout::Layout) {
    let width = if layout.narrow {
        layout.left_w
    } else {
        layout.right_w
    };
    let total = render::info_lines(state, width).len();
    let max_scroll = total.saturating_sub(layout.body_rows);
    if state.info_scroll > max_scroll {
        state.info_scroll = max_scroll;
    }
}

fn handle_key(
    state: &mut PickerState,
    key: Key,
    layout: &layout::Layout,
    title_rows: usize,
    hint_rows: usize,
    source_root: &Path,
) {
    match key {
        Key::Click { col, row } => {
            if let Some(click) = render::hint_click_at(layout, hint_rows, col, row) {
                apply_hint_click(state, click);
                return;
            }
            let info_focused = layout.narrow && state.focus == Focus::Info;
            match layout::hit_test(layout, title_rows, info_focused, col, row) {
                Some(layout::ClickTarget::ListRow(body_row)) => {
                    let index = state.scroll + body_row;
                    if index < state.skills.len() {
                        state.cursor = index;
                        state.toggle(index);
                        state.focus = Focus::List;
                    }
                }
                Some(layout::ClickTarget::Info { row, col }) => {
                    state.focus = Focus::Info;
                    handle_info_click(state, row + state.info_scroll, col, layout, source_root);
                }
                None => {}
            }
        }
        Key::Up | Key::Char('k') => state.move_by(-1),
        Key::Down | Key::Char('j') => state.move_by(1),
        Key::PageUp => state.move_by(-(layout.body_rows as isize)),
        Key::PageDown => state.move_by(layout.body_rows as isize),
        Key::Home => state.go_home(),
        Key::End => {
            let width = if layout.narrow {
                layout.left_w
            } else {
                layout.right_w
            };
            let max_scroll = render::info_lines(state, width)
                .len()
                .saturating_sub(layout.body_rows);
            state.go_end(max_scroll);
        }
        Key::Enter | Key::Space => state.toggle(state.cursor),
        Key::Tab | Key::ShiftTab => state.toggle_focus(),
        Key::Char('a') => state.select_all(),
        Key::Char('n') => state.select_none(),
        // d/r/m are focus-gated: the ACTIONS lines are only usable when the
        // info pane holds focus ('i' already means "install", so cycling
        // the mode could not reuse it). Pressed from the list pane instead,
        // they used to do nothing at all with no feedback; now they say why,
        // the same way `toggle`'s Blocked refusal already does.
        Key::Char('d') if state.focus == Focus::Info => state.dep_hint(),
        Key::Char('r') if state.focus == Focus::Info => state.reverify(source_root),
        Key::Char('m') if state.focus == Focus::Info => state.cycle_integration_mode(),
        Key::Char(c @ ('d' | 'r' | 'm')) if state.focus == Focus::List => {
            let action = match c {
                'd' => "help me install dependencies",
                'r' => "reverify dependencies",
                _ => "cycle integration mode",
            };
            state.message = vec![format!(
                "'{c}' ({action}) needs the DETAILS pane focused -- press Tab first"
            )];
        }
        Key::Char('i') => {
            state.done = true;
            state.confirmed = true;
        }
        Key::Char('q') | Key::Escape => {
            state.done = true;
            state.confirmed = false;
        }
        _ => {}
    }
}

/// A click on one of the hint bar's own buttons -- see `render::HintClick`
/// for which segments have one at all (a compound label like "Up/Dn move"
/// does not: the list rows are already directly clickable for that, and a
/// single click has no obvious "up or down" meaning of its own).
fn apply_hint_click(state: &mut PickerState, click: render::HintClick) {
    match click {
        render::HintClick::ToggleCurrent => state.toggle(state.cursor),
        render::HintClick::FocusToggle => state.toggle_focus(),
        render::HintClick::SelectAll => state.select_all(),
        render::HintClick::SelectNone => state.select_none(),
        render::HintClick::Install => {
            state.done = true;
            state.confirmed = true;
        }
        render::HintClick::Quit => {
            state.done = true;
            state.confirmed = false;
        }
    }
}

/// A click inside the DETAILS pane, once `layout::hit_test` has already
/// resolved it to a pane-relative `(row, col)` -- `row` here is already the
/// ABSOLUTE line index into `render::info_lines` (the caller added
/// `state.info_scroll`), matching what `render::info_layout`'s own row
/// numbers are computed against. Anywhere that is not one of the two
/// ACTIONS buttons or an integration-mode toggle segment is simply focus,
/// exactly as a click anywhere else in the pane always has been.
fn handle_info_click(
    state: &mut PickerState,
    row: usize,
    col: usize,
    layout: &layout::Layout,
    source_root: &Path,
) {
    let width = if layout.narrow {
        layout.left_w
    } else {
        layout.right_w
    };
    let info_layout = render::info_layout(state, width);
    if info_layout.dep_hint_row == Some(row) {
        state.dep_hint();
    } else if info_layout.reverify_row == Some(row) {
        state.reverify(source_root);
    } else if let Some(toggle) = &info_layout.mode_toggle {
        if toggle.row == row {
            if let Some((mode_name, _, _)) = toggle
                .segments
                .iter()
                .find(|(_, start, end)| (*start..*end).contains(&col))
            {
                state.set_integration_mode(mode_name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::requirements::SkillState as RequirementState;
    use crate::requirements::SkillStatus;

    fn skills(names: &[&str]) -> Vec<SkillEntry> {
        names
            .iter()
            .map(|n| SkillEntry {
                name: n.to_string(),
                description: format!("{n} does things."),
                installed: false,
                status: SkillStatus {
                    state: RequirementState::Ok,
                    blocker: None,
                    requirements: Vec::new(),
                },
                offered_modes: Vec::new(),
                mode: "skill".to_string(),
            })
            .collect()
    }

    /// A wide 80x24 layout at title_rows=1 -- the same numbers `layout`'s
    /// own hit_test tests use, so the click coordinates below (col 2, row
    /// 3/4 for the first/second list rows; col 25, row 3 for the info pane)
    /// are exactly what those tests already proved lands where this expects.
    fn wide_layout(names: &[&str]) -> layout::Layout {
        layout::compute(80, 24, names, true, 1, 1, false)
    }

    #[test]
    fn clicking_a_list_row_moves_the_cursor_there_and_toggles_it() {
        let mut state = PickerState::new(skills(&["todo", "bug-report"]));
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let layout = wide_layout(&names);
        let source = std::path::Path::new(".");
        // Everything starts selected; clicking the second row deselects it
        // and moves the cursor there in one action.
        handle_key(
            &mut state,
            Key::Click { col: 2, row: 4 },
            &layout,
            1,
            1,
            source,
        );
        assert_eq!(state.cursor, 1);
        assert!(!state.selected[1]);
        assert_eq!(state.focus, Focus::List);
    }

    #[test]
    fn clicking_the_info_pane_moves_focus_there_without_changing_selection() {
        let mut state = PickerState::new(skills(&["todo", "bug-report"]));
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let layout = wide_layout(&names);
        let source = std::path::Path::new(".");
        handle_key(
            &mut state,
            Key::Click { col: 25, row: 3 },
            &layout,
            1,
            1,
            source,
        );
        assert_eq!(state.focus, Focus::Info);
        assert!(state.selected[0]);
        assert!(state.selected[1]);
    }

    #[test]
    fn clicking_a_border_changes_nothing() {
        let mut state = PickerState::new(skills(&["todo", "bug-report"]));
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let layout = wide_layout(&names);
        let source = std::path::Path::new(".");
        handle_key(
            &mut state,
            Key::Click { col: 1, row: 3 },
            &layout,
            1,
            1,
            source,
        );
        assert_eq!(state.cursor, 0);
        assert_eq!(state.focus, Focus::List);
        assert!(state.selected[0]);
        assert!(state.selected[1]);
    }
}
