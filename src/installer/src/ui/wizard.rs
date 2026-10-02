// MODE: DEV
// PACKAGE: PROD
//! The graphical front door: install vs. uninstall (a centered chooser, the
//! wizard's own first step), then where to install (or add a custom
//! directory), then -- when the chosen root already has something
//! installed -- "use these settings?" listing exactly what is already
//! there. Everything that used to be asked as plain sequential text before
//! the skill picker ever started is a step of one continuous full-screen
//! flow here, in the same raw-mode/alternate-screen session the picker
//! already uses, with real back-navigation between these steps. Asking
//! install-vs-uninstall FIRST (rather than after a destination was already
//! picked) is deliberate: `commit_root_selection`, not
//! `handle_install_or_uninstall`, is what actually decides what happens
//! next, precisely because that decision needs a destination to already be
//! known.
//!
//! Scope: this covers choosing ONE root set per run, exactly as the flow it
//! replaces did -- picking several roots still applies the same skill
//! selection and mode to each of them. Independent per-root skill/mode
//! selection is real further work, not started here.
//!
//! Once this hands off to `super::run_picker` or `super::uninstall_picker`,
//! those keep their existing q/Esc-quits-the-program semantics unchanged:
//! both are reached from other, non-wizard entry points too (`--target`/
//! `--agent` skip this module entirely), so giving them a second, wizard-
//! only "back" meaning would make quitting mean two different things
//! depending on how the picker was reached.

use super::buttons::colorize_button;
use super::input::{self, Key};
use super::mascot::{self, ColorMode};
use super::render::BorderSet;
use super::terminal;
use super::text::{is_precomposed_line, overflow, pad, wrap};
use std::path::PathBuf;

/// One selectable install destination, already resolved to a real path --
/// the wizard's own view of what `main.rs`'s `AvailableTarget` already
/// computed (an auto-detected agent root, or a previously-saved custom
/// directory).
pub struct AvailableRoot {
    pub path: PathBuf,
    pub label: String,
    pub kind: Option<String>,
    pub exists: bool,
}

/// What the wizard decided, for `run_interactive` to act on. `use_previous`
/// is only meaningful when `uninstall` is false; the caller pre-seeds the
/// picker's selection from what is already installed at the first checked
/// root instead of defaulting to everything selected.
pub struct Outcome {
    pub roots: Vec<(PathBuf, Option<String>)>,
    pub uninstall: bool,
    pub use_previous: bool,
    /// Any directory typed into `CustomPath` and confirmed this run --
    /// `main.rs` persists these to `custom_locations`, the same as the
    /// plain-text flow this replaces did in its own custom-directory
    /// branch. Never includes an auto-detected agent root: only a path the
    /// user actually typed belongs in that file.
    pub newly_added_custom: Vec<PathBuf>,
}

/// What `main.rs` already knows is installed at the root the wizard is
/// about to ask "use these settings?" about -- built by the caller from
/// `discover::discover_skills` + `integration::resolve_mode`, the same way
/// the skill picker's own entries are, so the wizard does not need its own
/// copy of that logic or its dependencies.
pub struct InstalledSkill {
    pub name: String,
    /// `None` for a skill that offers no mode choice at all (no
    /// `integration.tsv`, the near-total majority) -- `resolve_mode` still
    /// answers something for those (its own fallback default), but showing
    /// it here would claim a real choice was made where there was none to
    /// make, the same reason `render.rs`'s own info pane only shows a mode
    /// line when a skill's `offered_modes.len() > 1`.
    pub mode: Option<String>,
}

enum Step {
    RootSelect,
    CustomPath,
    InstallOrUninstall,
    UsePrevious,
}

struct WizardState {
    available: Vec<AvailableRoot>,
    checked: Vec<bool>,
    cursor: usize,
    step: Step,
    back_stack: Vec<Step>,
    custom_input: String,
    /// Set once `CustomPath`'s Enter finds a path that does not exist yet --
    /// the SAME step then shows "create it?" instead of the text field,
    /// rather than a whole separate step for one yes/no question.
    custom_pending_create: Option<PathBuf>,
    uninstall_cursor: usize, // 0 = Install, 1 = Uninstall
    installed_here: Vec<InstalledSkill>,
    message: Option<String>,
    done: bool,
    confirmed: bool,
    use_previous: bool,
    newly_added_custom: Vec<PathBuf>,
    /// `None` while whichever step is active has its own LIST pane focused
    /// (the default); `Some(0)`/`Some(1)` once Tab has moved focus to that
    /// step's own two details-pane buttons instead (`RootSelect`'s
    /// Install-now/Cancel, `UsePrevious`'s Use-these-settings/Start-fresh).
    /// Shared across steps rather than one field per step, since only one
    /// step is ever active at a time; `commit_root_selection` resets it to
    /// `None` on every transition into `UsePrevious` so a stale focus from
    /// `RootSelect` never carries over.
    button_focus: Option<usize>,
    /// `UsePrevious`'s own list cursor, into `installed_here` -- separate
    /// from `cursor` (`RootSelect`'s), since a user could in principle
    /// (via Escape) revisit `RootSelect` after `UsePrevious` and neither
    /// should disturb the other's position.
    use_previous_cursor: usize,
}

impl WizardState {
    fn new(available: Vec<AvailableRoot>) -> Self {
        let checked = vec![false; available.len()];
        WizardState {
            available,
            checked,
            cursor: 0,
            // Ask install-vs-uninstall FIRST: which destinations make sense
            // to offer (and what a picked one's own details mean) reads
            // more naturally once the user's intent is already known, and
            // it avoids the old order's "now that you've already picked
            // where, was that even for installing or removing something?".
            step: Step::InstallOrUninstall,
            back_stack: Vec::new(),
            custom_input: String::new(),
            custom_pending_create: None,
            uninstall_cursor: 0,
            installed_here: Vec::new(),
            message: None,
            done: false,
            confirmed: false,
            newly_added_custom: Vec::new(),
            use_previous: false,
            button_focus: None,
            use_previous_cursor: 0,
        }
    }

    /// `RootSelect`'s list has one extra row past `available` -- "add a
    /// custom directory" -- so its own bounds are one wider than the
    /// checkbox array's.
    fn root_rows(&self) -> usize {
        self.available.len() + 1
    }
}

/// Runs the wizard to completion. `installed_at` is called at most once,
/// lazily, only once a checked root needs its "use previous settings?"
/// listing built -- it is the caller's own `discover_skills` +
/// `resolve_mode` glue, kept out of this module so wizard.rs depends on
/// neither `discover` nor `integration` directly. Returns `None` on quit
/// (q/Esc/Ctrl-C/EOF) or a non-tty (the caller's cue to fall back to
/// whatever non-interactive path it already has for that case).
pub fn run(
    available: Vec<AvailableRoot>,
    installed_at: impl Fn(&std::path::Path) -> Vec<InstalledSkill>,
) -> Option<Outcome> {
    if !terminal::is_tty() || available.is_empty() {
        return None;
    }
    let mut state = WizardState::new(available);
    let saved = terminal::enter();
    let rx = terminal::spawn_reader();
    // Probed once, the same reason `run_picker` does: this redraws on every
    // keypress and tick, and a per-frame `tput` shellout would spawn a
    // process on every redraw.
    let color_mode = super::mascot::detect_color_mode();
    let unicode = super::mascot::detect_utf8_capable();
    let mut eyes = super::mascot::EyeAnimator::new();

    loop {
        let (cols, rows) = terminal::size();
        let (frame, layout) = render(&state, cols, rows, color_mode, unicode);
        terminal::draw(&frame);
        if layout.mascot_on {
            if matches!(state.step, Step::InstallOrUninstall) {
                // No list pane to pin the sprite under on this screen --
                // centered at the top instead, matching where the frame
                // itself left room (see `install_or_uninstall_frame`). Row 2,
                // not 1: a blank row above it first, so it doesn't start
                // flush against the terminal's own top edge.
                let col = cols.saturating_sub(mascot::WIDTH) / 2 + 1;
                super::draw_mascot_at(2, col, color_mode, eyes.current(), unicode);
            } else {
                super::draw_mascot(&layout, color_mode, eyes.current(), unicode);
            }
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
            key => handle_key(&mut state, key, &layout, &installed_at),
        }
        if state.done {
            break;
        }
    }

    terminal::leave(&saved);
    if !state.confirmed {
        return None;
    }
    let roots: Vec<(PathBuf, Option<String>)> = state
        .available
        .iter()
        .zip(state.checked.iter())
        .filter(|(_, checked)| **checked)
        .map(|(root, _)| (root.path.clone(), root.kind.clone()))
        .collect();
    if roots.is_empty() {
        return None;
    }
    Some(Outcome {
        roots,
        uninstall: state.uninstall_cursor == 1,
        use_previous: state.use_previous,
        newly_added_custom: state.newly_added_custom,
    })
}

fn handle_key(
    state: &mut WizardState,
    key: Key,
    layout: &super::layout::Layout,
    installed_at: &impl Fn(&std::path::Path) -> Vec<InstalledSkill>,
) {
    // A message is transient feedback from the LAST action (e.g. "pick at
    // least one root"); any new key press clears it rather than letting a
    // stale warning linger over an unrelated next action.
    state.message = None;
    if let Key::Click { col, row } = key {
        handle_click(state, layout, col, row, installed_at);
        return;
    }
    match state.step {
        Step::RootSelect => handle_root_select(state, key, installed_at),
        Step::CustomPath => handle_custom_path(state, key),
        Step::InstallOrUninstall => handle_install_or_uninstall(state, key),
        Step::UsePrevious => handle_use_previous(state, key),
    }
}

