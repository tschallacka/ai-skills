// MODE: DEV
// PACKAGE: PROD

use std::io::Write;
use std::process::{Command, Output, Stdio};

struct Case {
    name: &'static str,
    args: &'static [&'static str],
    input: &'static str,
}

const CORPUS: &[Case] = &[
    Case {
        name: "array iteration",
        args: &[".[]"],
        input: "[1,2,3]\n",
    },
    Case {
        name: "object projection",
        args: &[".items[] | .name"],
        input: "{\"items\":[{\"name\":\"a\"}]}\n",
    },
    Case {
        name: "raw output",
        args: &["-r", ".name"],
        input: "{\"name\":\"rjq\"}\n",
    },
    Case {
        name: "compact output",
        args: &["-c", "."],
        input: "{\"a\": 1, \"b\": [true]}\n",
    },
    Case {
        name: "string variable",
        args: &["--arg", "name", "rjq", ".name"],
        input: "{\"name\":\"input\"}\n",
    },
    Case {
        name: "json variable",
        args: &["--argjson", "n", "3", ". + $n"],
        input: "4\n",
    },
];

#[test]
fn differential_corpus_is_executed() {
    let Some(reference) = std::env::var_os("RJQ_REFERENCE_JQ") else {
        eprintln!("UNCONFIGURED: RJQ_REFERENCE_JQ must point to pinned jq 1.7");
        return;
    };
    let rjq = std::env::var_os("CARGO_BIN_EXE_rjq").expect("cargo must provide the rjq binary");

    for case in CORPUS {
        let expected = run(&reference, case);
        let actual = run(&rjq, case);
        assert_eq!(actual.status, expected.status, "{} exit status", case.name);
        assert_eq!(actual.stdout, expected.stdout, "{} stdout", case.name);
        assert_eq!(actual.stderr, expected.stderr, "{} stderr", case.name);
    }
}

fn run(program: &std::ffi::OsStr, case: &Case) -> Output {
    let mut child = Command::new(program)
        .args(case.args)
        .env_remove("RUST_BACKTRACE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("differential process must start");
    child
        .stdin
        .take()
        .expect("differential stdin must be available")
        .write_all(case.input.as_bytes())
        .expect("differential input must be written");
    child
        .wait_with_output()
        .expect("differential process must exit")
}
