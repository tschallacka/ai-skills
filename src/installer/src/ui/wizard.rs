// MODE: DEV
// PACKAGE: PROD
//! The graphical front door: choosing where to install (or add a custom
//! directory), install vs. uninstall, and -- when the chosen root already
//! has something installed -- "use these settings?" listing exactly what is
//! already there. Everything that used to be asked as plain sequential text
//! before the skill picker ever started is a step of one continuous
//! full-screen flow here, in the same raw-mode/alternate-screen session the
//! picker already uses, with real back-navigation between these steps.
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

use super::input::{self, Key};
use super::render::BorderSet;
use super::terminal;
use super::text::{overflow, pad};
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
}

impl WizardState {
    fn new(available: Vec<AvailableRoot>) -> Self {
        let checked = vec![false; available.len()];
        WizardState {
            available,
            checked,
            cursor: 0,
            step: Step::RootSelect,
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
    let unicode = super::mascot::detect_utf8_capable();

    loop {
        let (cols, rows) = terminal::size();
        terminal::draw(&render(&state, cols, rows, unicode));

        match input::read_key(&rx) {
            Key::Tick => continue,
            Key::Eof => {
                state.done = true;
                state.confirmed = false;
            }
            key => handle_key(&mut state, key, &installed_at),
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
    installed_at: &impl Fn(&std::path::Path) -> Vec<InstalledSkill>,
) {
    // A message is transient feedback from the LAST action (e.g. "pick at
    // least one root"); any new key press clears it rather than letting a
    // stale warning linger over an unrelated next action.
    state.message = None;
    if let Key::Click { col, row } = key {
        handle_click(state, col, row, installed_at);
        return;
    }
    match state.step {
        Step::RootSelect => handle_root_select(state, key),
        Step::CustomPath => handle_custom_path(state, key),
        Step::InstallOrUninstall => handle_install_or_uninstall(state, key, installed_at),
        Step::UsePrevious => handle_use_previous(state, key),
    }
}

/// Maps a click onto the same effect the equivalent keys already have --
/// `RootSelect`'s rows are clickable checkboxes (and the "add custom" row),
/// `InstallOrUninstall`'s two rows are real buttons a single click commits,
/// exactly what "Install"/"Uninstall" look like. `CustomPath` (a text field)
/// and `UsePrevious` (no distinct button rows in its own view yet) stay
/// keyboard-only for now -- a real, deliberate scope boundary, not an
/// oversight.
///
/// Assumes every step's own title is exactly one line, true for every title
/// string this module actually uses at any width realistic enough to run a
/// terminal UI in at all (the longest, "Choose where to install", is 24
/// characters) -- an unrealistically narrow terminal could make a click
/// land one row off, never a crash or a corrupted frame.
fn handle_click(
    state: &mut WizardState,
    col: u16,
    row: u16,
    installed_at: &impl Fn(&std::path::Path) -> Vec<InstalledSkill>,
) {
    const TITLE_LINES: usize = 1;
    if col <= 1 || row == 0 {
        return; // col 1 is the left border; row 0 is never a real row
    }
    let body_start = TITLE_LINES + 2; // title line(s) + the top border, 1-based
    let row = row as usize;
    if row < body_start {
        return;
    }
    let body_row = row - body_start;
    match state.step {
        Step::RootSelect => {
            if body_row >= state.root_rows() {
                return;
            }
            state.cursor = body_row;
            if body_row == state.available.len() {
                state.custom_input.clear();
                state.custom_pending_create = None;
                state.back_stack.push(Step::RootSelect);
                state.step = Step::CustomPath;
            } else {
                state.checked[body_row] = !state.checked[body_row];
            }
        }
        Step::InstallOrUninstall => {
            if body_row > 1 {
                return;
            }
            state.uninstall_cursor = body_row;
            handle_install_or_uninstall(state, Key::Enter, installed_at);
        }
        Step::CustomPath | Step::UsePrevious => {}
    }
}

fn handle_root_select(state: &mut WizardState, key: Key) {
    let rows = state.root_rows();
    match key {
        Key::Up | Key::Char('k') => {
            state.cursor = state.cursor.saturating_sub(1);
        }
        Key::Down | Key::Char('j') => {
            state.cursor = (state.cursor + 1).min(rows - 1);
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
            if state.checked.iter().any(|c| *c) {
                state.back_stack.push(Step::RootSelect);
                state.step = Step::InstallOrUninstall;
            } else {
                state.message =
                    Some("Pick at least one destination (Space), or add a custom one.".into());
            }
        }
        Key::Char('q') | Key::Escape => {
            state.done = true;
            state.confirmed = false;
        }
        _ => {}
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

fn handle_install_or_uninstall(
    state: &mut WizardState,
    key: Key,
    installed_at: &impl Fn(&std::path::Path) -> Vec<InstalledSkill>,
) {
    match key {
        Key::Up | Key::Down | Key::Char('k') | Key::Char('j') => {
            state.uninstall_cursor = 1 - state.uninstall_cursor;
        }
        Key::Char('i') => state.uninstall_cursor = 0,
        Key::Char('u') => state.uninstall_cursor = 1,
        Key::Enter => {
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
                state.back_stack.push(Step::InstallOrUninstall);
                state.step = Step::UsePrevious;
            }
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

fn handle_use_previous(state: &mut WizardState, key: Key) {
    match key {
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
            state.step = state.back_stack.pop().unwrap_or(Step::InstallOrUninstall);
        }
        Key::Char('q') => {
            state.done = true;
            state.confirmed = false;
        }
        _ => {}
    }
}

// ---- rendering --------------------------------------------------------

fn render(state: &WizardState, cols: usize, rows: usize, unicode: bool) -> Vec<String> {
    let (title, body, hint) = match state.step {
        Step::RootSelect => root_select_view(state, cols),
        Step::CustomPath => custom_path_view(state, cols),
        Step::InstallOrUninstall => install_or_uninstall_view(state, cols),
        Step::UsePrevious => use_previous_view(state, cols),
    };
    bordered_frame(cols, rows, unicode, &title, body, &hint)
}

fn root_select_view(state: &WizardState, width: usize) -> (String, Vec<String>, String) {
    let mut body = Vec::new();
    for (i, root) in state.available.iter().enumerate() {
        let cursor = if i == state.cursor { '>' } else { ' ' };
        let checkbox = if state.checked[i] { "[x]" } else { "[ ]" };
        let tag = if root.exists {
            "[exists]"
        } else {
            "[will create]"
        };
        body.push(pad(
            &format!("{cursor}{checkbox} {} {tag}", root.label),
            width - 2,
        ));
    }
    let custom_cursor = if state.cursor == state.available.len() {
        '>'
    } else {
        ' '
    };
    body.push(pad(
        &format!("{custom_cursor}    + Add a custom directory..."),
        width - 2,
    ));
    if let Some(message) = &state.message {
        body.push(pad("", width - 2));
        body.push(pad(message, width - 2));
    }
    (
        "Choose where to install".to_string(),
        body,
        " Up/Dn move  Space select  Enter continue  q quit".to_string(),
    )
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

fn install_or_uninstall_view(state: &WizardState, width: usize) -> (String, Vec<String>, String) {
    let row = |index: usize, label: &str| {
        let cursor = if state.uninstall_cursor == index {
            '>'
        } else {
            ' '
        };
        pad(&format!("{cursor} {label}"), width - 2)
    };
    let body = vec![row(0, "Install"), row(1, "Uninstall")];
    (
        "Install or uninstall?".to_string(),
        body,
        " Up/Dn move  Enter choose  Esc back  q quit".to_string(),
    )
}

fn use_previous_view(state: &WizardState, width: usize) -> (String, Vec<String>, String) {
    let mut body = vec![pad("Already installed here:", width - 2)];
    for skill in &state.installed_here {
        let line = match &skill.mode {
            Some(mode) => format!("  {}  ({mode})", skill.name),
            None => format!("  {}", skill.name),
        };
        body.push(pad(&line, width - 2));
    }
    body.push(pad("", width - 2));
    body.push(pad("Use these settings?", width - 2));
    (
        "Use previous settings?".to_string(),
        body,
        " y use them  n start fresh  Esc back  q quit".to_string(),
    )
}

fn bordered_frame(
    cols: usize,
    rows: usize,
    unicode: bool,
    title: &str,
    body: Vec<String>,
    hint: &str,
) -> Vec<String> {
    let b = BorderSet::for_unicode(unicode);
    let inner = cols.saturating_sub(2).max(1);
    let mut out = Vec::with_capacity(rows);
    out.extend(overflow(title, cols, 3));
    out.push(format!(
        "{}{}{}",
        b.corner_tl,
        std::iter::repeat_n(b.horizontal, inner).collect::<String>(),
        b.corner_tr
    ));
    let hint_lines = overflow(hint, cols, 3).len();
    // `out` so far holds the title bar line(s) and the top border; one more
    // row is reserved for the bottom border, then `hint_lines` for the hint
    // bar -- whatever is left is the body's own row budget.
    let body_rows = rows.saturating_sub(out.len() + 1 + hint_lines);
    for i in 0..body_rows {
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
    out
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

    fn some_prior_installs(_: &std::path::Path) -> Vec<InstalledSkill> {
        vec![InstalledSkill {
            name: "todo".to_string(),
            mode: None,
        }]
    }

    #[test]
    fn space_toggles_the_checkbox_under_the_cursor() {
        let mut state = WizardState::new(roots(&["/a", "/b"]));
        handle_root_select(&mut state, Key::Space);
        assert_eq!(state.checked, vec![true, false]);
        handle_root_select(&mut state, Key::Space);
        assert_eq!(state.checked, vec![false, false]);
    }

    #[test]
    fn down_moves_the_cursor_but_clamps_at_the_last_row() {
        let mut state = WizardState::new(roots(&["/a", "/b"]));
        // two roots + the "add custom" row = 3 rows, indices 0..=2
        handle_root_select(&mut state, Key::Down);
        handle_root_select(&mut state, Key::Down);
        handle_root_select(&mut state, Key::Down);
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn space_then_enter_advances_to_install_or_uninstall() {
        let mut state = WizardState::new(roots(&["/a"]));
        handle_root_select(&mut state, Key::Space);
        handle_root_select(&mut state, Key::Enter);
        assert!(state.checked[0]);
        assert!(matches!(state.step, Step::InstallOrUninstall));
    }

    #[test]
    fn bare_enter_with_nothing_space_checked_does_not_advance_or_auto_check() {
        let mut state = WizardState::new(roots(&["/a"]));
        handle_root_select(&mut state, Key::Enter);
        assert!(!state.checked[0]);
        assert!(matches!(state.step, Step::RootSelect));
        assert!(state.message.is_some());
    }

    #[test]
    fn enter_on_the_custom_row_opens_custom_path() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.cursor = 1; // the "add custom" row, past the one real root
        handle_root_select(&mut state, Key::Enter);
        assert!(matches!(state.step, Step::CustomPath));
    }

    #[test]
    fn q_from_root_select_quits_unconfirmed() {
        let mut state = WizardState::new(roots(&["/a"]));
        handle_root_select(&mut state, Key::Char('q'));
        assert!(state.done);
        assert!(!state.confirmed);
    }

    #[test]
    fn typed_characters_build_up_the_custom_input() {
        let mut state = WizardState::new(roots(&[]));
        state.step = Step::CustomPath;
        handle_key(&mut state, Key::Char('/'), &no_prior_installs);
        handle_key(&mut state, Key::Char('x'), &no_prior_installs);
        assert_eq!(state.custom_input, "/x");
    }

    #[test]
    fn backspace_removes_the_last_typed_character() {
        let mut state = WizardState::new(roots(&[]));
        state.step = Step::CustomPath;
        state.custom_input = "/x".to_string();
        handle_key(&mut state, Key::Backspace, &no_prior_installs);
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
        state.step = Step::InstallOrUninstall;
        assert_eq!(state.uninstall_cursor, 0);
        handle_install_or_uninstall(&mut state, Key::Down, &no_prior_installs);
        assert_eq!(state.uninstall_cursor, 1);
        handle_install_or_uninstall(&mut state, Key::Up, &no_prior_installs);
        assert_eq!(state.uninstall_cursor, 0);
    }

    #[test]
    fn choosing_uninstall_finishes_immediately_with_no_use_previous_step() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.checked[0] = true;
        state.step = Step::InstallOrUninstall;
        state.uninstall_cursor = 1;
        handle_install_or_uninstall(&mut state, Key::Enter, &some_prior_installs);
        assert!(state.done);
        assert!(state.confirmed);
        assert!(matches!(state.step, Step::InstallOrUninstall));
    }

    #[test]
    fn choosing_install_with_nothing_installed_there_finishes_without_asking() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.checked[0] = true;
        state.step = Step::InstallOrUninstall;
        handle_install_or_uninstall(&mut state, Key::Enter, &no_prior_installs);
        assert!(state.done);
        assert!(state.confirmed);
        assert!(!state.use_previous);
    }

    #[test]
    fn choosing_install_with_something_already_installed_asks_first() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.checked[0] = true;
        state.step = Step::InstallOrUninstall;
        handle_install_or_uninstall(&mut state, Key::Enter, &some_prior_installs);
        assert!(!state.done);
        assert!(matches!(state.step, Step::UsePrevious));
        assert_eq!(state.installed_here.len(), 1);
        assert_eq!(state.installed_here[0].name, "todo");
    }

    #[test]
    fn escape_from_install_or_uninstall_returns_to_root_select() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.back_stack.push(Step::RootSelect);
        state.step = Step::InstallOrUninstall;
        handle_install_or_uninstall(&mut state, Key::Escape, &no_prior_installs);
        assert!(matches!(state.step, Step::RootSelect));
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
    fn escape_from_use_previous_returns_to_install_or_uninstall() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.back_stack.push(Step::InstallOrUninstall);
        state.step = Step::UsePrevious;
        handle_use_previous(&mut state, Key::Escape);
        assert!(matches!(state.step, Step::InstallOrUninstall));
    }

    #[test]
    fn a_new_key_press_clears_the_previous_steps_message() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.message = Some("stale warning".to_string());
        handle_key(&mut state, Key::Down, &no_prior_installs);
        assert!(state.message.is_none());
    }

    // Clicks: body row 0 sits at absolute row 3 with the assumed one-line
    // title (1) + top border (1) -- the same TITLE_LINES constant
    // `handle_click` itself uses.

    #[test]
    fn clicking_a_root_row_moves_the_cursor_there_and_toggles_it() {
        let mut state = WizardState::new(roots(&["/a", "/b"]));
        handle_key(
            &mut state,
            Key::Click { col: 2, row: 4 },
            &no_prior_installs,
        );
        assert_eq!(state.cursor, 1);
        assert!(state.checked[1]);
        assert!(!state.checked[0]);
    }

    #[test]
    fn clicking_the_add_custom_row_opens_custom_path() {
        let mut state = WizardState::new(roots(&["/a", "/b"]));
        // two roots -> the custom row is body row 2, absolute row 5
        handle_key(
            &mut state,
            Key::Click { col: 2, row: 5 },
            &no_prior_installs,
        );
        assert!(matches!(state.step, Step::CustomPath));
    }

    #[test]
    fn clicking_the_left_border_does_nothing() {
        let mut state = WizardState::new(roots(&["/a"]));
        handle_key(
            &mut state,
            Key::Click { col: 1, row: 3 },
            &no_prior_installs,
        );
        assert!(!state.checked[0]);
        assert!(matches!(state.step, Step::RootSelect));
    }

    #[test]
    fn clicking_install_commits_immediately_like_a_button() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.checked[0] = true;
        state.step = Step::InstallOrUninstall;
        handle_key(
            &mut state,
            Key::Click { col: 2, row: 3 },
            &no_prior_installs,
        );
        assert!(state.done);
        assert!(state.confirmed);
        assert_eq!(state.uninstall_cursor, 0);
    }

    #[test]
    fn clicking_uninstall_commits_immediately_like_a_button() {
        let mut state = WizardState::new(roots(&["/a"]));
        state.checked[0] = true;
        state.step = Step::InstallOrUninstall;
        handle_key(
            &mut state,
            Key::Click { col: 2, row: 4 },
            &some_prior_installs,
        );
        assert!(state.done);
        assert!(state.confirmed);
        assert_eq!(state.uninstall_cursor, 1);
    }

    #[test]
    fn root_select_view_renders_every_root_and_the_custom_row() {
        let mut state = WizardState::new(roots(&["/a", "/b"]));
        state.checked[0] = true;
        let (title, body, _hint) = root_select_view(&state, 40);
        assert!(title.contains("install"));
        assert!(body[0].contains("[x]"));
        assert!(body[0].contains("/a"));
        assert!(body[1].contains("[ ]"));
        assert!(body.last().unwrap().contains("custom directory"));
    }

    #[test]
    fn use_previous_view_lists_every_installed_skill_and_its_mode() {
        let mut state = WizardState::new(roots(&["/a"]));
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
        let (_title, body, _hint) = use_previous_view(&state, 40);
        assert!(body
            .iter()
            .any(|l| l.contains("ai-text-editor") && l.contains("mcp")));
        let todo_line = body.iter().find(|l| l.contains("todo")).unwrap();
        assert!(
            !todo_line.contains('('),
            "a mode-free skill should show no parenthetical: {todo_line:?}"
        );
    }

    #[test]
    fn bordered_frame_lines_are_all_exactly_cols_wide() {
        // Every real view function pads its own body lines to `cols - 2`
        // before handing them to `bordered_frame` (`root_select_view` and
        // its siblings all call `pad`); this fixture does the same, the
        // same contract `render.rs`'s own list/info cells rely on.
        for unicode in [false, true] {
            let frame = bordered_frame(40, 12, unicode, "Title", vec![pad("one", 38)], "hint");
            for line in &frame {
                assert_eq!(line.chars().count(), 40, "line was: {line:?}");
            }
        }
    }
}
