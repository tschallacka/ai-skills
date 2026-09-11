// MODE: DEV
// PACKAGE: PROD
//! Picker state -- selection, cursor, scroll, focus -- ported in spirit from
//! installer/src/35-ui-model.sh's IUI_* state, trimmed to what this slice
//! actually drives: [[requirements]] supplies the per-skill ok/degraded/
//! blocked state and DEPENDENCIES section, and `offered_modes`/`mode` carry
//! T95's integration-mode cycling (`m`, `iui_action_cycle_integration`).
//! There is still no `d`/`r` (dependency-install-hint text, reverify) --
//! those read installer/tools.tsv's own hint table, which nothing in this
//! installer parses yet.

use crate::requirements::{SkillState, SkillStatus};

pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub installed: bool,
    pub status: SkillStatus,
    /// Every integration mode this skill declares (`[]` for the near-total
    /// majority with no `integration.tsv`, meaning it offers no choice at
    /// all -- same as install.sh's `IUI_INTEGRATION_OFFERED` being empty).
    pub offered_modes: Vec<String>,
    /// The mode this skill installs in if selected right now: the run's
    /// already-resolved default until `m` cycles it, from then on whatever
    /// was last cycled to.
    pub mode: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Focus {
    List,
    Info,
}

pub struct PickerState {
    pub skills: Vec<SkillEntry>,
    pub selected: Vec<bool>,
    pub cursor: usize,
    pub scroll: usize,
    pub focus: Focus,
    pub info_scroll: usize,
    pub message: Vec<String>,
    pub done: bool,
    pub confirmed: bool,
}

impl PickerState {
    /// Everything installable starts selected, same as install.sh's numbered
    /// menu default of "all" -- but a Blocked skill is preselected through
    /// `toggle`, same as iui_load_installer_skills does, so it cannot end up
    /// selected: iui_toggle refuses it the same way a later keypress would.
    pub fn new(skills: Vec<SkillEntry>) -> Self {
        let selected = vec![false; skills.len()];
        let mut state = PickerState {
            skills,
            selected,
            cursor: 0,
            scroll: 0,
            focus: Focus::List,
            info_scroll: 0,
            message: Vec::new(),
            done: false,
            confirmed: false,
        };
        for i in 0..state.skills.len() {
            state.toggle(i);
        }
        state.message.clear();
        state
    }

    /// Deselecting is always allowed; selecting a Blocked skill is refused
    /// with the reason instead of allowed and then rejected by the install
    /// itself -- same rule as installer/src/35-ui-model.sh's iui_toggle.
    pub fn toggle(&mut self, index: usize) {
        let Some(skill) = self.skills.get(index) else {
            return;
        };
        let currently_selected = self.selected.get(index).copied().unwrap_or(false);
        if !currently_selected && skill.status.state == SkillState::Blocked {
            let reason = skill
                .status
                .blocker
                .clone()
                .unwrap_or_else(|| "a required tool".to_string());
            self.message = vec![format!(
                "{} is blocked: {reason} is missing",
                skill.name
            )];
            return;
        }
        if let Some(slot) = self.selected.get_mut(index) {
            *slot = !*slot;
        }
    }

    /// Same rule as the `a` key in install.sh's iui_handle_key: deselect
    /// everything first, then toggle each one back on, so a Blocked skill
    /// stays out through the same refusal `toggle` gives a direct keypress.
    pub fn select_all(&mut self) {
        for i in 0..self.skills.len() {
            self.selected[i] = false;
            self.toggle(i);
        }
    }

    pub fn select_none(&mut self) {
        self.selected.iter_mut().for_each(|s| *s = false);
    }

    /// Movement follows focus: the info pane scrolls its own text, the list
    /// pane moves the cursor and always resets the info scroll, since a
    /// newly focused skill's detail has its own length.
    pub fn move_by(&mut self, delta: isize) {
        if self.focus == Focus::Info {
            let next = self.info_scroll as isize + delta;
            self.info_scroll = next.max(0) as usize;
            return;
        }
        let count = self.skills.len() as isize;
        if count == 0 {
            return;
        }
        let next = (self.cursor as isize + delta).clamp(0, count - 1);
        self.cursor = next as usize;
        self.info_scroll = 0;
    }

    pub fn go_home(&mut self) {
        match self.focus {
            Focus::Info => self.info_scroll = 0,
            Focus::List => {
                self.cursor = 0;
                self.info_scroll = 0;
            }
        }
    }

