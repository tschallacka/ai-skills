// MODE: DEV
// PACKAGE: PROD
use plan_crypt::random::fill;
use plan_crypt::sha256::{hex, Sha256};
use planning_core::git_snapshot;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const COMMAND: &str = "mint-fix-keys.sh";
type Pair = (String, String);

fn usage(code: i32) -> ! {
    println!("Usage: {COMMAND} [--plan-dir] <plan-directory>\n       {COMMAND} --help");
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    std::process::exit(code)
}

fn random_hex(bytes: usize) -> Result<String, ()> {
    let mut value = vec![0; bytes];
    fill(&mut value).map_err(|_| ())?;
    Ok(hex(&value))
}

fn digest(secret: &str, message: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(message.as_bytes());
    hex(&hasher.finish())
}

fn scratch_dir() -> PathBuf {
    env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("planning-agent")
}

fn session_path(session: &str) -> PathBuf {
    scratch_dir().join("review-fix-keys").join(session)
}

fn existing_session(plan: &Path) -> Option<String> {
    let text = fs::read_to_string(plan.join("fix-keys.json")).ok()?;
    let marker = "\"session_id\"";
    let start = text.find(marker)?;
    let tail = &text[start + marker.len()..];
    let start = tail.find('"')? + 1;
    let end = tail[start..].find('"')? + start;
    Some(tail[start..end].to_string())
}

fn ensure_session(plan: &Path) -> String {
    if let Some(session) = existing_session(plan) {
        if session_path(&session).join("secret").is_file() {
            return session;
        }
    }
    let session = random_hex(8).unwrap_or_else(|_| die("cannot generate a session id: no OS random source (need the plan-crypt binary or a readable /dev/urandom)", 69));
    let directory = session_path(&session);
    fs::create_dir_all(&directory).unwrap_or_else(|error| die(error.to_string(), 69));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&directory).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&directory, permissions).unwrap();
    }
    let secret = random_hex(32).unwrap_or_else(|_| die("cannot generate a session secret: no OS random source (need the plan-crypt binary or a readable /dev/urandom)", 69));
    let secret_file = directory.join("secret");
    fs::write(&secret_file, format!("{secret}\n"))
        .unwrap_or_else(|error| die(error.to_string(), 69));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&secret_file).unwrap().permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(&secret_file, permissions).unwrap();
    }
    session
}

fn valid_id(value: &str, prefix: &str) -> bool {
    value
        .strip_prefix(prefix)
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()))
}

fn gated_pairs(review: &str) -> (Vec<Pair>, Vec<Pair>) {
    let mut in_findings = false;
    let mut valid = Vec::new();
    let mut invalid = Vec::new();
    for line in review.lines() {
        if line == "## Findings" {
            in_findings = true;
            continue;
        }
        if in_findings && line.starts_with("## Verdict") {
            break;
        }
        if !in_findings || !line.starts_with('|') {
            continue;
        }
        let fields: Vec<_> = line.split('|').map(str::trim).collect();
        let fid = fields.get(1).copied().unwrap_or_default();
        let wu = fields.get(5).copied().unwrap_or_default();
        if fid == "ID" || fid == "---" || wu == "---" || wu.is_empty() || wu == "N/A" || wu == "—"
        {
            continue;
        }
        if valid_id(fid, "AR-") && valid_id(wu, "W") {
            valid.push((fid.to_string(), wu.to_string()));
        } else {
            invalid.push((fid.to_string(), wu.to_string()));
        }
    }
    (valid, invalid)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        usage(0);
    }
    if args.len() != 1 || args[0].starts_with('-') {
        usage(64);
    }
    let plan = PathBuf::from(&args[0]);
    if !plan.is_dir() {
        die(format!("Plan directory not found: {}", plan.display()), 66);
    }
    let review_file = plan.join("adversarial-review.md");
    if !review_file.is_file() {
        die(
            format!("adversarial-review.md not found: {}", review_file.display()),
            64,
        );
    }
    git_snapshot(&plan);
    let session = ensure_session(&plan);
    let secret = fs::read_to_string(session_path(&session).join("secret"))
        .unwrap_or_else(|error| die(error.to_string(), 69));
    let secret = secret.trim_end_matches(['\n', '\r']);
    let review =
        fs::read_to_string(&review_file).unwrap_or_else(|error| die(error.to_string(), 64));
    let (pairs, invalid) = gated_pairs(&review);
    for (fid, wu) in &invalid {
        eprintln!("{COMMAND}: WARN skipping gated row with non-conforming id: finding id \"{fid}\" work unit \"{wu}\" (expect ^AR-[0-9]+$ and ^W[0-9]+$)");
    }
    if !invalid.is_empty() {
        die(format!("mint-fix-keys: {} gated row(s) could not be minted; fix the finding/work-unit ids so the fix-key gate is not silently disabled", invalid.len()), 64);
    }
    let minted_by = env::var("MINTED_BY").unwrap_or_else(|_| session.clone());
    let mut json = format!(
        "{{\n  \"session_id\": \"{session}\",\n  \"minted_by\": \"{minted_by}\",\n  \"keys\": {{\n"
    );
    let mut current = String::new();
    let mut first_in_finding = true;
    for (fid, wu) in &pairs {
        if *fid != current {
            if !current.is_empty() {
                json.push_str("    },\n");
            }
            json.push_str(&format!("    \"{fid}\": {{\n"));
            current = fid.clone();
            first_in_finding = true;
        }
        let key = digest(secret, &format!("{session}|{fid}|{wu}"));
        if first_in_finding {
            json.push_str(&format!("      \"{wu}\": \"{key}\"\n"));
            first_in_finding = false;
        } else {
            json.push_str(&format!("    , \"{wu}\": \"{key}\"\n"));
        }
    }
    if !current.is_empty() {
        json.push_str("    }\n");
    }
    json.push_str("  }\n}\n");
    fs::write(plan.join("fix-keys.json"), json).unwrap_or_else(|error| die(error.to_string(), 70));
    println!(
        "Minted fix keys for {} (session {})",
        plan.display(),
        session
    );
}
