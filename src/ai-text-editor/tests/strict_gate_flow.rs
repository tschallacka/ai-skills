// MODE: DEV
//! What the strict editor gate denies and lets through, driven through the same
//! JSON a harness hands the hook. Every denied search names one file, and the
//! allowed list holds the near misses a careless pattern would catch.

use ai_text_editor::strict_gate::{answer, decide};
use serde_json::{json, Value};

const FILE: &str = "notes.md";

const SEARCHES: &[&str] = &[
    "grep foo notes.md",
    "grep -n foo notes.md",
    "grep -rn foo notes.md",
    "grep -A3 foo notes.md",
    "grep -A 3 foo notes.md",
    "grep -e foo notes.md",
    "grep --regexp=foo notes.md",
    "grep -q foo notes.md && echo y",
    "grep foo notes.md 2>/dev/null",
    "rg foo notes.md",
    "/usr/bin/grep foo notes.md",
    "GREP_OK=1 grep foo notes.md",
    "sudo grep foo notes.md",
    "timeout 5 grep foo notes.md",
    "cd src && grep foo ../notes.md",
    "grep foo < notes.md",
    "cat notes.md | grep foo",
    "head -100 notes.md | grep foo",
    "tail -n 5 notes.md | rg foo",
    "x=\"$(grep foo notes.md)\"",
    "echo a\ngrep foo notes.md",
    "bash -c 'grep foo notes.md'",
    "eval grep foo notes.md",
    "grep foo \\\n notes.md",
    "sed -n '/foo/p' notes.md",
    "sed '/foo/!d' notes.md",
    "sed -n 's/foo/bar/p' notes.md",
    "cat notes.md | sed -n '/foo/p'",
    "awk '/foo/' notes.md",
    "awk -F: '$1 ~ /x/ {print}' notes.md",
    "perl -ne 'print if /foo/' notes.md",
    "python3 -c \"import re; print(re.findall('x', open('notes.md').read()))\"",
    "python3 - <<'EOF'\nfor line in open('notes.md'):\n    if 'x' in line:\n        print(line)\nEOF",
    "node -e \"console.log(require('fs').readFileSync('notes.md','utf8').includes('x'))\"",
];

const SHELL_WRITES: &[&str] = &[
    "echo hi > out.txt",
    "printf 'x' >> /opt/thing",
    "ls | tee out.txt",
    "cat > out.txt <<EOF\nhi\nEOF",
    "> out.txt",
    "echo hi 1> out.txt",
];

const NEAR_MISSES: &[&str] = &[
    "git log | grep foo",
    "ls -la | rg foo",
    "tail -f app.log | grep ERROR",
    "tail -F app.log | grep ERROR",
    "grep -r foo src",
    "grep foo a.txt b.txt",
    "grep foo src/*.rs",
    "rg foo",
    "rg foo src",
    "rg --files src",
    "echo 'grep foo notes.md'",
    "echo \"cat x | grep y\"",
    "sed -n '10,20p' notes.md",
    "sed 's/a/b/' notes.md",
    "git diff | sed -n '/foo/p'",
    "awk '{print $1}' notes.md",
    "ls | awk '/x/'",
    "echo hi",
    "echo 2 3",
    "echo hi > /dev/null",
    "echo hi >&2",
    "echo hi 2> err.log",
    "echo hi >> \"$TMPDIR/x.log\"",
    "make > build.log",
    "cargo test > ~/.config/tsch-ai-skills/tmp/t.log 2>&1",
    "python3 -c 'print(1)'",
    "python3 -c \"import json; print(json.load(open('a.json'))['x'].find('y'))\"",
    "python3 - <<'EOF'\nimport os\nfor r,d,f in os.walk('.'):\n    print(r)\nEOF",
    "cat <<EOF | grep foo\nfoo\nEOF",
    "grep foo <<< \"$x\"",
    "x=$(( 1 << 2 ))\necho ok",
    "cat notes.md",
    "echo 'unterminated",
];

fn workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(FILE), "needle\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    dir
}

fn payload(dir: &tempfile::TempDir, tool: &str, input: Value) -> Value {
    json!({"tool_name": tool, "tool_input": input, "cwd": dir.path().to_str().unwrap()})
}

fn bash_verdict(dir: &tempfile::TempDir, command: &str) -> Option<String> {
    decide(&payload(dir, "Bash", json!({ "command": command })))
}