    pub fn go_end(&mut self, info_max_scroll: usize) {
        match self.focus {
            Focus::Info => self.info_scroll = info_max_scroll,
            Focus::List => {
                self.cursor = self.skills.len().saturating_sub(1);
                self.info_scroll = 0;
            }
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::List => Focus::Info,
            Focus::Info => Focus::List,
        };
        self.info_scroll = 0;
    }

    /// Scroll follows the cursor, same clamp rule as iui_clamp_scroll: never
    /// let the cursor run off either edge of the visible window, and never
    /// scroll past the point where the list would show trailing blank rows.
    pub fn clamp_scroll(&mut self, visible_rows: usize) {
        let count = self.skills.len();
        if self.scroll > self.cursor {
            self.scroll = self.cursor;
        }
        if visible_rows > 0 && self.cursor >= self.scroll + visible_rows {
            self.scroll = self.cursor + 1 - visible_rows;
        }
        if count <= visible_rows {
            self.scroll = 0;
        } else if self.scroll > count - visible_rows {
            self.scroll = count - visible_rows;
        }
    }

    /// Every selected skill's name paired with the mode it will install in
    /// -- the answer `i` hands the caller, the mode being whatever `m` last
    /// cycled it to (or the run's already-resolved default otherwise).
    pub fn selected_with_modes(&self) -> Vec<(String, String)> {
        self.skills
            .iter()
            .zip(self.selected.iter())
            .filter(|(_, sel)| **sel)
            .map(|(skill, _)| (skill.name.clone(), skill.mode.clone()))
            .collect()
    }

    /// Advances the skill under the cursor to its next offered mode,
    /// wrapping -- ported from installer/src/37-ui-input.sh's
    /// `iui_action_cycle_integration`. A no-op, with no message, for a
    /// skill offering fewer than two modes -- there is nothing to refuse,
    /// so unlike `toggle` this never has anything to say.
    pub fn cycle_integration_mode(&mut self) {
        let Some(skill) = self.skills.get_mut(self.cursor) else {
            return;
        };
        if skill.offered_modes.len() < 2 {
            return;
        }
        let current_index = skill.offered_modes.iter().position(|m| m == &skill.mode);
        let next_index = match current_index {
            Some(i) => (i + 1) % skill.offered_modes.len(),
            None => 0,
        };
        skill.mode = skill.offered_modes[next_index].clone();
        self.message = vec![format!("{} will be installed in {} mode", skill.name, skill.mode)];
    }

    pub fn selected_count(&self) -> usize {
        self.selected.iter().filter(|s| **s).count()
    }

