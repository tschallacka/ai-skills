// MODE: DEV
// PACKAGE: PROD
//! UI validation formerly provided by `validate-plan-ui-lib.sh`.

use planning_validator_common::{require_heading, trim, Findings};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Story {
    pub id: String,
    pub actions: String,
    pub interaction: String,
    pub status: String,
    pub evidence: String,
    pub related: String,
    pub cache_path: String,
}

pub fn parse_stories(path: &Path) -> Vec<Story> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            if !line.starts_with('|') {
                return None;
            }
            let fields = line.split('|').map(table_trim).collect::<Vec<_>>();
            if fields.len() < 10 || !story_id(&fields[1]) {
                return None;
            }
            Some(Story {
                id: fields[1].clone(),
                actions: fields[3].clone(),
                interaction: fields[4].clone(),
                status: fields[6].clone(),
                evidence: fields[7].clone(),
                related: fields[8].clone(),
                cache_path: fields[9].clone(),
            })
        })
        .collect()
}

pub fn validate(
    plan: &Path,
    ui_affected: &str,
    complete_mode: bool,
    unit_types: &HashMap<String, String>,
    findings: &mut Findings,
) {
    if ui_affected == "yes" {
        let description = plan.join("plan-description.md");
        require_heading(&description, "## UI validation", findings);
        let description_text = fs::read_to_string(&description).unwrap_or_default();
        if !description_text
            .lines()
            .any(|line| line == "- Required: yes")
        {
            findings.fail("UI-affected plan must require UI validation");
        }
        let stories_path = plan.join("ui-user-stories.md");
        let bugs_path = plan.join("bugs.md");
        if !stories_path.is_file() {
            findings.fail("UI validation is required but ui-user-stories.md is missing");
        } else {
            let stories_text = fs::read_to_string(&stories_path).unwrap_or_default();
            if !stories_text
                .lines()
                .any(|line| line.starts_with("# UI user stories: ") && line.len() > 19)
            {
                findings.fail(format!(
                    "Missing user-story title in {}",
                    stories_path.display()
                ));
            }
            let stories = parse_stories(&stories_path);
            let mut bug_stories = Vec::new();
            for story in &stories {
                if !has_interaction(&format!("{} {}", story.actions, story.interaction)) {
                    findings.fail(format!(
                        "{} has no documented direct user interaction",
                        story.id
                    ));
                }
                if has_prohibited(&format!("{} {}", story.actions, story.interaction)) {
                    findings.fail(format!(
                        "{} is driven by prohibited console, state, or direct-API input",
                        story.id
                    ));
                }
                validate_story_cache(plan, story, complete_mode, findings);
                if !matches!(
                    story.status.as_str(),
                    "💤 untested" | "⏳ in progress" | "✅ passed" | "🐛 bug found" | "⏭️ excluded"
                ) {
                    findings.fail(format!(
                        "{} has an unsupported user-story status '{}'",
                        story.id, story.status
                    ));
                }
                if story.status == "🐛 bug found" {
                    bug_stories.push(story.id.clone());
                }
                let verification_count = work_unit_ids(&story.related)
                    .into_iter()
                    .filter(|id| match unit_types.get(id) {
                        None => {
                            findings.fail(format!("{} refers to unknown work unit {id}", story.id));
                            false
                        }
                        Some(kind) => kind == "verification",
                    })
                    .count();
                if verification_count == 0 {
                    findings.fail(format!(
                        "{} has no related verification work unit",
                        story.id
                    ));
                }
                if complete_mode {
                    match story.status.as_str() {
                        "✅ passed" => {}
                        "⏭️ excluded" => {
                            if !contains_case_insensitive(&story.evidence, "user-approved")
                                && !contains_case_insensitive(&story.evidence, "user approved")
                            {
                                findings.fail(format!(
                                    "{} is excluded without recorded user approval",
                                    story.id
                                ));
                            }
                        }
                        _ => findings.fail(format!(
                            "{} is not validated at plan completion ({})",
                            story.id, story.status
                        )),
                    }
                }
            }
            if stories.is_empty() {
                findings.fail("ui-user-stories.md has no story rows; use IDs such as US-01");
            }
            validate_bugs(&bugs_path, &bug_stories, complete_mode, findings);
        }
    } else if fs::read_to_string(plan.join("plan-description.md"))
        .unwrap_or_default()
        .lines()
        .any(|line| line == "- Required: yes")
    {
        findings.fail("Plan requires UI validation but declares UI affected: no");
    }
}