/// Runs the real server's subcommand on a `Write` call, optionally with the off switch.
fn client_answer(off_switch: bool) -> String {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut command = Command::new(env!("CARGO_BIN_EXE_ai-text-editor-server"));
    command
        .arg("strict-gate")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    if off_switch {
        command.env("EDITOR_GATE_OFF", "1");
    } else {
        command.env_remove("EDITOR_GATE_OFF");
    }
    let mut child = command.spawn().unwrap();
    let input = json!({"tool_name": "Write", "tool_input": {"file_path": "/x"}}).to_string();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(input.as_bytes()).unwrap();
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn native_edit_tools_are_denied_and_name_the_editor() {
    let dir = workspace();
    for tool in ["Edit", "Write", "MultiEdit"] {
        let reason = decide(&payload(&dir, tool, json!({"file_path": "/x/y"})))
            .unwrap_or_else(|| panic!("{tool} was allowed"));
        assert!(reason.contains("mcp__ai-text-editor__open"), "{reason}");
    }
}

#[test]
fn plan_mode_stays_writable() {
    let dir = workspace();
    let mut planning = payload(&dir, "Write", json!({"file_path": "/x/y"}));
    planning["permission_mode"] = json!("plan");
    assert_eq!(decide(&planning), None);
}

#[test]
fn the_plan_file_stays_writable() {
    let dir = workspace();
    let plan = "~/.claude/plans/a.md";
    assert_eq!(
        decide(&payload(&dir, "Write", json!({ "file_path": plan }))),
        None
    );
}

#[test]
fn native_grep_is_denied_on_one_file_only() {
    let dir = workspace();
    let grep = |path: Value| {
        decide(&payload(
            &dir,
            "Grep",
            json!({"pattern": "x", "path": path}),
        ))
    };
    assert!(grep(json!(FILE)).is_some());
    assert_eq!(grep(json!("src")), None);
    assert_eq!(
        decide(&payload(&dir, "Grep", json!({"pattern": "x"}))),
        None
    );
}

#[test]
fn single_file_shell_searches_are_denied() {
    let dir = workspace();
    for command in SEARCHES {
        let reason = bash_verdict(&dir, command)
            .unwrap_or_else(|| panic!("allowed, should deny: {command}"));
        assert!(reason.contains("mcp__ai-text-editor__search"), "{reason}");
    }
}

#[test]
fn a_saved_script_that_only_searches_one_file_is_denied() {
    let dir = workspace();
    std::fs::write(
        dir.path().join("find.py"),
        "import re\nprint(re.findall('x', open('notes.md').read()))\n",
    )
    .unwrap();
    assert!(bash_verdict(&dir, "python3 find.py").is_some());
    std::fs::write(dir.path().join("big.py"), "x\n".repeat(5000)).unwrap();
    assert_eq!(bash_verdict(&dir, "python3 big.py"), None);
}

#[test]
fn shell_writes_to_a_real_path_are_denied() {
    // Not the temp workspace: a relative target under it is a scratch path.
    let cwd = "/opt/project";
    for command in SHELL_WRITES {
        let input = json!({"tool_name": "Bash", "cwd": cwd, "tool_input": {"command": command}});
        let reason = decide(&input).unwrap_or_else(|| panic!("allowed, should deny: {command}"));
        assert!(reason.contains("Writing a file"), "{reason}");
    }
}

#[test]
fn near_misses_are_allowed() {
    let dir = workspace();
    for command in NEAR_MISSES {
        assert_eq!(bash_verdict(&dir, command), None, "denied: {command}");
    }
}

#[cfg(unix)]
#[test]
fn shell_writes_to_a_temp_path_are_allowed() {
    let dir = workspace();
    for command in ["echo hi > /tmp/x.log", "ls | tee /tmp/x.log"] {
        assert_eq!(bash_verdict(&dir, command), None, "denied: {command}");
    }
}

#[cfg(unix)]
#[test]
fn a_heredoc_body_naming_a_search_is_data_not_a_command() {
    let dir = workspace();
    let command = "cat <<EOF >/tmp/x\ngrep foo notes.md\nEOF";
    assert_eq!(bash_verdict(&dir, command), None);
}

#[test]
fn the_hook_answer_is_json_and_fails_closed() {
    let dir = workspace();
    let allowed = answer(&payload(&dir, "Read", json!({"file_path": FILE})).to_string());
    assert_eq!(allowed, "{}");

    let denied = answer(&payload(&dir, "Write", json!({"file_path": "/x"})).to_string());
    let parsed: Value = serde_json::from_str(&denied).unwrap();
    assert_eq!(parsed["hookSpecificOutput"]["permissionDecision"], "deny");

    for garbage in ["not json", ""] {
        let parsed: Value = serde_json::from_str(&answer(garbage)).unwrap();
        assert_eq!(parsed["hookSpecificOutput"]["permissionDecision"], "deny");
    }
}

#[test]
fn the_client_subcommand_reads_stdin_and_prints_the_answer() {
    assert!(client_answer(false).contains("\"deny\""));
}

#[test]
fn the_session_off_switch_opens_the_gate() {
    assert_eq!(client_answer(true), "{}");
}