/// Maps a click onto the same effect the equivalent keys already have --
/// `InstallOrUninstall`'s two rows are real buttons a single click commits,
/// `RootSelect`'s list rows are clickable checkboxes (and the "add custom"
/// row), and its Install-now/Cancel buttons (living in the DETAILS pane,
/// under the "Selected" summary -- see `root_select_details_raw`) commit
/// directly rather than routing through a key. `CustomPath` (a text field)
/// and `UsePrevious` (no distinct button rows in its own view yet) stay
/// keyboard-only for now -- a real, deliberate scope boundary, not an
/// oversight.
///
/// `InstallOrUninstall` is handled first and returns immediately: its
/// centered layout has no title bar or border box at all (see
/// `install_or_uninstall_frame`), so the `body_start` convention every
/// OTHER step's rows are measured from does not apply to it.
fn handle_click(
    state: &mut WizardState,
    layout: &super::layout::Layout,
    col: u16,
    row: u16,
    installed_at: &impl Fn(&std::path::Path) -> Vec<InstalledSkill>,
) {
    if col <= 1 || row == 0 {
        return; // col 1 is the left border; row 0 is never a real row
    }
    let col = col as usize;
    let row = row as usize;
    if matches!(state.step, Step::InstallOrUninstall) {
        let (install_row, uninstall_row) = install_or_uninstall_click_rows(layout);
        if row == install_row {
            state.uninstall_cursor = 0;
            handle_install_or_uninstall(state, Key::Enter);
        } else if row == uninstall_row {
            state.uninstall_cursor = 1;
            handle_install_or_uninstall(state, Key::Enter);
        }
        return;
    }
    if matches!(state.step, Step::UsePrevious) {
        handle_use_previous_click(state, layout, col, row);
        return;
    }
    if !matches!(state.step, Step::RootSelect) {
        return;
    }

    // Assumes every OTHER step's own title is exactly one line, true for
    // every title string this module actually uses at any width realistic
    // enough to run a terminal UI in at all (the longest, "Choose where to
    // install", is 24 characters) -- an unrealistically narrow terminal
    // could make a click land one row off, never a crash or a corrupted
    // frame.
    const TITLE_LINES: usize = 1;
    let body_start = TITLE_LINES + 2; // title line(s) + the top border, 1-based
    if row < body_start {
        return;
    }
    let body_row = row - body_start;
    let details_start_col = layout.left_w + 3; // 1-based: the details pane's own first content column
    if col >= details_start_col {
        // A click inside the DETAILS pane, not the list -- the only thing
        // there to click is the button row under the "Selected" summary
        // (see `root_select_details_raw`, which always appends it last).
        let button_row =
            root_select_button_row_index(state, layout.right_w, layout.unicode_borders);
        if body_row == button_row {
            let content_col = col - details_start_col;
            match button_hit(content_col, INSTALL_LABEL, CANCEL_LABEL) {
                Some(0) => commit_root_selection(state, installed_at),
                Some(1) => {
                    state.done = true;
                    state.confirmed = false;
                }
                _ => {}
            }
        }
        return;
    }
    if body_row >= state.root_rows() {
        return;
    }
    state.cursor = body_row;
    state.button_focus = None; // clicking a list row returns focus to the list
    if body_row == state.available.len() {
        state.custom_input.clear();
        state.custom_pending_create = None;
        state.back_stack.push(Step::RootSelect);
        state.step = Step::CustomPath;
    } else {
        state.checked[body_row] = !state.checked[body_row];
    }
}

/// Mirrors `handle_click`'s own `RootSelect` handling, for the same shape
/// of screen: a click in the LIST pane moves the cursor there and returns
/// focus to the list; a click in the DETAILS pane only does anything on the
/// button row under the mode explanation (`use_previous_details_raw`
/// always appends it last), where it commits exactly like Enter/y/n
/// already do.
fn handle_use_previous_click(
    state: &mut WizardState,
    layout: &super::layout::Layout,
    col: usize,
    row: usize,
) {
    const TITLE_LINES: usize = 1;
    let body_start = TITLE_LINES + 2;
    if row < body_start {
        return;
    }
    let body_row = row - body_start;
    let details_start_col = layout.left_w + 3;
    if col >= details_start_col {
        let button_row =
            use_previous_button_row_index(state, layout.right_w, layout.unicode_borders);
        if body_row == button_row {
            let content_col = col - details_start_col;
            match button_hit(content_col, USE_PREVIOUS_LABEL, START_FRESH_LABEL) {
                Some(0) => {
                    state.use_previous = true;
                    state.done = true;
                    state.confirmed = true;
                }
                Some(1) => {
                    state.use_previous = false;
                    state.done = true;
                    state.confirmed = true;
                }
                _ => {}
            }
        }
        return;
    }
    if body_row >= state.installed_here.len() {
        return;
    }
    state.use_previous_cursor = body_row;
    state.button_focus = None;
}

fn handle_root_select(
    state: &mut WizardState,
    key: Key,
    installed_at: &impl Fn(&std::path::Path) -> Vec<InstalledSkill>,
) {
    if let Some(focused) = state.button_focus {
        match key {
            // Left at the first button ("Install now") hands focus back to
            // the list -- there is nothing further left than it -- and
            // Right at the last ("Cancel") is a no-op, since there is
            // nothing further right to hand off to. `h`/`l` are the same
            // vim-style synonyms Up/Down already have here (`k`/`j`).
            Key::Left | Key::Char('h') => {
                state.button_focus = if focused == 0 {
                    None
                } else {
                    Some(focused - 1)
                };
            }
            Key::Right | Key::Char('l') => {
                if focused < 1 {
                    state.button_focus = Some(focused + 1);
                }
            }
            Key::Tab => {
                state.button_focus = if focused == 1 {
                    None
                } else {
                    Some(focused + 1)
                };
            }
            Key::ShiftTab => {
                state.button_focus = if focused == 0 {
                    None
                } else {
                    Some(focused - 1)
                };
            }
            Key::Up => {
                state.button_focus = None;
            }
            Key::Enter if focused == 0 => commit_root_selection(state, installed_at),
            Key::Enter => {
                state.done = true;
                state.confirmed = false;
            }
            Key::Char('q') => {
                state.done = true;
                state.confirmed = false;
            }
            Key::Escape => {
                state.step = state.back_stack.pop().unwrap_or(Step::InstallOrUninstall);
                state.button_focus = None;
            }
            _ => {}
        }
        return;
    }
    let rows = state.root_rows();
    match key {
        Key::Up | Key::Char('k') => {
            state.cursor = state.cursor.saturating_sub(1);
        }
        Key::Down | Key::Char('j') => {
            state.cursor = (state.cursor + 1).min(rows - 1);
        }
        Key::Tab | Key::Right | Key::Char('l') => {
            state.button_focus = Some(0);
        }
        Key::ShiftTab => {
            state.button_focus = Some(1); // reverses in from the list: lands on the LAST button
        }
        Key::Space => {
            if state.cursor < state.available.len() {
                state.checked[state.cursor] = !state.checked[state.cursor];
            }
        }
        Key::Enter => {
            if state.cursor == state.available.len() {
                state.custom_input.clear();
                state.custom_pending_create = None;
                state.back_stack.push(Step::RootSelect);
                state.step = Step::CustomPath;
                return;
            }
            // Enter commits whatever Space has already checked and moves
            // on -- it never checks the row under the cursor itself, so a
            // single Enter with nothing checked yet is a no-op refusal
            // rather than a surprise "oh, that selected it after all".
            commit_root_selection(state, installed_at);
        }
        Key::Char('q') => {
            state.done = true;
            state.confirmed = false;
        }
        Key::Escape => {
            state.step = state.back_stack.pop().unwrap_or(Step::InstallOrUninstall);
        }
        _ => {}
    }
}

/// What Enter (or the footer's "[ Install now ]" button) on `RootSelect`
/// actually commits: refuses by name when nothing is checked, finishes
/// outright for Uninstall (the separate uninstall picker asks which skill,
/// working from whichever root is checked), and for Install looks up
/// what is already at the FIRST checked root -- lazily, only now that a
/// destination is actually known -- to decide whether `UsePrevious` needs
/// asking at all. This used to run from `InstallOrUninstall`'s own Enter,
/// back when that step came after this one; it moved here when the flow
/// was reordered to ask install-vs-uninstall first, since a destination
/// (and so `installed_at`'s answer) does not exist yet at that point.
fn commit_root_selection(
    state: &mut WizardState,
    installed_at: &impl Fn(&std::path::Path) -> Vec<InstalledSkill>,
) {
    if !state.checked.iter().any(|c| *c) {
        state.message = Some("Pick at least one destination (Space), or add a custom one.".into());
        return;
    }
    if state.uninstall_cursor == 1 {
        state.done = true;
        state.confirmed = true;
        return;
    }
    let first_checked = state
        .available
        .iter()
        .zip(state.checked.iter())
        .find(|(_, checked)| **checked)
        .map(|(root, _)| root.path.clone());
    let installed = first_checked.map(|p| installed_at(&p)).unwrap_or_default();
    if installed.is_empty() {
        state.done = true;
        state.confirmed = true;
        state.use_previous = false;
    } else {
        state.installed_here = installed;
        state.use_previous_cursor = 0;
        state.button_focus = None;
        state.back_stack.push(Step::RootSelect);
        state.step = Step::UsePrevious;
    }
}

fn handle_custom_path(state: &mut WizardState, key: Key) {
    if let Some(path) = state.custom_pending_create.clone() {
        match key {
            Key::Char('y') | Key::Char('Y') | Key::Enter => {
                if std::fs::create_dir_all(&path).is_err() {
                    state.message = Some(format!("Could not create {}", path.display()));
                    state.custom_pending_create = None;
                    return;
                }
                add_custom_root(state, path);
            }
            Key::Char('n') | Key::Char('N') | Key::Escape => {
                state.custom_pending_create = None;
            }
            Key::Char('q') => {
                state.done = true;
                state.confirmed = false;
            }
            _ => {}
        }
        return;
    }
    match key {
        Key::Enter => {
            let typed = state.custom_input.trim();
            if typed.is_empty() {
                state.message = Some("A directory is required.".into());
                return;
            }
            let path = PathBuf::from(typed);
            if !path.is_absolute() {
                state.message = Some("The directory must be an absolute path.".into());
                return;
            }
            if path.is_dir() {
                add_custom_root(state, path);
            } else {
                state.custom_pending_create = Some(path);
            }
        }
        Key::Escape => {
            state.step = state.back_stack.pop().unwrap_or(Step::RootSelect);
        }
        Key::Backspace => {
            state.custom_input.pop();
        }
        Key::Char(c) => state.custom_input.push(c),
        Key::Space => state.custom_input.push(' '),
        _ => {}
    }
}

/// Adds `path` to the checkbox list (checked immediately -- the point of
/// typing it was to install there) and returns to `RootSelect` to show it.
/// Persisting it to `custom_locations` is the caller's job once `run`
/// returns with it in `Outcome::roots`, the same as the plain-text flow this
/// replaces left that persistence to `main.rs` rather than duplicating it
/// here.
fn add_custom_root(state: &mut WizardState, path: PathBuf) {
    state.newly_added_custom.push(path.clone());
    let label = format!("Custom: {}", path.display());
    state.available.push(AvailableRoot {
        path,
        label,
        kind: None,
        exists: true,
    });
    state.checked.push(true);
    state.cursor = state.available.len() - 1;
    state.custom_pending_create = None;
    state.step = state.back_stack.pop().unwrap_or(Step::RootSelect);
}

/// This is the FIRST step of the wizard, so unlike every other step there
/// is nowhere to go back to -- Escape quits exactly like `q` here, rather
/// than popping `back_stack` (which is empty at this point regardless).
/// Enter always advances to `RootSelect`, whichever of Install/Uninstall is
/// highlighted: that choice only decides what happens once a destination is
/// picked (`commit_root_selection` reads `uninstall_cursor` there), not
/// anything reachable from here.
fn handle_install_or_uninstall(state: &mut WizardState, key: Key) {
    match key {
        Key::Up | Key::Down | Key::Char('k') | Key::Char('j') => {
            state.uninstall_cursor = 1 - state.uninstall_cursor;
        }
        Key::Char('i') => state.uninstall_cursor = 0,
        Key::Char('u') => state.uninstall_cursor = 1,
        Key::Enter => {
            state.back_stack.push(Step::InstallOrUninstall);
            state.step = Step::RootSelect;
        }
        Key::Char('q') | Key::Escape => {
            state.done = true;
            state.confirmed = false;
        }
        _ => {}
    }
}

