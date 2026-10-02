// MODE: DEV
// PACKAGE: PROD

use std::io::Write;

use jaq_core::load::{Arena, File, Loader};
use jaq_core::{data, unwrap_valr, Compiler, Ctx, Vars};
use jaq_json::{read, write, Val};

/// `IN/1` and `IN/2`, ported from jq's own `builtin.jq` rather than trusting a
/// remembered one-liner (T85). jaq's stdlib carries `in/1` (`. as $x | xs |
/// has($x)`, the has-based membership test on the arg) but not jq's capital
/// `IN`, which instead asks "does `.` equal any value the generator(s)
/// produce" -- so it needs no `has`, never errors on a non-container, and
/// returns false rather than erroring when the generator is empty.
///
/// One line, no trailing newline: prepended to the caller's filter source
/// verbatim, so a single-line filter (the overwhelming majority of callers in
/// this repo) still starts on line 1 and any syntax error in it still reports
/// the line number it would have reported without this prelude.
const IN_PRELUDE: &str = "def IN(s): any(s == .; .); def IN(src; s): any(src == s; .); ";

pub fn run(filter_source: &str, input: &str) -> Result<String, String> {
    run_with_options(filter_source, input, false, false, Vec::new())
}

pub fn run_with_options(
    filter_source: &str,
    input: &str,
    raw_output: bool,
    compact_output: bool,
    variables: Vec<(String, Val)>,
) -> Result<String, String> {
    let filter_source = format!("{IN_PRELUDE}{filter_source}");
    let mut inputs = read::parse_many(input.as_bytes());
    let defs = jaq_core::defs()
        .chain(jaq_std::defs())
        .chain(jaq_json::defs());
    let funs = jaq_core::funs()
        .chain(jaq_std::funs())
        .chain(jaq_json::funs());
    let arena = Arena::default();
    let modules = Loader::new(defs)
        .load(
            &arena,
            File {
                code: &filter_source,
                path: (),
            },
        )
        .map_err(|error| format!("{error:?}"))?;
    let variable_names: Vec<&'static str> = variables
        .iter()
        .map(|(name, _)| Box::leak(format!("${name}").into_boxed_str()) as &'static str)
        .collect();
    let variable_values: Vec<Val> = variables.into_iter().map(|(_, value)| value).collect();
    let filter = Compiler::default()
        .with_global_vars(variable_names)
        .with_funs(funs)
        .compile(modules)
        .map_err(|error| format!("{error:?}"))?;
    let context = Ctx::<data::JustLut<Val>>::new(&filter.lut, Vars::new(variable_values));
    let mut output = Vec::new();
    let printer = write::Pp {
        indent: (!compact_output).then(|| "  ".to_owned()),
        sep_space: !compact_output,
        ..Default::default()
    };
    for input in inputs.by_ref() {
        let input = input.map_err(|error| error.to_string())?;
        for value in filter.id.run((context.clone(), input)).map(unwrap_valr) {
            let value = value.map_err(|error| format!("{error:?}"))?;
            if raw_output {
                if let Val::TStr(text) = &value {
                    output.extend_from_slice(text);
                } else {
                    write::write(&mut output, &printer, 0, &value)
                        .map_err(|error| error.to_string())?;
                }
            } else {
                write::write(&mut output, &printer, 0, &value)
                    .map_err(|error| error.to_string())?;
            }
            output.write_all(b"\n").map_err(|error| error.to_string())?;
        }
    }
    String::from_utf8(output).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn in1_matches_a_value_the_generator_produces() {
        assert_eq!(run(".[] | IN(1,2,3)", "[1,4]\n").unwrap(), "true\nfalse\n");
    }

    #[test]
    fn in1_against_an_empty_generator_is_false_not_an_error() {
        // The trap T85 exists to close: a hand-rolled `has(...)` check on a
        // non-container errors instead of reporting false, and the error
        // lands on stderr while stdout -- what a `$(...)` capture reads --
        // stays empty, so a caller checking only `-z "$out"` sees a sound
        // register that never ran the check at all.
        assert_eq!(run("IN(empty)", "\"z\"\n").unwrap(), "false\n");
    }

    #[test]
    fn in2_cross_compares_both_generators() {
        assert_eq!(run("IN((1,2); (2,3))", "null\n").unwrap(), "true\n");
        assert_eq!(run("IN((1,2); (3,4))", "null\n").unwrap(), "false\n");
    }

    #[test]
    fn a_users_own_syntax_error_still_names_their_own_text() {
        // The prelude adds working definitions, not brokenness: a parse error
        // in the CALLER's filter must still be attributed to the caller's own
        // offending token, proving the prelude parses clean ahead of it
        // rather than shifting blame onto text the caller never wrote.
        let error = run(".foo |+", "null\n").unwrap_err();
        assert!(
            error.contains("\"|+\""),
            "expected the error to blame the caller's own token, got: {error}"
        );
    }
}
