// MODE: DEV
// PACKAGE: PROD

use std::io::Write;

use jaq_core::load::{Arena, File, Loader};
use jaq_core::{data, unwrap_valr, Compiler, Ctx, Vars};
use jaq_json::{read, write, Val};

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
                code: filter_source,
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
