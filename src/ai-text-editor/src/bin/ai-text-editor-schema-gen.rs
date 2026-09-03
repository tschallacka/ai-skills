// MODE: DEV
// PACKAGE: PROD
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let output = args
        .windows(2)
        .find(|pair| pair[0] == "--out-dir")
        .map(|pair| PathBuf::from(&pair[1]))
        .unwrap_or_else(|| PathBuf::from("ai-text-editor/schemas"));
    fs::create_dir_all(&output).unwrap_or_else(|error| die(&error.to_string()));
    write(&output.join("protocol.v1.json"), protocol_schema());
    write(&output.join("capabilities.v1.json"), capability_schema());
    println!("generated {}", output.display());
}

fn write(path: &std::path::Path, value: serde_json::Value) {
    let bytes = serde_json::to_vec_pretty(&value).unwrap();
    fs::write(path, [bytes.as_slice(), b"\n"].concat())
        .unwrap_or_else(|error| die(&error.to_string()));
}

fn protocol_schema() -> serde_json::Value {
    json!({"$schema":"https://json-schema.org/draft/2020-12/schema","title":"ai-text-editor v1 envelope","type":"object","required":["version","request_id","method","payload"],"properties":{"version":{"const":1},"request_id":{"type":"string","minLength":1},"method":{"type":"string","minLength":1},"revision":{"type":["integer","null"],"minimum":0},"auth_token":{"type":["string","null"]},"session_token":{"type":["string","null"]},"payload":{"type":"object"}},"additionalProperties":false})
}

fn capability_schema() -> serde_json::Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "ai-text-editor capabilities v1",
        "type": "object",
        "required": ["protocol_version", "search_modes", "document_modes", "presentations", "edit_methods", "job_methods"],
        "properties": {
            "protocol_version": {"const": 1},
            "document_modes": {"type": "array", "items": {"enum": ["text_utf8", "raw_bytes", "hex_view"]}},
            "search_modes": {"type": "array", "items": {"enum": ["exact_text", "exact_bytes", "wildcard", "shell_wildcard", "path_wildcard", "regex_rust", "regex_pcre2", "fuzzy_edit", "fuzzy_subsequence", "fuzzy_token", "fuzzy_ngram", "fuzzy_phonetic", "fuzzy_soundex"]}},
            "presentations": {"type": "array", "items": {"enum": ["structured", "text", "paging", "stream"]}},
            "edit_methods": {"type": "array", "items": {"enum": ["insert", "replace", "large_edit", "begin_transaction", "end_transaction", "restore", "undo", "redo", "save", "save_as", "close", "resolve_external"]}},
            "job_methods": {"type": "array", "items": {"enum": ["job_start", "job_poll", "job_progress", "job_complete", "job_cancel", "job_transfer", "job_release"]}}
        },
        "additionalProperties": false
    })
}

fn die(message: &str) -> ! {
    eprintln!("ai-text-editor-schema-gen: {message}");
    std::process::exit(64);
}