/// `y`/`n` always work, focused or not -- muscle memory from every earlier
/// version of this screen. Tab/Shift-Tab/Left/Right/Up mirror
/// `handle_root_select`'s own button-focus handling exactly (the two
/// screens share the same "list pane + two details-pane buttons" shape).
fn handle_use_previous(state: &mut WizardState, key: Key) {
    if let Some(focused) = state.button_focus {
        match key {
            // See `handle_root_select`'s own identical arms: Left at the
            // first button hands focus back to the list, Right at the last
            // is a no-op.
            Key::Left | Key::Char('h') => {
                state.button_focus = if focused == 0 {
                    None
                } else {
                    Some(focused - 1)
                };
            }
            Key::Right | Key::Char('l') => {
                if focused < 1 {
                    state.button_focus = Some(focused + 1);
                }
            }
            Key::Tab => {
                state.button_focus = if focused == 1 {
                    None
                } else {
                    Some(focused + 1)
                };
            }
            Key::ShiftTab => {
                state.button_focus = if focused == 0 {
                    None
                } else {
                    Some(focused - 1)
                };
            }
            Key::Up => {
                state.button_focus = None;
            }
            Key::Enter if focused == 0 => {
                state.use_previous = true;
                state.done = true;
                state.confirmed = true;
            }
            Key::Enter => {
                state.use_previous = false;
                state.done = true;
                state.confirmed = true;
            }
            Key::Char('y') | Key::Char('Y') => {
                state.use_previous = true;
                state.done = true;
                state.confirmed = true;
            }
            Key::Char('n') | Key::Char('N') => {
                state.use_previous = false;
                state.done = true;
                state.confirmed = true;
            }
            Key::Char('q') => {
                state.done = true;
                state.confirmed = false;
            }
            Key::Escape => {
                state.step = state.back_stack.pop().unwrap_or(Step::RootSelect);
                state.button_focus = None;
            }
            _ => {}
        }
        return;
    }
    let rows = state.installed_here.len().max(1);
    match key {
        Key::Up | Key::Char('k') => {
            state.use_previous_cursor = state.use_previous_cursor.saturating_sub(1);
        }
        Key::Down | Key::Char('j') => {
            state.use_previous_cursor = (state.use_previous_cursor + 1).min(rows - 1);
        }
        Key::Tab | Key::Right | Key::Char('l') => {
            state.button_focus = Some(0);
        }
        Key::ShiftTab => {
            state.button_focus = Some(1);
        }
        Key::Char('y') | Key::Char('Y') | Key::Enter => {
            state.use_previous = true;
            state.done = true;
            state.confirmed = true;
        }
        Key::Char('n') | Key::Char('N') => {
            state.use_previous = false;
            state.done = true;
            state.confirmed = true;
        }
        Key::Escape => {
            state.step = state.back_stack.pop().unwrap_or(Step::RootSelect);
        }
        Key::Char('q') => {
            state.done = true;
            state.confirmed = false;
        }
        _ => {}
    }
}

// ---- rendering --------------------------------------------------------

fn render(
    state: &WizardState,
    cols: usize,
    rows: usize,
    color_mode: super::mascot::ColorMode,
    unicode: bool,
) -> (Vec<String>, super::layout::Layout) {
    let color_capable = color_mode != super::mascot::ColorMode::None;
    match state.step {
        Step::RootSelect => root_select_frame(state, cols, rows, color_mode, unicode),
        Step::InstallOrUninstall => {
            install_or_uninstall_frame(state, cols, rows, color_capable, unicode)
        }
        Step::CustomPath => {
            let (title, body, hint) = custom_path_view(state, cols);
            single_pane_frame(cols, rows, color_capable, unicode, &title, body, &hint)
        }
        Step::UsePrevious => use_previous_frame(state, cols, rows, color_mode, unicode),
    }
}

/// A short, human explanation of what installing into this kind of root
/// means, for the details pane -- `None` (a custom location, never one of
/// `manifest::AGENTS`' own kinds) gets its own generic description rather
/// than an empty pane.
fn kind_description(kind: Option<&str>) -> &'static str {
    match kind {
        Some("universal") => {
            "Shared by every agent tool that looks here, so one install covers all of them at once."
        }
        Some("codex") => "Codex CLI's own skills directory.",
        Some("claude") => "Claude Code's own skills directory.",
        Some("opencode") => "OpenCode's own skills directory.",
        Some("openclaw") => "OpenClaw's own managed skills directory.",
        Some("cline") => "Cline's own skills directory.",
        _ => "A directory used before, or a new one about to be added below.",
    }
}

/// One line for a single checked root, in the details pane's own "Selected"
/// block -- distinct from `kind_description` (which explains what a KIND of
/// root IS, for someone still deciding) because this states what will
/// actually HAPPEN, past the decision rather than an explanation of it.
fn selection_summary_line(root: &AvailableRoot) -> String {
    if root.kind.as_deref() == Some("universal") {
        "Add a universal skill directory for other agents to look in.".to_string()
    } else if root.exists {
        format!("Update the existing install in {}", root.label)
    } else {
        format!("Fresh install in {}", root.label)
    }
}

const INSTALL_LABEL: &str = "[ Install now ]";
const CANCEL_LABEL: &str = "[ Cancel ]";
const BUTTON_GAP: &str = "    ";
/// A muted green and a muted red -- resting colors for the two buttons,
/// distinct from each other and from the default terminal background, so
/// they read as real buttons rather than plain bracketed text.
/// `ColorMode::None` renders neither (see `colorize_button`). Keyboard
/// focus (Tab) is shown with reverse video instead of a brighter color --
/// the same treatment the list's own cursor row already gets, so "this is
/// the focused control" always looks the same way regardless of which
/// pane it is in, and works even without color support.
const INSTALL_BUTTON_BG: (u8, u8, u8) = (25, 110, 60);
const CANCEL_BUTTON_BG: (u8, u8, u8) = (120, 45, 45);

/// Where the SECOND of two side-by-side buttons starts, in content columns,
/// given the first one's label -- shared by every screen with this "two
/// buttons, one row" shape (`RootSelect`, `UsePrevious`) so the layout math
/// and the click hit-test (`button_hit`) always agree on where the second
/// button actually begins.
fn second_button_col(first_label: &str) -> usize {
    first_label.chars().count() + BUTTON_GAP.chars().count()
}

/// A row of two side-by-side buttons, sized to `right_w` -- the DETAILS
/// pane's own width, so it sits directly under that pane's own summary
/// content rather than spanning the full frame. Already padded here (using
/// the PLAIN, uncolored text's visual width) rather than left to
/// `two_pane_frame`'s generic `pad`, which would miscount a colored line's
/// width by counting its invisible SGR bytes as display columns. Shared by
/// every "two buttons, one row" screen; `button_hit` is this same
/// construction read backwards, for click testing.
#[allow(clippy::too_many_arguments)]
fn two_button_line(
    mode: ColorMode,
    right_w: usize,
    focus: Option<usize>,
    first_label: &str,
    first_bg: (u8, u8, u8),
    second_label: &str,
    second_bg: (u8, u8, u8),
) -> String {
    let first = colorize_button(mode, first_label, first_bg, focus == Some(0));
    let second = colorize_button(mode, second_label, second_bg, focus == Some(1));
    let visual_len = second_button_col(first_label) + second_label.chars().count();
    let trailing = " ".repeat(right_w.saturating_sub(visual_len));
    format!("{first}{BUTTON_GAP}{second}{trailing}")
}

/// Which of the two buttons (0 or 1) a click's 0-based content column
/// landed on, if either -- the read-backwards counterpart to
/// `two_button_line`'s own layout, so a click is tested against exactly
/// the columns that construction actually produces.
fn button_hit(content_col: usize, first_label: &str, second_label: &str) -> Option<usize> {
    if content_col < first_label.chars().count() {
        return Some(0);
    }
    let second_start = second_button_col(first_label);
    if (second_start..second_start + second_label.chars().count()).contains(&content_col) {
        return Some(1);
    }
    None
}

fn root_select_buttons_line(mode: ColorMode, right_w: usize, focus: Option<usize>) -> String {
    two_button_line(
        mode,
        right_w,
        focus,
        INSTALL_LABEL,
        INSTALL_BUTTON_BG,
        CANCEL_LABEL,
        CANCEL_BUTTON_BG,
    )
}

/// Wraps a pane's raw lines to `width`, the rule `two_pane_frame` already
/// applies to any details content (an empty line stays one blank row
/// rather than `wrap` collapsing it away) -- except a line already carrying
/// an SGR escape (a colored, pre-padded button row) passes through
/// untouched, since `wrap`/`pad` both measure by character count and would
/// miscount one that includes invisible color bytes (`text::is_precomposed_line`).
fn wrap_pane_lines(lines: &[String], width: usize) -> Vec<String> {
    lines
        .iter()
        .flat_map(|line| {
            if line.is_empty() {
                vec![String::new()]
            } else if is_precomposed_line(line) {
                vec![line.clone()]
            } else {
                wrap(line, width)
            }
        })
        .collect()
}

/// Everything `RootSelect`'s DETAILS pane shows ABOVE its own buttons: the
/// highlighted item's own explanation, any transient message, a divider
/// rule, then a plain-language summary of every CHECKED root (never just
/// the cursor's). `right_w` sizes the divider to the pane's own width --
/// known only once `layout::compute` has run, hence the parameter rather
/// than a `WizardState`-only computation. Shared by the renderer and
/// `root_select_button_row_index` so the button row's position is always
/// derived from the SAME content, never a second guess at it.
fn root_select_details_raw(state: &WizardState, right_w: usize, unicode: bool) -> Vec<String> {
    let custom_index = state.available.len();
    let mut details: Vec<String> = if state.cursor == custom_index {
        vec![
            "+ Add a custom directory".to_string(),
            String::new(),
            "Type any absolute path on the next step. It will be".to_string(),
            "created if it does not exist yet, and remembered for".to_string(),
            "next time.".to_string(),
        ]
    } else {
        let root = &state.available[state.cursor];
        vec![
            root.label.clone(),
            String::new(),
            format!("Path: {}", root.path.display()),
            if root.exists {
                "[exists] -- already has skills installed, or ready to.".to_string()
            } else {
                "[will create] -- created the moment something installs".to_string()
            },
            kind_description(root.kind.as_deref()).to_string(),
        ]
    };
    if let Some(message) = &state.message {
        details.push(String::new());
        details.push(message.clone());
    }
    details.push(String::new());
    details.push(
        BorderSet::for_unicode(unicode)
            .horizontal
            .to_string()
            .repeat(right_w),
    );
    details.push(String::new());
    // What is actually going to happen -- every CHECKED root, never just
    // the one the cursor happens to be explaining above.
    if state.checked.iter().any(|c| *c) {
        details.push("Selected:".to_string());
        for (root, checked) in state.available.iter().zip(&state.checked) {
            if *checked {
                details.push(format!("  {}", selection_summary_line(root)));
            }
        }
    } else {
        details.push("Nothing selected yet -- pick a destination above.".to_string());
    }
    details
}

