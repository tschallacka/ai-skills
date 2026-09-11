// MODE: DEV
// PACKAGE: PROD
//! Picker state -- selection, cursor, scroll, focus -- ported in spirit from
//! installer/src/35-ui-model.sh's IUI_* state, trimmed to what this slice
//! actually drives: [[requirements]] supplies the per-skill ok/degraded/
//! blocked state and DEPENDENCIES section, but there is still no
//! integration-mode cycling (T95, needs integration.tsv) and no ACTIONS pane
//! (d/r/m -- install-hint text, reverify, mode cycling) since none of those
//! read from a model this installer has yet.

use crate::requirements::{SkillState, SkillStatus};

pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub installed: bool,
    pub status: SkillStatus,
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

    pub fn selected_names(&self) -> Vec<String> {
        self.skills
            .iter()
            .zip(self.selected.iter())
            .filter(|(_, sel)| **sel)
            .map(|(skill, _)| skill.name.clone())
            .collect()
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
        assert_eq!(state.selected_names(), vec!["a", "c"]);
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
}