fn validate_story_cache(plan: &Path, story: &Story, complete_mode: bool, findings: &mut Findings) {
    let path = trim(&story.cache_path);
    if !cache_path(&path) {
        findings.fail(format!(
            "{} has invalid run-cache path '{}'",
            story.id, path
        ));
        return;
    }
    let cache = plan.join(&path);
    if !cache.is_file() {
        findings.fail(format!(
            "{} run cache is missing: {}",
            story.id,
            cache.display()
        ));
        return;
    }
    for heading in [
        format!("# Browser run cache: {}", story.id),
        "## Starting state".into(),
        "## Buffered interaction sequence".into(),
        "## Waits and readiness".into(),
        "## Run result".into(),
    ] {
        require_heading(&cache, &heading, findings);
    }
    let interaction = section(&cache, "## Buffered interaction sequence");
    if !has_interaction(&interaction) {
        findings.fail(format!(
            "{} run cache has no direct UI input in its interaction sequence",
            story.id
        ));
    }
    if has_prohibited(&interaction) {
        findings.fail(format!(
            "{} run cache drives the page with prohibited console, state, or direct-API input",
            story.id
        ));
    }
    if contains_placeholder(&fs::read_to_string(&cache).unwrap_or_default()) {
        findings.fail(format!(
            "{} run cache still contains a template placeholder",
            story.id
        ));
    }
    if complete_mode {
        match story.status.as_str() {
            "✅ passed" if !has_line(&cache, "- Status: `✅ passed`") => {
                findings.fail(format!("{} run cache is not recorded as passed", story.id))
            }
            "⏭️ excluded"
                if !fs::read_to_string(&cache)
                    .unwrap_or_default()
                    .lines()
                    .any(|line| line.starts_with("- Status: ") && line.contains("excluded")) =>
            {
                findings.fail(format!(
                    "{} excluded run cache is not recorded as excluded",
                    story.id
                ))
            }
            _ => {}
        }
    }
}

fn validate_bugs(
    path: &Path,
    bug_stories: &[String],
    complete_mode: bool,
    findings: &mut Findings,
) {
    if !path.is_file() {
        findings.fail("UI validation requires bugs.md");
        return;
    }
    let text = fs::read_to_string(path).unwrap_or_default();
    if !text
        .lines()
        .any(|line| line.starts_with("# UI bugs: ") && line.len() > 11)
    {
        findings.fail(format!("Missing UI bug title in {}", path.display()));
    }
    for story in bug_stories {
        let linked = text.lines().any(|line| {
            if !line.starts_with('|') {
                return false;
            }
            let fields = line.split('|').map(table_trim).collect::<Vec<_>>();
            fields.len() >= 5
                && fields[1].starts_with("BUG-")
                && fields[2] == *story
                && fields[3].contains("-investigate-")
                && fields[4].contains("-fix-")
        });
        if !linked {
            findings.fail(format!(
                "{} bug lacks linked investigation and fix goals",
                story
            ));
        }
    }
    if complete_mode
        && text.lines().any(|line| {
            line.starts_with('|')
                && (line.contains("💤 open")
                    || line.contains("⏳ open")
                    || line.contains("⏳ in progress"))
        })
    {
        findings.fail("UI bugs.md has unresolved bugs at plan completion");
    }
}