/// Where the button row lands in the DETAILS pane's own WRAPPED row
/// numbering (0-based, relative to the pane's first body row) -- it is
/// always the last row, since `root_select_frame` always appends exactly
/// one blank line then the button line after `root_select_details_raw`,
/// and the button line's own SGR content makes `wrap_pane_lines` pass it
/// through as a single row unchanged (see its own doc comment).
fn root_select_button_row_index(state: &WizardState, right_w: usize, unicode: bool) -> usize {
    let above = root_select_details_raw(state, right_w, unicode);
    wrap_pane_lines(&above, right_w).len() + 1 // + the blank line before the buttons
}

fn root_select_frame(
    state: &WizardState,
    cols: usize,
    rows: usize,
    color_mode: ColorMode,
    unicode: bool,
) -> (Vec<String>, super::layout::Layout) {
    let color_capable = color_mode != ColorMode::None;
    let mut labels: Vec<&str> = state.available.iter().map(|r| r.label.as_str()).collect();
    labels.push("+ Add a custom directory...");

    let mut list_rows: Vec<String> = state
        .available
        .iter()
        .enumerate()
        .map(|(i, root)| {
            let cursor = if i == state.cursor { '>' } else { ' ' };
            let checkbox = if state.checked[i] { "[x]" } else { "[ ]" };
            let tag = if root.exists {
                "[exists]"
            } else {
                "[will create]"
            };
            format!("{cursor}{checkbox} {} {tag}", root.label)
        })
        .collect();
    let custom_index = state.available.len();
    let custom_cursor = if state.cursor == custom_index {
        '>'
    } else {
        ' '
    };
    list_rows.push(format!("{custom_cursor}    + Add a custom directory..."));

    const TITLE: &str = "Choose where to install";
    const HINT: &str = " Up/Dn move  Space select  Tab focus  Enter continue  q quit";
    // A probe layout, purely to learn `right_w` before the button row (part
    // of `details_raw`, a parameter TO `two_pane_frame`) can be built --
    // `two_pane_frame` recomputes the identical layout internally from the
    // same inputs, so this can never drift from what actually renders.
    let probe = super::layout::compute(
        cols,
        rows,
        &labels,
        color_capable,
        overflow(TITLE, cols, 3).len(),
        overflow(HINT, cols, 3).len(),
        unicode,
    );

    let mut details = root_select_details_raw(state, probe.right_w, unicode);
    details.push(String::new());
    details.push(root_select_buttons_line(
        color_mode,
        probe.right_w,
        state.button_focus,
    ));

    two_pane_frame(
        cols,
        rows,
        color_capable,
        unicode,
        TITLE,
        "DESTINATIONS",
        "DETAILS",
        &labels,
        &list_rows,
        state.cursor,
        &details,
        HINT,
    )
}

/// The row (1-based, absolute) each of the two choices lands on -- shared
/// between `install_or_uninstall_frame` (which builds the frame in exactly
/// this order) and `handle_click` (which needs to know where a click landed
/// without re-deriving the frame). `layout.mascot_on` is the SAME flag the
/// frame itself uses to decide whether to reserve the sprite's rows, so the
/// two never disagree about whether the mascot is actually taking up space.
fn install_or_uninstall_click_rows(layout: &super::layout::Layout) -> (usize, usize) {
    let mascot_rows = if layout.mascot_on {
        mascot::HEIGHT + 1 // + the blank separator line below it
    } else {
        0
    };
    let install_row = mascot_rows + 3; // the question(1) + a blank line(1) + this row
    (install_row, install_row + 1)
}

/// The wizard's front door: no border box, no list/details split -- a
/// centered splash-style chooser, the mascot at the top (when there is
/// room for it -- `layout.mascot_on`, the same rule the list-based screens
/// already use), the question right under it, then the two choices and a
/// short explanation of whichever is highlighted. Indented to the mascot's
/// own left edge rather than independently centered, so the text block
/// reads as sitting "under" the sprite rather than merely nearby it.
fn install_or_uninstall_frame(
    state: &WizardState,
    cols: usize,
    rows: usize,
    color_capable: bool,
    unicode: bool,
) -> (Vec<String>, super::layout::Layout) {
    let hint = " Up/Dn move  Enter choose  q quit";
    let hint_lines = overflow(hint, cols, 3);
    // Reused purely for its `mascot_on` rule (the same spare-body-rows test
    // the list-based screens apply) -- this screen has no border box of its
    // own, so `body_rows`/`left_w`/`right_w`/`narrow` go unused, the same
    // precedent `single_pane_frame` already established.
    let layout =
        super::layout::compute(cols, rows, &[], color_capable, 1, hint_lines.len(), unicode);
    let margin = cols.saturating_sub(mascot::WIDTH) / 2;
    let indent = " ".repeat(margin);
    // +1 leading blank (so the sprite doesn't start flush against the
    // terminal's own top edge -- it used to "hug the ceiling") + the
    // sprite's own height + 1 trailing blank before the question text.
    let mascot_rows = if layout.mascot_on {
        mascot::HEIGHT + 2
    } else {
        0
    };

    let mut out: Vec<String> = Vec::with_capacity(rows);
    for _ in 0..mascot_rows {
        out.push(pad("", cols));
    }
    out.push(pad(&format!("{indent}Install or uninstall skills?"), cols));
    out.push(pad("", cols));

    let choices = ["Install", "Uninstall"];
    for (i, label) in choices.iter().enumerate() {
        let marker = if state.uninstall_cursor == i {
            ">"
        } else {
            " "
        };
        let head = format!("{indent}{marker} ");
        let button = format!("[ {label} ]");
        let visual_len = head.chars().count() + button.chars().count();
        let trailing = " ".repeat(cols.saturating_sub(visual_len));
        let line = if state.uninstall_cursor == i {
            format!("{head}\x1b[7m{button}\x1b[0m{trailing}")
        } else {
            format!("{head}{button}{trailing}")
        };
        out.push(line);
    }
    out.push(pad("", cols));

    let description: [&str; 2] = if state.uninstall_cursor == 0 {
        [
            "Choose which skills to add, update, or reconfigure at",
            "the destination(s) just picked.",
        ]
    } else {
        [
            "Choose one previously installed skill to remove from",
            "the destination just picked.",
        ]
    };
    for line in description {
        out.push(pad(&format!("{indent}{line}"), cols));
    }

    while out.len() + hint_lines.len() < rows {
        out.push(pad("", cols));
    }
    out.extend(hint_lines);
    (out, layout)
}

fn custom_path_view(state: &WizardState, width: usize) -> (String, Vec<String>, String) {
    if let Some(path) = &state.custom_pending_create {
        let body = vec![
            pad(&format!("{} does not exist.", path.display()), width - 2),
            pad("Create it?", width - 2),
        ];
        return (
            "Custom directory".to_string(),
            body,
            " y create  n cancel  q quit".to_string(),
        );
    }
    let mut body = vec![pad(&format!("Path: {}_", state.custom_input), width - 2)];
    if let Some(message) = &state.message {
        body.push(pad("", width - 2));
        body.push(pad(message, width - 2));
    }
    (
        "Custom directory".to_string(),
        body,
        " Enter confirm  Esc cancel  q quit".to_string(),
    )
}

const USE_PREVIOUS_LABEL: &str = "[ Use these settings ]";
const START_FRESH_LABEL: &str = "[ Start fresh ]";
/// The same affirmative green `INSTALL_BUTTON_BG` uses -- "use these
/// settings" is the affirmative choice here too. A muted blue, not the same
/// red `CANCEL_BUTTON_BG` uses: "start fresh" is a different path forward,
/// not a cancel/destructive action, so it earns its own resting color
/// rather than borrowing one that means something else elsewhere.
const START_FRESH_BUTTON_BG: (u8, u8, u8) = (55, 90, 130);

/// A short, human explanation of what an integration mode means for the
/// DETAILS pane -- `None` (the near-total majority of skills, which offer
/// no mode choice at all) gets its own generic line rather than an empty
/// one, the same reason `kind_description` exists for `RootSelect`.
fn mode_description(mode: Option<&str>) -> String {
    match mode {
        Some("mcp") => {
            "Installed as an MCP server: your agent talks to it directly over a background connection, instead of running shell commands.".to_string()
        }
        Some("skill") => {
            "Installed as a plain skill: your agent runs its shell commands directly, no background server.".to_string()
        }
        Some(other) => format!("Installed in '{other}' integration mode."),
        None => "No separate integration mode for this skill -- installed as plain files.".to_string(),
    }
}

/// Everything `UsePrevious`'s DETAILS pane shows ABOVE its own buttons: the
/// highlighted skill's name, what its integration mode means, a divider
/// rule, then the question itself. `right_w` sizes the divider to the
/// pane's own width, the same reason `root_select_details_raw` takes it.
fn use_previous_details_raw(state: &WizardState, right_w: usize, unicode: bool) -> Vec<String> {
    let mut details = match state.installed_here.get(state.use_previous_cursor) {
        Some(skill) => vec![
            skill.name.clone(),
            String::new(),
            mode_description(skill.mode.as_deref()),
        ],
        None => vec!["Nothing was installed here yet.".to_string()],
    };
    details.push(String::new());
    details.push(
        BorderSet::for_unicode(unicode)
            .horizontal
            .to_string()
            .repeat(right_w),
    );
    details.push(String::new());
    details.push("Use these settings, or start fresh and choose again?".to_string());
    details
}

/// Where the button row lands in the DETAILS pane's own wrapped row
/// numbering -- the same reasoning as `root_select_button_row_index`: it
/// is always the last row, since `use_previous_frame` always appends
/// exactly one blank line then the button line after
/// `use_previous_details_raw`.
fn use_previous_button_row_index(state: &WizardState, right_w: usize, unicode: bool) -> usize {
    let above = use_previous_details_raw(state, right_w, unicode);
    wrap_pane_lines(&above, right_w).len() + 1 // + the blank line before the buttons
}

