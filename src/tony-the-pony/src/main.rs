// MODE: DEV
// PACKAGE: PROD
//! Tony The Pony He Comes — a tool-call gate for agent CLIs.
//!
//! Named for the Stack Overflow answer about parsing HTML with regex, because
//! the joke cuts both ways: the first version of this gate *was* a pile of
//! regex pointed at shell commands, and it blocked writing a file that merely
//! mentioned searching. This one lexes.
//!
//! Reads a hook payload as JSON on stdin and, when a text searcher is being
//! invoked over source code, denies the call with the question the agent should
//! answer first. Claude Code and Codex share this protocol: `tool_input.command`
//! in, `hookSpecificOutput.permissionDecision` out.
//!
//! No dependencies, no runtime: neither Claude Code, Codex nor opencode
//! guarantees bash, node or python3 is present, and on Windows a Claude Code
//! hook runs under PowerShell rather than bash. A static binary is the only
//! implementation that needs nothing.
//!
//! Modes:
//!   (no args)        read a hook payload on stdin, decide, print JSON if denying
//!   --check '<cmd>'  print the verdict for one command and exit 0/1
//!   --self-test      run the built-in cases; exit non-zero on any failure

mod json;
mod lexer;

use lexer::Verdict;
use std::io::Read;

const MESSAGE: &str = concat!(
    "Tony The Pony He Comes. Text search is gated — is it the right tool here?\n",
    "\n",
    "Answer before retrying:\n",
    "  1. A question about CODE STRUCTURE — who calls this, what depends on it,\n",
    "     where is it defined, what is the blast radius? Use the repository's own\n",
    "     index (CodeGraph or equivalent). It follows dynamic dispatch; text\n",
    "     search cannot.\n",
    "  2. A question a framework's config merge decides — dependency injection,\n",
    "     plugin order, overrides, routes, schema? Use that framework's own query\n",
    "     tool. It models the merge rules; text search sees neither.\n",
    "  3. A LITERAL STRING in a non-code file, a log, or command output — or a\n",
    "     repository-aware lookup already failed to answer it? Then text search is\n",
    "     correct, and you may mint a token for THIS command.\n",
    "\n",
    "An empty index result means NOT INDEXED, never \"does not exist\"."
);

fn deny(reason: &str) {
    // Built by hand rather than with a serialiser: one output shape, and a
    // dependency-free binary is the entire point of this implementation.
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\"permissionDecisionReason\":{}}}}}",
        json::encode_string(reason)
    );
}

fn self_test() -> i32 {
    // (command, should the gate deny it)
    let cases: [(&str, bool); 16] = [
        ("grep -rn foo src/", true),
        ("/usr/bin/grep -rn foo src/", true),
        ("LC_ALL=C grep -rn foo src/", true),
        ("sudo grep -rn foo /etc/", true),
        ("ls; grep -rn foo src/", true),
        ("(cd src && grep -rn foo .)", true),
        ("rg --files-with-matches foo", true),
        ("xargs grep -l foo", true),
        ("ls -la | grep foo", false),
        ("gh run view 1 | grep -E 'FAIL'", false),
        ("echo 'grep is gated'", false),
        ("printf \"%s\" \"use grep -rn\"", false),
        ("python3 -c \"print('grep -rn foo')\"", false),
        ("cat ~/.claude/hooks/grep-gate", false),
        ("cat > f.md <<'EOF'\nUse grep -rn foo\nEOF", false),
        ("git commit -m \"stop using grep for structure\"", false),
    ];

    let mut failures = 0;
    for (command, expect_deny) in cases {
        let verdict = lexer::inspect(command);
        let denied = matches!(verdict, Verdict::Gated(_));
        let ok = denied == expect_deny;
        if !ok {
            failures += 1;
        }
        println!(
            "  {}  deny={:<5} want={:<5}  {}",
            if ok { "ok  " } else { "FAIL" },
            denied,
            expect_deny,
            command.replace('\n', "\\n")
        );
    }

    // Unlexable text must be undecidable, so a caller can fail closed.
    let undecidable = lexer::inspect("echo 'unterminated") == Verdict::Undecidable;
    if !undecidable {
        failures += 1;
    }
    println!(
        "  {}  unterminated quote is Undecidable (caller fails closed)",
        if undecidable { "ok  " } else { "FAIL" }
    );

    if failures == 0 {
        println!("\nall {} checks passed", cases.len() + 1);
        0
    } else {
        println!("\n{failures} CHECK(S) FAILED");
        1
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("--self-test") => std::process::exit(self_test()),
        Some("--version") => {
            println!("tony-the-pony {}", env!("CARGO_PKG_VERSION"));
        }
        Some("--check") => {
            let command = args.get(1).map(String::as_str).unwrap_or("");
            match lexer::inspect(command) {
                Verdict::Gated(name) => {
                    println!("gated: {name} is invoked as a command");
                    std::process::exit(1);
                }
                Verdict::Clear => println!("clear"),
                Verdict::Undecidable => {
                    println!("undecidable: could not lex; a caller must fail closed");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            let mut payload = String::new();
            if std::io::stdin().read_to_string(&mut payload).is_err() {
                return; // unreadable stdin is not grounds to block a tool call
            }
            let command = match json::string_at(&payload, &["tool_input", "command"]) {
                Some(c) => c,
                None => return,
            };
            match lexer::inspect(&command) {
                Verdict::Gated(_) => deny(MESSAGE),
                Verdict::Clear => {}
                // Fail closed: an unparseable command is not evidence of innocence.
                Verdict::Undecidable => deny(MESSAGE),
            }
        }
    }
}