fn section(path: &Path, heading: &str) -> String {
    let Ok(text) = fs::read_to_string(path) else {
        return String::new();
    };
    let mut active = false;
    let mut result = String::new();
    for line in text.lines() {
        if line == heading {
            active = true;
            continue;
        }
        if active && line.starts_with("## ") {
            break;
        }
        if active {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

fn has_line(path: &Path, expected: &str) -> bool {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .any(|line| line == expected)
}

fn contains_placeholder(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut start = 0;
    while let Some(offset) = text[start..].find('<') {
        let open = start + offset;
        if let Some(end_offset) = text[open + 1..].find('>') {
            if end_offset > 0 {
                return true;
            }
        }
        start = open + 1;
        if start >= bytes.len() {
            break;
        }
    }
    false
}

fn has_interaction(text: &str) -> bool {
    [
        "click", "tap", "type", "keyboard", "press", "swipe", "pinch", "drag", "select",
    ]
    .iter()
    .any(|word| contains_case_insensitive(text, word))
}

fn has_prohibited(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "evaluate(",
        "devtools",
        "inject",
        "localstorage",
        "sessionstorage",
        "xmlhttprequest",
        "fetch(",
        "curl",
        "direct-api",
        "direct api",
        "console command",
        "console script",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn contains_case_insensitive(text: &str, needle: &str) -> bool {
    text.to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

fn table_trim(value: &str) -> String {
    value
        .trim_matches(|character: char| character.is_ascii_whitespace())
        .to_owned()
}

fn cache_path(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("ui-story-runs/US-") else {
        return false;
    };
    let Some(number) = rest.strip_suffix(".md") else {
        return false;
    };
    number.len() >= 2 && number.bytes().all(|byte| byte.is_ascii_digit())
}

fn story_id(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("US-") else {
        return false;
    };
    rest.len() >= 2 && rest.bytes().all(|byte| byte.is_ascii_digit())
}

fn work_unit_ids(value: &str) -> Vec<String> {
    let bytes = value.as_bytes();
    let mut ids = Vec::new();
    let mut index = 0;
    while index + 2 < bytes.len() {
        if bytes[index] == b'W'
            && bytes[index + 1].is_ascii_digit()
            && bytes[index + 2].is_ascii_digit()
        {
            let start = index;
            index += 3;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            ids.push(value[start..index].to_owned());
        } else {
            index += 1;
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::{parse_stories, validate};
    use planning_validator_common::Findings;
    use std::collections::HashMap;
    use std::fs;

    #[test]
    fn parses_the_shell_story_columns() {
        let root = std::env::temp_dir().join(format!("validator-ui-{}", std::process::id()));
        let file = root.join("stories.md");
        let _ = fs::create_dir_all(&root);
        fs::write(
            &file,
            "| ID | Persona | Browser actions | Interaction evidence | Expected | Status | Evidence | Related work units | Run cache |\n|---|---|---|---|---|---|---|---|---|\n| US-01 | person | Click page | one click | result | 💤 untested | — | W01,W02 | `ui-story-runs/US-01.md` |\n",
        )
        .unwrap();
        let stories = parse_stories(&file);
        assert_eq!(stories.len(), 1);
        assert_eq!(stories[0].actions, "Click page");
        assert_eq!(stories[0].cache_path, "`ui-story-runs/US-01.md`");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn state_is_checked_without_a_browser_shortcut() {
        let root = std::env::temp_dir().join(format!("validator-ui-gate-{}", std::process::id()));
        let plan = root.join("plan");
        let _ = fs::create_dir_all(&plan);
        fs::write(plan.join("plan-description.md"), "## UI validation\n").unwrap();
        fs::write(
            plan.join("ui-user-stories.md"),
            "# UI user stories: x\n| ID | Persona | Browser actions | Interaction evidence | Expected | Status | Evidence | Related work units | Run cache |\n|---|---|---|---|---|---|---|---|---|\n| US-01 | p | Open | click | result | 💤 untested | — | W01 | `bad` |\n",
        )
        .unwrap();
        let mut findings = Findings::default();
        validate(&plan, "yes", false, &HashMap::new(), &mut findings);
        assert!(findings.errors > 0);
        let _ = fs::remove_dir_all(root);
    }
}