    pub fn installed_count(&self) -> usize {
        self.skills.iter().filter(|s| s.installed).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_status() -> SkillStatus {
        SkillStatus {
            state: SkillState::Ok,
            blocker: None,
            requirements: Vec::new(),
        }
    }

    fn blocked_status(blocker: &str) -> SkillStatus {
        SkillStatus {
            state: SkillState::Blocked,
            blocker: Some(blocker.to_string()),
            requirements: Vec::new(),
        }
    }

    fn skills(names: &[&str]) -> Vec<SkillEntry> {
        names
            .iter()
            .map(|n| SkillEntry {
                name: n.to_string(),
                description: format!("{n} description"),
                installed: false,
                status: ok_status(),
                offered_modes: Vec::new(),
                mode: "skill".to_string(),
            })
            .collect()
    }

    #[test]
    fn everything_starts_selected() {
        let state = PickerState::new(skills(&["a", "b", "c"]));
        assert_eq!(state.selected_count(), 3);
    }

    #[test]
    fn toggle_flips_one_entry_only() {
        let mut state = PickerState::new(skills(&["a", "b"]));
        state.toggle(0);
        assert_eq!(state.selected, vec![false, true]);
    }

    #[test]
    fn select_all_and_none() {
        let mut state = PickerState::new(skills(&["a", "b"]));
        state.select_none();
        assert_eq!(state.selected_count(), 0);
        state.select_all();
        assert_eq!(state.selected_count(), 2);
    }

    #[test]
    fn cursor_movement_clamps_to_the_list_bounds() {
        let mut state = PickerState::new(skills(&["a", "b", "c"]));
        state.move_by(-5);
        assert_eq!(state.cursor, 0);
        state.move_by(5);
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn moving_the_cursor_resets_info_scroll() {
        let mut state = PickerState::new(skills(&["a", "b"]));
        state.info_scroll = 4;
        state.move_by(1);
        assert_eq!(state.info_scroll, 0);
    }

    #[test]
    fn focused_on_info_movement_scrolls_the_detail_pane_instead() {
        let mut state = PickerState::new(skills(&["a", "b"]));
        state.focus = Focus::Info;
        state.move_by(3);
        assert_eq!(state.info_scroll, 3);
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn info_scroll_never_goes_negative() {
        let mut state = PickerState::new(skills(&["a"]));
        state.focus = Focus::Info;
        state.move_by(-10);
        assert_eq!(state.info_scroll, 0);
    }

    #[test]
    fn scroll_follows_the_cursor_down_and_up() {
        let mut state = PickerState::new(skills(&["a", "b", "c", "d", "e"]));
        state.cursor = 4;
        state.clamp_scroll(2);
        assert_eq!(state.scroll, 3);
        state.cursor = 0;
        state.clamp_scroll(2);
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn scroll_never_shows_trailing_blank_rows() {
        let mut state = PickerState::new(skills(&["a", "b", "c"]));
        state.scroll = 2;
        state.cursor = 0;
        state.clamp_scroll(5);
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn selected_names_preserves_skill_order() {
        let mut state = PickerState::new(skills(&["a", "b", "c"]));
        state.toggle(1);
        let names: Vec<String> = state
            .selected_with_modes()
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(names, vec!["a", "c"]);
    }

    #[test]
    fn toggle_focus_alternates_and_resets_info_scroll() {
        let mut state = PickerState::new(skills(&["a"]));
        state.info_scroll = 5;
        state.toggle_focus();
        assert_eq!(state.focus, Focus::Info);
        assert_eq!(state.info_scroll, 0);
        state.toggle_focus();
        assert_eq!(state.focus, Focus::List);
    }

    #[test]
    fn a_blocked_skill_starts_unselected_and_cannot_be_toggled_on() {
        let mut list = skills(&["a", "b"]);
        list[1].status = blocked_status("rjq");
        let mut state = PickerState::new(list);
        assert_eq!(state.selected, vec![true, false]);
        state.toggle(1);
        assert_eq!(state.selected, vec![true, false]);
        assert!(state.message[0].contains("b"));
        assert!(state.message[0].contains("rjq"));
    }

    #[test]
    fn deselecting_a_blocked_skill_is_still_allowed() {
        let mut list = skills(&["a"]);
        list[0].status = blocked_status("rjq");
        let state = PickerState::new(list);
        assert_eq!(state.selected, vec![false]);
    }

    #[test]
    fn select_all_skips_a_blocked_skill() {
        let mut list = skills(&["a", "b"]);
        list[1].status = blocked_status("rjq");
        let mut state = PickerState::new(list);
        state.select_none();
        state.select_all();
        assert_eq!(state.selected, vec![true, false]);
    }

    #[test]
    fn cycling_a_skill_with_no_offered_modes_is_a_no_op() {
        let mut state = PickerState::new(skills(&["a"]));
        state.cycle_integration_mode();
        assert_eq!(state.skills[0].mode, "skill");
        assert!(state.message.is_empty());
    }

    #[test]
    fn cycling_advances_to_the_next_offered_mode_and_wraps() {
        let mut list = skills(&["a"]);
        list[0].offered_modes = vec!["skill".to_string(), "mcp".to_string()];
        let mut state = PickerState::new(list);
        state.cycle_integration_mode();
        assert_eq!(state.skills[0].mode, "mcp");
        assert!(state.message[0].contains("a will be installed in mcp mode"));
        state.cycle_integration_mode();
        assert_eq!(state.skills[0].mode, "skill");
    }

    #[test]
    fn cycling_operates_on_the_skill_under_the_cursor() {
        let mut list = skills(&["a", "b"]);
        list[1].offered_modes = vec!["skill".to_string(), "mcp".to_string()];
        let mut state = PickerState::new(list);
        state.cursor = 1;
        state.cycle_integration_mode();
        assert_eq!(state.skills[0].mode, "skill");
        assert_eq!(state.skills[1].mode, "mcp");
    }

    #[test]
    fn selected_with_modes_reports_each_selected_skills_current_mode() {
        let mut list = skills(&["a", "b"]);
        list[1].offered_modes = vec!["skill".to_string(), "mcp".to_string()];
        let mut state = PickerState::new(list);
        state.cursor = 1;
        state.cycle_integration_mode();
        assert_eq!(
            state.selected_with_modes(),
            vec![("a".to_string(), "skill".to_string()), ("b".to_string(), "mcp".to_string())]
        );
    }
}
