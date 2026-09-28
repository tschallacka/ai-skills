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
use super::text::{overflow, pad, wrap};
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
            super::draw_mascot(&layout, color_mode, eyes.current(), unicode);
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

fn render(
    state: &WizardState,
    cols: usize,
    rows: usize,
    color_mode: super::mascot::ColorMode,
    unicode: bool,
) -> (Vec<String>, super::layout::Layout) {
    let color_capable = color_mode != super::mascot::ColorMode::None;
    match state.step {
        Step::RootSelect => root_select_frame(state, cols, rows, color_capable, unicode),
        Step::InstallOrUninstall => {
            install_or_uninstall_frame(state, cols, rows, color_capable, unicode)
        }
        Step::CustomPath => {
            let (title, body, hint) = custom_path_view(state, cols);
            single_pane_frame(cols, rows, color_capable, unicode, &title, body, &hint)
        }
        Step::UsePrevious => {
            let (title, body, hint) = use_previous_view(state, cols);
            single_pane_frame(cols, rows, color_capable, unicode, &title, body, &hint)
        }
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

fn root_select_frame(
    state: &WizardState,
    cols: usize,
    rows: usize,
    color_capable: bool,
    unicode: bool,
) -> (Vec<String>, super::layout::Layout) {
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

    two_pane_frame(
        cols,
        rows,
        color_capable,
        unicode,
        "Choose where to install",
        "DESTINATIONS",
        "DETAILS",
        &labels,
        &list_rows,
        state.cursor,
        &details,
        " Up/Dn move  Space select  Enter continue  q quit",
    )
}

fn install_or_uninstall_frame(
    state: &WizardState,
    cols: usize,
    rows: usize,
    color_capable: bool,
    unicode: bool,
) -> (Vec<String>, super::layout::Layout) {
    let labels = ["Install", "Uninstall"];
    let list_rows: Vec<String> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let cursor = if state.uninstall_cursor == i {
                '>'
            } else {
                ' '
            };
            format!("{cursor} [ {label} ]")
        })
        .collect();
    let details: Vec<String> = if state.uninstall_cursor == 0 {
        vec![
            "Install".to_string(),
            String::new(),
            "Choose which skills to add, update, or reconfigure at".to_string(),
            "the destination(s) just picked.".to_string(),
        ]
    } else {
        vec![
            "Uninstall".to_string(),
            String::new(),
            "Choose one previously installed skill to remove from".to_string(),
            "the destination just picked.".to_string(),
        ]
    };

    two_pane_frame(
        cols,
        rows,
        color_capable,
        unicode,
        "Install or uninstall?",
        "CHOOSE",
        "DETAILS",
        &labels,
        &list_rows,
        state.uninstall_cursor,
        &details,
        " Up/Dn move  Enter choose  Esc back  q quit",
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

/// A single full-width pane (`CustomPath`, `UsePrevious`) -- no details
/// pane, since neither step has a per-row "choice" for one to explain.
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

    let details: Vec<String> = details_raw
        .iter()
        .flat_map(|line| {
            if line.is_empty() {
                vec![String::new()]
            } else {
                wrap(line, layout.right_w)
            }
        })
        .map(|line| pad(&line, layout.right_w))
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
    fn root_select_frame_renders_every_root_and_the_custom_row() {
        let mut state = WizardState::new(roots(&["/a", "/b"]));
        state.checked[0] = true;
        let (frame, _layout) = root_select_frame(&state, 90, 24, true, false);
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
        let (frame, _layout) = root_select_frame(&state, 90, 24, true, false);
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
        let (frame, _layout) = root_select_frame(&state, 90, 24, true, false);
        let joined = frame.join("\n");
        assert!(joined.contains("Add a custom directory"));
        assert!(joined.contains("remembered for"));
    }

    #[test]
    fn install_or_uninstall_frame_shows_both_choices_as_buttons() {
        let state = WizardState::new(roots(&["/a"]));
        let (frame, _layout) = install_or_uninstall_frame(&state, 90, 24, true, false);
        let joined = frame.join("\n");
        assert!(joined.contains("[ Install ]"));
        assert!(joined.contains("[ Uninstall ]"));
        assert!(joined.contains("\x1b[7m"));
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
