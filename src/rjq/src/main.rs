// MODE: DEV
// PACKAGE: PROD

fn main() {
    use std::rc::Rc;

    if std::env::args().any(|argument| argument == "--version") {
        println!("rjq 0.1.0 (JSON query-compatible engine)");
        return;
    }
    if std::env::args().any(|argument| argument == "--help" || argument == "-h") {
        println!("Usage: rjq [options] filter [file...]");
        println!("Options: -r/--raw-output -c/--compact-output -e/--exit-status --arg --argjson --slurpfile");
        return;
    }

    let mut args = std::env::args().skip(1).peekable();
    let mut raw_output = false;
    let mut compact_output = false;
    let mut exit_status = false;
    let mut null_input = false;
    let mut raw_input = false;
    let mut slurp = false;
    let mut variables = Vec::new();
    let mut filter = None;
    let mut input_file = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "-r" | "--raw-output" => raw_output = true,
            "-c" | "--compact-output" => compact_output = true,
            "-e" | "--exit-status" => exit_status = true,
            "-n" | "--null-input" => null_input = true,
            "-R" | "--raw-input" => raw_input = true,
            "-s" | "--slurp" => slurp = true,
            "-Rs" | "-sR" => {
                raw_input = true;
                slurp = true;
            }
            "-rn" | "-nr" => {
                raw_output = true;
                null_input = true;
            }
            "-Rn" | "-nR" => {
                raw_input = true;
                null_input = true;
            }
            "-rRs" | "-Rsr" | "-srR" => {
                raw_output = true;
                raw_input = true;
                slurp = true;
            }
            "--arg" => {
                let name = args.next().unwrap_or_default();
                let value = args.next().unwrap_or_default();
                variables.push((name, jaq_json::Val::utf8_str(value)));
            }
            "--argjson" => {
                let name = args.next().unwrap_or_default();
                let value = args.next().unwrap_or_default();
                match jaq_json::read::parse_single(value.as_bytes()) {
                    Ok(value) => variables.push((name, value)),
                    Err(error) => {
                        eprintln!("rjq: {error}");
                        std::process::exit(2);
                    }
                }
            }
            "--slurpfile" => {
                let name = args.next().unwrap_or_default();
                let path = args.next().unwrap_or_default();
                let contents = match std::fs::read(path) {
                    Ok(contents) => contents,
                    Err(error) => {
                        eprintln!("rjq: {error}");
                        std::process::exit(2);
                    }
                };
                let values =
                    match jaq_json::read::parse_many(&contents).collect::<Result<Vec<_>, _>>() {
                        Ok(values) => values,
                        Err(error) => {
                            eprintln!("rjq: {error}");
                            std::process::exit(2);
                        }
                    };
                variables.push((name, jaq_json::Val::Arr(Rc::new(values))));
            }
            "--rawfile" => {
                let name = args.next().unwrap_or_default();
                let path = args.next().unwrap_or_default();
                match std::fs::read_to_string(path) {
                    Ok(value) => variables.push((name, jaq_json::Val::utf8_str(value))),
                    Err(error) => {
                        eprintln!("rjq: {error}");
                        std::process::exit(2);
                    }
                }
            }
            "-f" | "--from-file" => {
                let path = args.next().unwrap_or_default();
                filter = match std::fs::read_to_string(path) {
                    Ok(value) => Some(value),
                    Err(error) => {
                        eprintln!("rjq: {error}");
                        std::process::exit(2);
                    }
                };
            }
            value if filter.is_none() => filter = Some(value.to_owned()),
            value => input_file = Some(value.to_owned()),
        }
    }
    let filter = match filter {
        Some(filter) => filter,
        None => {
            eprintln!("rjq: a filter is required");
            std::process::exit(2);
        }
    };
    let mut input = String::new();
    let read_result = if null_input {
        Ok(())
    } else {
        match input_file {
            Some(path) => std::fs::read_to_string(path).map(|value| input = value),
            None => std::io::Read::read_to_string(&mut std::io::stdin(), &mut input).map(|_| ()),
        }
    };
    if let Err(error) = read_result {
        eprintln!("rjq: {error}");
        std::process::exit(2);
    }
    if null_input {
        input = "null\n".to_owned();
    } else if raw_input {
        input = if slurp {
            json_string(&input)
        } else {
            input
                .lines()
                .map(json_string)
                .collect::<Vec<_>>()
                .join("\n")
        };
    } else if slurp {
        input = match slurp_json(&input) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("rjq: {error}");
                std::process::exit(2);
            }
        };
    }
    let output = match rjq::run_with_options(&filter, &input, raw_output, compact_output, variables)
    {
        Ok(output) => output,
        Err(error) => {
            eprintln!("rjq: {error}");
            std::process::exit(5);
        }
    };
    print!("{output}");
    if exit_status {
        let last = jaq_json::read::parse_many(output.as_bytes()).last();
        std::process::exit(match last {
            Some(Ok(jaq_json::Val::Null | jaq_json::Val::Bool(false))) => 1,
            Some(Ok(_)) => 0,
            _ => 4,
        });
    }
}

fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32))
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

fn slurp_json(input: &str) -> Result<String, String> {
    use std::io::Write;
    let mut output = Vec::new();
    output.write_all(b"[").map_err(|error| error.to_string())?;
    let printer = jaq_json::write::Pp {
        indent: None,
        sep_space: false,
        ..Default::default()
    };
    for (index, value) in jaq_json::read::parse_many(input.as_bytes()).enumerate() {
        if index > 0 {
            output.write_all(b",").map_err(|error| error.to_string())?;
        }
        let value = value.map_err(|error| error.to_string())?;
        jaq_json::write::write(&mut output, &printer, 0, &value)
            .map_err(|error| error.to_string())?;
    }
    output.write_all(b"]").map_err(|error| error.to_string())?;
    String::from_utf8(output).map_err(|error| error.to_string())
}
