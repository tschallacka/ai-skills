// MODE: DEV
// PACKAGE: PROD
//! Picker state -- selection, cursor, scroll, focus -- ported in spirit from
//! installer/src/35-ui-model.sh's IUI_* state, trimmed to what this slice
//! actually drives: no dependency/requirement table (runtime_requirements
//! is not ported), no integration-mode cycling (T95, needs integration.tsv),
//! no per-tool cache. Every skill is treated as installable; the ACTIONS
//! pane (d/r/m) and the blocked/degraded state tag do not exist yet.

pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub installed: bool,
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
    /// Everything starts selected, same as install.sh's numbered menu
    /// default of "all" and the picker's own iui_load_installer_skills.
    pub fn new(skills: Vec<SkillEntry>) -> Self {
        let selected = vec![true; skills.len()];
        PickerState {
            skills,
            selected,
            cursor: 0,
            scroll: 0,
            focus: Focus::List,
            info_scroll: 0,
            message: Vec::new(),
            done: false,
            confirmed: false,
        }
    }

    pub fn toggle(&mut self, index: usize) {
        if let Some(slot) = self.selected.get_mut(index) {
            *slot = !*slot;
        }
    }

    pub fn select_all(&mut self) {
        self.selected.iter_mut().for_each(|s| *s = true);
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

    fn skills(names: &[&str]) -> Vec<SkillEntry> {
        names
            .iter()
            .map(|n| SkillEntry {
                name: n.to_string(),
                description: format!("{n} description"),
                installed: false,
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
}