fn use_previous_frame(
    state: &WizardState,
    cols: usize,
    rows: usize,
    color_mode: ColorMode,
    unicode: bool,
) -> (Vec<String>, super::layout::Layout) {
    let color_capable = color_mode != ColorMode::None;
    let labels: Vec<&str> = state
        .installed_here
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    let list_rows: Vec<String> = state
        .installed_here
        .iter()
        .enumerate()
        .map(|(i, skill)| {
            let cursor = if i == state.use_previous_cursor {
                '>'
            } else {
                ' '
            };
            match &skill.mode {
                Some(mode) => format!("{cursor}{}  ({mode})", skill.name),
                None => format!("{cursor}{}", skill.name),
            }
        })
        .collect();

    const TITLE: &str = "Use previous settings?";
    const HINT: &str = " Up/Dn move  Tab focus  y use  n fresh  Esc back  q quit";
    let probe = super::layout::compute(
        cols,
        rows,
        &labels,
        color_capable,
        overflow(TITLE, cols, 3).len(),
        overflow(HINT, cols, 3).len(),
        unicode,
    );

    let mut details = use_previous_details_raw(state, probe.right_w, unicode);
    details.push(String::new());
    details.push(two_button_line(
        color_mode,
        probe.right_w,
        state.button_focus,
        USE_PREVIOUS_LABEL,
        INSTALL_BUTTON_BG,
        START_FRESH_LABEL,
        START_FRESH_BUTTON_BG,
    ));

    two_pane_frame(
        cols,
        rows,
        color_capable,
        unicode,
        TITLE,
        "INSTALLED",
        "DETAILS",
        &labels,
        &list_rows,
        state.use_previous_cursor,
        &details,
        HINT,
    )
}

/// A single full-width pane (`CustomPath` only now) -- no details pane,
/// since that step has no per-row "choice" for one to explain.
/// Still computes a `layout::Layout` (from an empty item list: its
/// `left_w`/`right_w`/`narrow` go unused here, only `body_rows`/
/// `list_rows`/`mascot_on` do) so the mascot placement rule stays the one
/// `draw_mascot` already implements, the same as every other step.
fn single_pane_frame(
    cols: usize,
    rows: usize,
    color_capable: bool,
    unicode: bool,
    title: &str,
    body: Vec<String>,
    hint: &str,
) -> (Vec<String>, super::layout::Layout) {
    let b = BorderSet::for_unicode(unicode);
    let inner = cols.saturating_sub(2).max(1);
    let title_lines = overflow(title, cols, 3);
    let hint_lines = overflow(hint, cols, 3);
    let layout = super::layout::compute(
        cols,
        rows,
        &[],
        color_capable,
        title_lines.len(),
        hint_lines.len(),
        unicode,
    );
    let mut out = Vec::with_capacity(rows);
    out.extend(title_lines);
    out.push(format!(
        "{}{}{}",
        b.corner_tl,
        std::iter::repeat_n(b.horizontal, inner).collect::<String>(),
        b.corner_tr
    ));
    for i in 0..layout.body_rows {
        let content = body.get(i).cloned().unwrap_or_else(|| pad("", inner));
        out.push(format!("{}{content}{}", b.vertical, b.vertical));
    }
    out.push(format!(
        "{}{}{}",
        b.corner_bl,
        std::iter::repeat_n(b.horizontal, inner).collect::<String>(),
        b.corner_br
    ));
    out.extend(overflow(hint, cols, 3));
    (out, layout)
}

/// A list pane + a details pane, the same visual shape (and using the same
/// `Layout`/`BorderSet` machinery) as the skill picker itself, since
/// `RootSelect` and `InstallOrUninstall` are the same KIND of screen the
/// picker is: a list of choices, and an explanation of whichever one has
/// the cursor. `list_item_labels` sizes the left pane's width exactly the
/// way the picker's own skill names do; `list_rows` are the already-built
/// (but not yet padded) row strings, one per item, in the SAME order as
/// `list_item_labels`; `details_raw` is the raw (unwrapped) lines to show
/// for the current `cursor` -- wrapped to the details pane's actual width
/// here, once that width is known, rather than by the caller.
///
/// The cursor row is wrapped in reverse video (`\x1b[7m`/`\x1b[0m`) around
/// its own already-padded content -- the row a real GUI would render as a
/// pressed/focused button. This is the one place a returned row's byte
/// length is not its display width (the SGR bytes are zero-width), which
/// is why the width-invariant tests below check the cursor row separately
/// from every other row instead of holding all of them to `.chars().count()`.
///
/// `details_raw` may itself contain a pre-colored, pre-padded-to-`right_w`
/// line (`RootSelect`'s own button row) -- `wrap_pane_lines` passes any
/// line already carrying an SGR escape through unchanged rather than
/// measuring it by character count, which would miscount its invisible
/// color bytes as display columns.
#[allow(clippy::too_many_arguments)]
fn two_pane_frame(
    cols: usize,
    rows: usize,
    color_capable: bool,
    unicode: bool,
    title: &str,
    list_label: &str,
    details_label: &str,
    list_item_labels: &[&str],
    list_rows: &[String],
    cursor: usize,
    details_raw: &[String],
    hint: &str,
) -> (Vec<String>, super::layout::Layout) {
    let b = BorderSet::for_unicode(unicode);
    let title_lines = overflow(title, cols, 3);
    let hint_lines = overflow(hint, cols, 3);
    let layout = super::layout::compute(
        cols,
        rows,
        list_item_labels,
        color_capable,
        title_lines.len(),
        hint_lines.len(),
        unicode,
    );

    let details: Vec<String> = wrap_pane_lines(details_raw, layout.right_w)
        .into_iter()
        .map(|line| {
            if is_precomposed_line(&line) {
                line
            } else {
                pad(&line, layout.right_w)
            }
        })
        .collect();

    let mut out = Vec::with_capacity(rows);
    out.extend(title_lines);
    out.push(format!(
        "{}{}{}{}{}",
        b.corner_tl,
        pad_center(list_label, layout.left_w, b.horizontal),
        b.divider_top,
        pad_center(details_label, layout.right_w, b.horizontal),
        b.corner_tr
    ));
    for body in 0..layout.body_rows {
        let list_content = list_rows.get(body).cloned().unwrap_or_default();
        let list_cell = pad(&list_content, layout.left_w);
        let list_cell = if body == cursor {
            format!("\x1b[7m{list_cell}\x1b[0m")
        } else {
            list_cell
        };
        let details_cell = details
            .get(body)
            .cloned()
            .unwrap_or_else(|| pad("", layout.right_w));
        out.push(format!(
            "{}{list_cell}{}{details_cell}{}",
            b.vertical, b.vertical, b.vertical
        ));
    }
    out.push(format!(
        "{}{}{}{}{}",
        b.corner_bl,
        std::iter::repeat_n(b.horizontal, layout.left_w).collect::<String>(),
        b.divider_bottom,
        std::iter::repeat_n(b.horizontal, layout.right_w).collect::<String>(),
        b.corner_br
    ));
    out.extend(overflow(hint, cols, 3));
    (out, layout)
}

/// Like `render.rs`'s private `pad_center_dash`, generalized to any fill
/// character (that module's own version is hardcoded to `-`, and is not
/// `pub(crate)`): centers `label` inside `width` by right-padding with
/// `fill`, or truncates it when `label` itself does not fit.
fn pad_center(label: &str, width: usize, fill: char) -> String {
    if label.len() >= width {
        return label[..width.min(label.len())].to_string();
    }
    format!(
        "{label}{}",
        std::iter::repeat_n(fill, width - label.len()).collect::<String>()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots(paths: &[&str]) -> Vec<AvailableRoot> {
        paths
            .iter()
            .map(|p| AvailableRoot {
                path: PathBuf::from(p),
                label: p.to_string(),
                kind: Some("test".to_string()),
                exists: true,
            })
            .collect()
    }

    fn no_prior_installs(_: &std::path::Path) -> Vec<InstalledSkill> {
        Vec::new()
    }

    /// A generously-sized 90x24 layout -- the same numbers the click tests
    /// below already assumed implicitly before `handle_key` took a layout
    /// at all. Its `body_rows` (~20) puts the footer well past every row
    /// number those tests use, so it never interferes with them; the footer
    /// tests that DO care compute their own layout from the real frame.
    fn test_layout() -> super::super::layout::Layout {
        super::super::layout::compute(90, 24, &[], true, 1, 1, false)
    }

    fn some_prior_installs(_: &std::path::Path) -> Vec<InstalledSkill> {
        vec![InstalledSkill {
            name: "todo".to_string(),
            mode: None,
        }]
    }

    #[test]
    fn space_toggles_the_checkbox_under_the_cursor() {
        let mut state = WizardState::new(roots(&["/a", "/b"]));
        handle_root_select(&mut state, Key::Space, &no_prior_installs);
        assert_eq!(state.checked, vec![true, false]);
        handle_root_select(&mut state, Key::Space, &no_prior_installs);
        assert_eq!(state.checked, vec![false, false]);
    }

    #[test]
    fn down_moves_the_cursor_but_clamps_at_the_last_row() {
        let mut state = WizardState::new(roots(&["/a", "/b"]));
        // two roots + the "add custom" row = 3 rows, indices 0..=2
        handle_root_select(&mut state, Key::Down, &no_prior_installs);
        handle_root_select(&mut state, Key::Down, &no_prior_installs);
        handle_root_select(&mut state, Key::Down, &no_prior_installs);
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn space_then_enter_finishes_when_nothing_was_previously_installed() {
        // RootSelect is now the SECOND step -- Enter here commits the
        // destination directly (install-vs-uninstall was already decided
        // on the earlier screen), rather than advancing to it.
        let mut state = WizardState::new(roots(&["/a"]));
        handle_root_select(&mut state, Key::Space, &no_prior_installs);
        handle_root_select(&mut state, Key::Enter, &no_prior_installs);
        assert!(state.checked[0]);
        assert!(state.done);
        assert!(state.confirmed);
        assert!(!state.use_previous);
    }

    #[test]
    fn bare_enter_with_nothing_space_checked_does_not_finish_or_auto_check() {
        let mut state = WizardState::new(roots(&["/a"]));
        handle_root_select(&mut state, Key::Enter, &no_prior_installs);
        assert!(!state.checked[0]);
        assert!(!state.done);
        assert!(state.message.is_some());
    }

    #[test]
    fn enter_on_the_custom_row_opens_custom_path() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.cursor = 1; // the "add custom" row, past the one real root
        handle_root_select(&mut state, Key::Enter, &no_prior_installs);
        assert!(matches!(state.step, Step::CustomPath));
    }

    #[test]
    fn q_from_root_select_quits_unconfirmed() {
        let mut state = WizardState::new(roots(&["/a"]));
        handle_root_select(&mut state, Key::Char('q'), &no_prior_installs);
        assert!(state.done);
        assert!(!state.confirmed);
    }

    #[test]
    fn escape_from_root_select_returns_to_install_or_uninstall() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.back_stack.push(Step::InstallOrUninstall);
        state.step = Step::RootSelect;
        handle_root_select(&mut state, Key::Escape, &no_prior_installs);
        assert!(matches!(state.step, Step::InstallOrUninstall));
    }

    #[test]
    fn typed_characters_build_up_the_custom_input() {
        let mut state = WizardState::new(roots(&[]));
        state.step = Step::CustomPath;
        handle_key(
            &mut state,
            Key::Char('/'),
            &test_layout(),
            &no_prior_installs,
        );
        handle_key(
            &mut state,
            Key::Char('x'),
            &test_layout(),
            &no_prior_installs,
        );
        assert_eq!(state.custom_input, "/x");
    }

    #[test]
    fn backspace_removes_the_last_typed_character() {
        let mut state = WizardState::new(roots(&[]));
        state.step = Step::CustomPath;
        state.custom_input = "/x".to_string();
        handle_key(
            &mut state,
            Key::Backspace,
            &test_layout(),
            &no_prior_installs,
        );
        assert_eq!(state.custom_input, "/");
    }

    #[test]
    fn an_empty_custom_path_is_refused() {
        let mut state = WizardState::new(roots(&[]));
        state.step = Step::CustomPath;
        handle_custom_path(&mut state, Key::Enter);
        assert!(state.message.is_some());
        assert!(matches!(state.step, Step::CustomPath));
    }

    #[test]
    fn a_relative_custom_path_is_refused() {
        let mut state = WizardState::new(roots(&[]));
        state.step = Step::CustomPath;
        state.custom_input = "relative/dir".to_string();
        handle_custom_path(&mut state, Key::Enter);
        assert!(state.message.is_some());
        assert!(matches!(state.step, Step::CustomPath));
    }

    #[test]
    fn a_nonexistent_absolute_path_asks_to_create_it() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("not-yet-there");
        let mut state = WizardState::new(roots(&[]));
        state.step = Step::CustomPath;
        state.custom_input = missing.display().to_string();
        handle_custom_path(&mut state, Key::Enter);
        assert_eq!(
            state.custom_pending_create.as_deref(),
            Some(missing.as_path())
        );
    }

    #[test]
    fn confirming_create_adds_it_checked_and_records_it_as_newly_added() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("not-yet-there");
        let mut state = WizardState::new(roots(&[]));
        state.back_stack.push(Step::RootSelect);
        state.step = Step::CustomPath;
        state.custom_pending_create = Some(missing.clone());
        handle_custom_path(&mut state, Key::Char('y'));
        assert!(missing.is_dir());
        assert_eq!(state.available.len(), 1);
        assert_eq!(state.available[0].path, missing);
        assert_eq!(state.checked, vec![true]);
        assert_eq!(state.newly_added_custom, vec![missing]);
        assert!(matches!(state.step, Step::RootSelect));
    }

    #[test]
    fn declining_create_returns_to_the_text_field_with_nothing_added() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("not-yet-there");
        let mut state = WizardState::new(roots(&[]));
        state.step = Step::CustomPath;
        state.custom_pending_create = Some(missing.clone());
        handle_custom_path(&mut state, Key::Char('n'));
        assert!(!missing.is_dir());
        assert!(state.available.is_empty());
        assert!(state.custom_pending_create.is_none());
    }

    #[test]
    fn escape_from_custom_path_returns_to_root_select_discarding_input() {
        let mut state = WizardState::new(roots(&[]));
        state.back_stack.push(Step::RootSelect);
        state.step = Step::CustomPath;
        state.custom_input = "/typed/but/abandoned".to_string();
        handle_custom_path(&mut state, Key::Escape);
        assert!(matches!(state.step, Step::RootSelect));
    }

    #[test]
    fn up_and_down_both_flip_between_install_and_uninstall() {
        let mut state = WizardState::new(roots(&["/a"]));
        assert_eq!(state.uninstall_cursor, 0);
        handle_install_or_uninstall(&mut state, Key::Down);
        assert_eq!(state.uninstall_cursor, 1);
        handle_install_or_uninstall(&mut state, Key::Up);
        assert_eq!(state.uninstall_cursor, 0);
    }

    #[test]
    fn enter_on_install_or_uninstall_always_advances_to_root_select() {
        // Whichever choice is highlighted, this step's own job is only to
        // record it (`commit_root_selection`, run once a destination is
        // actually known, is what acts on it) -- Enter here just moves on.
        for cursor in [0, 1] {
            let mut state = WizardState::new(roots(&["/a"]));
            state.uninstall_cursor = cursor;
            handle_install_or_uninstall(&mut state, Key::Enter);
            assert!(matches!(state.step, Step::RootSelect));
            assert_eq!(state.uninstall_cursor, cursor);
        }
    }

    #[test]
    fn escape_from_install_or_uninstall_quits_unconfirmed() {
        // The first step: nothing to go back to, so Escape quits exactly
        // like `q` rather than popping an empty back_stack.
        let mut state = WizardState::new(roots(&["/a"]));
        handle_install_or_uninstall(&mut state, Key::Escape);
        assert!(state.done);
        assert!(!state.confirmed);
    }

    #[test]
    fn q_from_install_or_uninstall_quits_unconfirmed() {
        let mut state = WizardState::new(roots(&["/a"]));
        handle_install_or_uninstall(&mut state, Key::Char('q'));
        assert!(state.done);
        assert!(!state.confirmed);
    }

    #[test]
    fn commit_root_selection_refuses_when_nothing_is_checked() {
        let mut state = WizardState::new(roots(&["/a"]));
        commit_root_selection(&mut state, &no_prior_installs);
        assert!(!state.done);
        assert!(state.message.is_some());
    }

    #[test]
    fn commit_root_selection_finishes_immediately_for_uninstall() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.checked[0] = true;
        state.uninstall_cursor = 1;
        commit_root_selection(&mut state, &some_prior_installs);
        assert!(state.done);
        assert!(state.confirmed);
    }

    #[test]
    fn commit_root_selection_finishes_install_without_asking_when_nothing_was_installed_there() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.checked[0] = true;
        commit_root_selection(&mut state, &no_prior_installs);
        assert!(state.done);
        assert!(state.confirmed);
        assert!(!state.use_previous);
    }

    #[test]
    fn commit_root_selection_asks_first_when_something_is_already_installed_there() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.checked[0] = true;
        commit_root_selection(&mut state, &some_prior_installs);
        assert!(!state.done);
        assert!(matches!(state.step, Step::UsePrevious));
        assert_eq!(state.installed_here.len(), 1);
        assert_eq!(state.installed_here[0].name, "todo");
    }

    #[test]
    fn y_accepts_previous_settings_and_finishes() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::UsePrevious;
        handle_use_previous(&mut state, Key::Char('y'));
        assert!(state.done);
        assert!(state.confirmed);
        assert!(state.use_previous);
    }

    #[test]
    fn n_declines_previous_settings_and_still_finishes() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::UsePrevious;
        handle_use_previous(&mut state, Key::Char('n'));
        assert!(state.done);
        assert!(state.confirmed);
        assert!(!state.use_previous);
    }

    #[test]
    fn escape_from_use_previous_returns_to_root_select() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.back_stack.push(Step::RootSelect);
        state.step = Step::UsePrevious;
        handle_use_previous(&mut state, Key::Escape);
        assert!(matches!(state.step, Step::RootSelect));
    }

    #[test]
    fn a_new_key_press_clears_the_previous_steps_message() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.message = Some("stale warning".to_string());
        handle_key(&mut state, Key::Down, &test_layout(), &no_prior_installs);
        assert!(state.message.is_none());
    }

    // Clicks: body row 0 sits at absolute row 3 with the assumed one-line
    // title (1) + top border (1) -- the same TITLE_LINES constant
    // `handle_click` itself uses.

    #[test]
    fn clicking_a_root_row_moves_the_cursor_there_and_toggles_it() {
        let mut state = WizardState::new(roots(&["/a", "/b"]));
        state.step = Step::RootSelect;
        handle_key(
            &mut state,
            Key::Click { col: 2, row: 4 },
            &test_layout(),
            &no_prior_installs,
        );
        assert_eq!(state.cursor, 1);
        assert!(state.checked[1]);
        assert!(!state.checked[0]);
    }

    #[test]
    fn clicking_the_add_custom_row_opens_custom_path() {
        let mut state = WizardState::new(roots(&["/a", "/b"]));
        state.step = Step::RootSelect;
        // two roots -> the custom row is body row 2, absolute row 5
        handle_key(
            &mut state,
            Key::Click { col: 2, row: 5 },
            &test_layout(),
            &no_prior_installs,
        );
        assert!(matches!(state.step, Step::CustomPath));
    }

    #[test]
    fn clicking_the_left_border_does_nothing() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::RootSelect;
        handle_key(
            &mut state,
            Key::Click { col: 1, row: 3 },
            &test_layout(),
            &no_prior_installs,
        );
        assert!(!state.checked[0]);
        assert!(matches!(state.step, Step::RootSelect));
    }

    #[test]
    fn clicking_install_advances_to_root_select_like_enter() {
        // InstallOrUninstall is the wizard's default first step, so no
        // explicit `state.step` assignment is needed here.
        let mut state = WizardState::new(roots(&["/a"]));
        let (install_row, _) = install_or_uninstall_click_rows(&test_layout());
        handle_key(
            &mut state,
            Key::Click {
                col: 2,
                row: install_row as u16,
            },
            &test_layout(),
            &no_prior_installs,
        );
        assert!(matches!(state.step, Step::RootSelect));
        assert_eq!(state.uninstall_cursor, 0);
    }

    #[test]
    fn clicking_uninstall_advances_to_root_select_like_enter() {
        let mut state = WizardState::new(roots(&["/a"]));
        let (_, uninstall_row) = install_or_uninstall_click_rows(&test_layout());
        handle_key(
            &mut state,
            Key::Click {
                col: 2,
                row: uninstall_row as u16,
            },
            &test_layout(),
            &no_prior_installs,
        );
        assert!(matches!(state.step, Step::RootSelect));
        assert_eq!(state.uninstall_cursor, 1);
    }

    #[test]
    fn root_select_frame_renders_every_root_and_the_custom_row() {
        let mut state = WizardState::new(roots(&["/a", "/b"]));
        state.checked[0] = true;
        let (frame, _layout) = root_select_frame(&state, 90, 24, ColorMode::TrueColor, false);
        let joined = frame.join("\n");
        assert!(joined.contains("Choose where to install"));
        assert!(joined.contains("[x]"));
        assert!(joined.contains("/a"));
        assert!(joined.contains("[ ]"));
        assert!(joined.contains("custom directory"));
        // the highlighted (cursor) row is wrapped in reverse video
        assert!(joined.contains("\x1b[7m"));
    }

    #[test]
    fn root_select_frame_details_pane_explains_the_highlighted_root() {
        let state = WizardState::new(roots(&["/a", "/b"]));
        let (frame, _layout) = root_select_frame(&state, 90, 24, ColorMode::TrueColor, false);
        let joined = frame.join("\n");
        // cursor starts on the first root ("/a"); its path and kind
        // description must appear in the details pane. The fixture's own
        // "test" kind matches none of kind_description's real branches, so
        // its default ("a directory used before...") is what should show.
        assert!(joined.contains("Path: /a"));
        // a single word, so word-wrapping at this width cannot split it
        assert!(joined.contains("directory"));
    }

    #[test]
    fn root_select_frame_details_pane_explains_the_custom_row() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.cursor = 1; // the "add custom" row
        let (frame, _layout) = root_select_frame(&state, 90, 24, ColorMode::TrueColor, false);
        let joined = frame.join("\n");
        assert!(joined.contains("Add a custom directory"));
        assert!(joined.contains("remembered for"));
    }

    #[test]
    fn root_select_frame_details_pane_summarizes_every_checked_root_not_just_the_cursor() {
        let mut state = WizardState::new(roots(&["/a", "/b"]));
        // roots() defaults every fixture to exists:true; flip one so both
        // wordings (fresh vs. update) are exercised in the same test.
        state.available[0].exists = false;
        state.checked = vec![true, true];
        state.cursor = 0; // the details pane above only ever explains "/a"
        let (frame, _layout) = root_select_frame(&state, 90, 24, ColorMode::TrueColor, false);
        let joined = frame.join("\n");
        assert!(joined.contains("Selected:"));
        assert!(joined.contains("Fresh install in /a"));
        assert!(joined.contains("Update the existing install in /b"));
        assert!(joined.contains("[ Install now ]"));
        assert!(joined.contains("[ Cancel ]"));
    }

    #[test]
    fn root_select_frame_says_nothing_selected_when_nothing_is_checked() {
        let state = WizardState::new(roots(&["/a"]));
        let (frame, _layout) = root_select_frame(&state, 90, 24, ColorMode::TrueColor, false);
        let joined = frame.join("\n");
        assert!(joined.contains("Nothing selected yet"));
        assert!(!joined.contains("Selected:"));
        // Cancel is still offered with nothing checked; Install now still
        // renders too (clicking it gets the same "pick one" refusal Enter
        // already gives), so both buttons are always on screen.
        assert!(joined.contains("[ Install now ]"));
        assert!(joined.contains("[ Cancel ]"));
    }

    #[test]
    fn a_universal_root_gets_its_own_selected_wording_not_fresh_install() {
        let mut available = roots(&["/a"]);
        available[0].kind = Some("universal".to_string());
        let mut state = WizardState::new(available);
        state.checked = vec![true];
        let (frame, _layout) = root_select_frame(&state, 90, 24, ColorMode::TrueColor, false);
        let joined = frame.join("\n");
        assert!(joined.contains("Add a universal skill directory"));
        assert!(!joined.contains("Fresh install in"));
    }

    #[test]
    fn root_select_frame_draws_a_rule_between_the_item_details_and_selected() {
        let state = WizardState::new(roots(&["/a"]));
        let (frame, layout) = root_select_frame(&state, 90, 24, ColorMode::TrueColor, false);
        let joined = frame.join("\n");
        assert!(joined.contains(&"-".repeat(layout.right_w)));
    }

    #[test]
    fn root_select_frame_buttons_sit_under_the_selected_summary_not_full_width() {
        // The button row must be no wider than the details pane, and must
        // start at the details pane's own column -- not span the frame the
        // way the old full-width footer used to.
        let state = WizardState::new(roots(&["/a"]));
        let (frame, layout) = root_select_frame(&state, 90, 24, ColorMode::TrueColor, false);
        let button_line = frame
            .iter()
            .find(|l| l.contains(INSTALL_LABEL))
            .expect("a rendered line must contain the Install button");
        // The details pane's own left border sits at column layout.left_w+2
        // (0-based index layout.left_w+1); nothing left of it should carry
        // button text.
        let details_col = layout.left_w + 2;
        let before_details: String = button_line.chars().take(details_col).collect();
        assert!(
            !before_details.contains('['),
            "button text leaked left of the details pane: {button_line:?}"
        );
    }

    #[test]
    fn clicking_install_now_commits_even_while_the_custom_row_is_highlighted() {
        // Regression: Install-now used to route through the same Enter
        // handling the list uses, which special-cases the cursor sitting on
        // the "add custom directory" row -- so clicking the button while
        // that row was highlighted opened the custom-path text field
        // instead of committing the install.
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::RootSelect;
        state.checked[0] = true;
        state.cursor = state.available.len(); // the "+ Add a custom directory" row
        let (_frame, layout) = root_select_frame(&state, 90, 24, ColorMode::TrueColor, false);
        let body_start = 3;
        let button_row = (body_start
            + root_select_button_row_index(&state, layout.right_w, layout.unicode_borders))
            as u16;
        let details_start_col = (layout.left_w + 3) as u16;
        handle_key(
            &mut state,
            Key::Click {
                col: details_start_col,
                row: button_row,
            },
            &layout,
            &no_prior_installs,
        );
        assert!(state.done);
        assert!(state.confirmed);
        assert!(!matches!(state.step, Step::CustomPath));
    }

    #[test]
    fn tab_from_the_list_moves_focus_to_install_then_cancel_then_back() {
        let mut state = WizardState::new(roots(&["/a"]));
        assert_eq!(state.button_focus, None);
        handle_root_select(&mut state, Key::Tab, &no_prior_installs);
        assert_eq!(state.button_focus, Some(0));
        handle_root_select(&mut state, Key::Tab, &no_prior_installs);
        assert_eq!(state.button_focus, Some(1));
        handle_root_select(&mut state, Key::Tab, &no_prior_installs);
        assert_eq!(state.button_focus, None);
    }

    #[test]
    fn shift_tab_cycles_the_opposite_way() {
        let mut state = WizardState::new(roots(&["/a"]));
        handle_root_select(&mut state, Key::ShiftTab, &no_prior_installs);
        assert_eq!(state.button_focus, Some(1));
        handle_root_select(&mut state, Key::ShiftTab, &no_prior_installs);
        assert_eq!(state.button_focus, Some(0));
        handle_root_select(&mut state, Key::ShiftTab, &no_prior_installs);
        assert_eq!(state.button_focus, None);
    }

    #[test]
    fn left_and_right_switch_between_the_focused_buttons() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.button_focus = Some(0);
        handle_root_select(&mut state, Key::Right, &no_prior_installs);
        assert_eq!(state.button_focus, Some(1));
        handle_root_select(&mut state, Key::Left, &no_prior_installs);
        assert_eq!(state.button_focus, Some(0));
    }

    #[test]
    fn right_from_the_list_enters_button_focus_at_the_first_button() {
        let mut state = WizardState::new(roots(&["/a"]));
        assert_eq!(state.button_focus, None);
        handle_root_select(&mut state, Key::Right, &no_prior_installs);
        assert_eq!(state.button_focus, Some(0));
    }

    #[test]
    fn left_at_the_first_button_returns_focus_to_the_list() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.button_focus = Some(0);
        handle_root_select(&mut state, Key::Left, &no_prior_installs);
        assert_eq!(state.button_focus, None);
    }

    #[test]
    fn right_at_the_last_button_is_a_no_op() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.button_focus = Some(1);
        handle_root_select(&mut state, Key::Right, &no_prior_installs);
        assert_eq!(state.button_focus, Some(1));
    }

    #[test]
    fn up_from_a_focused_button_returns_focus_to_the_list() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.button_focus = Some(1);
        handle_root_select(&mut state, Key::Up, &no_prior_installs);
        assert_eq!(state.button_focus, None);
    }

    #[test]
    fn enter_while_install_is_focused_commits_the_selection() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.checked[0] = true;
        state.button_focus = Some(0);
        handle_root_select(&mut state, Key::Enter, &no_prior_installs);
        assert!(state.done);
        assert!(state.confirmed);
    }

    #[test]
    fn enter_while_cancel_is_focused_quits_unconfirmed() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.button_focus = Some(1);
        handle_root_select(&mut state, Key::Enter, &no_prior_installs);
        assert!(state.done);
        assert!(!state.confirmed);
    }

    #[test]
    fn the_focused_button_is_drawn_in_reverse_video() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.button_focus = Some(0);
        let (frame, _layout) = root_select_frame(&state, 90, 24, ColorMode::TrueColor, false);
        let button_line = frame
            .iter()
            .find(|l| l.contains(INSTALL_LABEL))
            .expect("a rendered line must contain the Install button");
        assert!(button_line.contains("\x1b[7m"));
    }

    #[test]
    fn neither_button_is_reverse_video_when_the_list_has_focus() {
        let state = WizardState::new(roots(&["/a"]));
        let (frame, _layout) = root_select_frame(&state, 90, 24, ColorMode::TrueColor, false);
        let button_line = frame
            .iter()
            .find(|l| l.contains(INSTALL_LABEL))
            .expect("a rendered line must contain the Install button");
        assert!(!button_line.contains("\x1b[7m"));
    }

    #[test]
    fn clicking_install_now_finishes_like_enter() {
        // RootSelect is the second step now, so Enter (or this button)
        // finishes the wizard directly rather than advancing further.
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::RootSelect;
        state.checked[0] = true;
        let (_frame, layout) = root_select_frame(&state, 90, 24, ColorMode::TrueColor, false);
        // handle_click's own body_start (TITLE_LINES(1) + 2): the first row
        // of the main box's body content. The button row lives in the
        // DETAILS pane, under the "Selected" summary -- always the LAST
        // wrapped row `root_select_details_raw` produces, plus the blank
        // line before it.
        let body_start = 3;
        let button_row = (body_start
            + root_select_button_row_index(&state, layout.right_w, layout.unicode_borders))
            as u16;
        let details_start_col = (layout.left_w + 3) as u16;
        // A click in the gap between the two buttons (content column
        // INSTALL_LABEL's length, the gap's own first column) hits neither.
        let gap_col = details_start_col + INSTALL_LABEL.chars().count() as u16;
        handle_key(
            &mut state,
            Key::Click {
                col: gap_col,
                row: button_row,
            },
            &layout,
            &no_prior_installs,
        );
        assert!(!state.done);
        handle_key(
            &mut state,
            Key::Click {
                col: details_start_col,
                row: button_row,
            },
            &layout,
            &no_prior_installs,
        );
        assert!(state.done);
        assert!(state.confirmed);
    }

    #[test]
    fn clicking_cancel_quits_unconfirmed() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::RootSelect;
        let (_frame, layout) = root_select_frame(&state, 90, 24, ColorMode::TrueColor, false);
        let body_start = 3;
        let button_row = (body_start
            + root_select_button_row_index(&state, layout.right_w, layout.unicode_borders))
            as u16;
        let details_start_col = (layout.left_w + 3) as u16;
        let cancel_col = details_start_col + second_button_col(INSTALL_LABEL) as u16;
        handle_key(
            &mut state,
            Key::Click {
                col: cancel_col,
                row: button_row,
            },
            &layout,
            &no_prior_installs,
        );
        assert!(state.done);
        assert!(!state.confirmed);
    }

    #[test]
    fn footer_rows_are_reserved_so_the_frame_never_exceeds_the_terminal_height() {
        let mut state = WizardState::new(roots(&["/a", "/b", "/c"]));
        state.checked = vec![true, true, true];
        let (frame, _layout) = root_select_frame(&state, 90, 24, ColorMode::TrueColor, false);
        assert!(
            frame.len() <= 24,
            "frame had {} lines for a 24-row terminal",
            frame.len()
        );
    }

    #[test]
    fn install_or_uninstall_frame_shows_both_choices_as_buttons() {
        let state = WizardState::new(roots(&["/a"]));
        let (frame, _layout) = install_or_uninstall_frame(&state, 90, 24, true, false);
        let joined = frame.join("\n");
        assert!(joined.contains("Install or uninstall skills?"));
        assert!(joined.contains("[ Install ]"));
        assert!(joined.contains("[ Uninstall ]"));
        assert!(joined.contains("\x1b[7m"));
        assert!(joined.contains("Choose which skills to add, update, or reconfigure"));
    }

    #[test]
    fn install_or_uninstall_frame_description_follows_the_highlighted_choice() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.uninstall_cursor = 1;
        let (frame, _layout) = install_or_uninstall_frame(&state, 90, 24, true, false);
        let joined = frame.join("\n");
        assert!(joined.contains("Choose one previously installed skill to remove"));
        assert!(!joined.contains("Choose which skills to add"));
    }

    #[test]
    fn install_or_uninstall_frame_never_exceeds_the_terminal_height() {
        let state = WizardState::new(roots(&["/a"]));
        for rows in [10, 24, 40] {
            let (frame, _layout) = install_or_uninstall_frame(&state, 90, rows, true, false);
            assert_eq!(frame.len(), rows, "rows={rows}");
        }
    }

    #[test]
    fn install_or_uninstall_frame_reserves_room_for_a_centered_mascot_at_a_tall_enough_terminal() {
        let state = WizardState::new(roots(&["/a"]));
        let (short_frame, short_layout) = install_or_uninstall_frame(&state, 90, 24, true, false);
        let (tall_frame, tall_layout) = install_or_uninstall_frame(&state, 90, 40, true, false);
        assert!(!short_layout.mascot_on);
        assert!(tall_layout.mascot_on);
        // The question line sits right after the mascot's reserved rows, so
        // a taller (mascot-on) frame pushes it further down than the short
        // one.
        let short_question_row = short_frame
            .iter()
            .position(|l| l.contains("Install or uninstall skills?"))
            .unwrap();
        let tall_question_row = tall_frame
            .iter()
            .position(|l| l.contains("Install or uninstall skills?"))
            .unwrap();
        assert!(tall_question_row > short_question_row);
    }

    #[test]
    fn install_or_uninstall_click_rows_moves_with_the_mascot() {
        let short_layout = super::super::layout::compute(90, 24, &[], true, 1, 1, false);
        let tall_layout = super::super::layout::compute(90, 40, &[], true, 1, 1, false);
        assert!(!short_layout.mascot_on);
        assert!(tall_layout.mascot_on);
        let (short_install, short_uninstall) = install_or_uninstall_click_rows(&short_layout);
        let (tall_install, tall_uninstall) = install_or_uninstall_click_rows(&tall_layout);
        assert_eq!(short_uninstall, short_install + 1);
        assert_eq!(tall_uninstall, tall_install + 1);
        assert!(tall_install > short_install);
    }

    #[test]
    fn the_wizard_starts_at_install_or_uninstall_not_root_select() {
        let state = WizardState::new(roots(&["/a"]));
        assert!(matches!(state.step, Step::InstallOrUninstall));
    }

    #[test]
    fn use_previous_frame_lists_every_installed_skill_and_its_mode() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::UsePrevious;
        state.installed_here = vec![
            InstalledSkill {
                name: "ai-text-editor".to_string(),
                mode: Some("mcp".to_string()),
            },
            InstalledSkill {
                name: "todo".to_string(),
                mode: None,
            },
        ];
        let (frame, _layout) = use_previous_frame(&state, 90, 24, ColorMode::TrueColor, false);
        let joined = frame.join("\n");
        assert!(joined.contains("ai-text-editor"));
        assert!(joined.contains("mcp"));
        let todo_line = frame.iter().find(|l| l.contains("todo")).unwrap();
        assert!(
            !todo_line.contains('('),
            "a mode-free skill should show no parenthetical: {todo_line:?}"
        );
    }

    #[test]
    fn use_previous_frame_explains_the_highlighted_skills_mode() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::UsePrevious;
        state.installed_here = vec![
            InstalledSkill {
                name: "ai-text-editor".to_string(),
                mode: Some("mcp".to_string()),
            },
            InstalledSkill {
                name: "todo".to_string(),
                mode: None,
            },
        ];
        let (frame, _layout) = use_previous_frame(&state, 90, 24, ColorMode::TrueColor, false);
        let joined = frame.join("\n");
        assert!(joined.contains("MCP server"));
        assert!(joined.contains("[ Use these settings ]"));
        assert!(joined.contains("[ Start fresh ]"));

        state.use_previous_cursor = 1;
        let (frame, _layout) = use_previous_frame(&state, 90, 24, ColorMode::TrueColor, false);
        let joined = frame.join("\n");
        assert!(joined.contains("No separate integration mode"));
    }

    #[test]
    fn tab_from_use_previous_list_moves_focus_to_the_buttons_and_back() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::UsePrevious;
        assert_eq!(state.button_focus, None);
        handle_use_previous(&mut state, Key::Tab);
        assert_eq!(state.button_focus, Some(0));
        handle_use_previous(&mut state, Key::Tab);
        assert_eq!(state.button_focus, Some(1));
        handle_use_previous(&mut state, Key::Tab);
        assert_eq!(state.button_focus, None);
    }

    #[test]
    fn right_from_the_use_previous_list_enters_button_focus() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::UsePrevious;
        handle_use_previous(&mut state, Key::Right);
        assert_eq!(state.button_focus, Some(0));
    }

    #[test]
    fn left_at_the_first_use_previous_button_returns_focus_to_the_list() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::UsePrevious;
        state.button_focus = Some(0);
        handle_use_previous(&mut state, Key::Left);
        assert_eq!(state.button_focus, None);
    }

    #[test]
    fn right_at_the_last_use_previous_button_is_a_no_op() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::UsePrevious;
        state.button_focus = Some(1);
        handle_use_previous(&mut state, Key::Right);
        assert_eq!(state.button_focus, Some(1));
    }

    #[test]
    fn enter_while_use_these_settings_is_focused_accepts_them() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::UsePrevious;
        state.button_focus = Some(0);
        handle_use_previous(&mut state, Key::Enter);
        assert!(state.done);
        assert!(state.confirmed);
        assert!(state.use_previous);
    }

    #[test]
    fn enter_while_start_fresh_is_focused_declines_them() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::UsePrevious;
        state.button_focus = Some(1);
        handle_use_previous(&mut state, Key::Enter);
        assert!(state.done);
        assert!(state.confirmed);
        assert!(!state.use_previous);
    }

    #[test]
    fn y_and_n_work_regardless_of_button_focus() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::UsePrevious;
        state.button_focus = Some(1); // Start fresh focused
        handle_use_previous(&mut state, Key::Char('y'));
        assert!(state.use_previous);
        assert!(state.done);
    }

    #[test]
    fn clicking_use_these_settings_accepts_them_even_with_a_different_skill_highlighted() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::UsePrevious;
        state.installed_here = vec![
            InstalledSkill {
                name: "ai-text-editor".to_string(),
                mode: Some("mcp".to_string()),
            },
            InstalledSkill {
                name: "todo".to_string(),
                mode: None,
            },
        ];
        state.use_previous_cursor = 1;
        let (_frame, layout) = use_previous_frame(&state, 90, 24, ColorMode::TrueColor, false);
        let body_start = 3;
        let button_row = (body_start
            + use_previous_button_row_index(&state, layout.right_w, layout.unicode_borders))
            as u16;
        let details_start_col = (layout.left_w + 3) as u16;
        handle_key(
            &mut state,
            Key::Click {
                col: details_start_col,
                row: button_row,
            },
            &layout,
            &no_prior_installs,
        );
        assert!(state.done);
        assert!(state.confirmed);
        assert!(state.use_previous);
    }

    #[test]
    fn clicking_a_skill_row_moves_the_cursor_there() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.step = Step::UsePrevious;
        state.installed_here = vec![
            InstalledSkill {
                name: "ai-text-editor".to_string(),
                mode: Some("mcp".to_string()),
            },
            InstalledSkill {
                name: "todo".to_string(),
                mode: None,
            },
        ];
        handle_key(
            &mut state,
            Key::Click { col: 2, row: 4 },
            &test_layout(),
            &no_prior_installs,
        );
        assert_eq!(state.use_previous_cursor, 1);
    }

    #[test]
    fn single_pane_frame_lines_are_all_exactly_cols_wide() {
        // Every real view function pads its own body lines to `cols - 2`
        // before handing them to `single_pane_frame` (`custom_path_view`
        // and its sibling both call `pad`); this fixture does the same,
        // the same contract `render.rs`'s own list/info cells rely on.
        for unicode in [false, true] {
            let (frame, _layout) =
                single_pane_frame(40, 12, true, unicode, "Title", vec![pad("one", 38)], "hint");
            for line in &frame {
                assert_eq!(line.chars().count(), 40, "line was: {line:?}");
            }
        }
    }

    #[test]
    fn two_pane_frame_divider_sits_at_the_left_panes_width() {
        let labels = ["alpha", "beta"];
        let rows = vec!["row a".to_string(), "row b".to_string()];
        let details = vec!["detail".to_string()];
        let (frame, layout) = two_pane_frame(
            90, 24, true, false, "Title", "LIST", "DETAILS", &labels, &rows, 0, &details, "hint",
        );
        assert!(!layout.narrow);
        // the top border row: corner, left_w horizontals, divider, right_w
        // horizontals, corner -- so the divider character sits right after
        // left_w plain '-' characters (ASCII mode here).
        let top = &frame[1];
        let chars: Vec<char> = top.chars().collect();
        assert_eq!(chars[1 + layout.left_w], '+');
    }

    #[test]
    fn two_pane_frame_only_the_cursor_row_is_reverse_video() {
        let labels = ["alpha", "beta"];
        let rows = vec!["row a".to_string(), "row b".to_string()];
        let details = vec!["detail".to_string()];
        let (frame, _layout) = two_pane_frame(
            90, 24, true, false, "Title", "LIST", "DETAILS", &labels, &rows, 1, &details, "hint",
        );
        // body starts right after the title line and the top border
        let cursor_row = &frame[3]; // title(1) + top border(1) + body row 1
        let other_row = &frame[2]; // body row 0
        assert!(cursor_row.contains("\x1b[7m"), "row was: {cursor_row:?}");
        assert!(!other_row.contains("\x1b[7m"), "row was: {other_row:?}");
    }

    #[test]
    fn two_pane_frame_non_cursor_rows_still_measure_exactly_cols_wide() {
        let labels = ["alpha", "beta"];
        let rows = vec!["row a".to_string(), "row b".to_string()];
        let details = vec!["detail".to_string()];
        let (frame, _layout) = two_pane_frame(
            90, 24, true, false, "Title", "LIST", "DETAILS", &labels, &rows, 0, &details, "hint",
        );
        for (i, line) in frame.iter().enumerate() {
            if line.contains("\x1b[7m") {
                continue; // the cursor row: SGR bytes are not display columns
            }
            assert_eq!(line.chars().count(), 90, "line {i} was: {line:?}");
        }
    }
}
