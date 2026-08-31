use std::env;
use std::path::PathBuf;
fn main() {
    let a: Vec<String> = env::args().skip(1).collect();
    let mut socket = None;
    let mut cols = 80;
    let mut rows = 24;
    let mut idle = 300;
    let mut cmd = Vec::new();
    let mut i = 0;
    while i < a.len() {
        match a[i].as_str() {
            "--socket" => {
                i += 1;
                socket = Some(PathBuf::from(&a[i]));
            }
            "--cols" => {
                i += 1;
                cols = a[i].parse().unwrap();
            }
            "--rows" => {
                i += 1;
                rows = a[i].parse().unwrap();
            }
            "--idle-timeout" => {
                i += 1;
                idle = a[i].parse().unwrap();
            }
            "--" => {
                cmd.extend_from_slice(&a[i + 1..]);
                break;
            }
            x => {
                eprintln!("unknown argument: {x}");
                std::process::exit(2)
            }
        }
        i += 1;
    }
    if let Err(e) =
        interactive_shell_core::run(socket.expect("--socket is required"), cols, rows, idle, cmd)
    {
        eprintln!("interactive-shell: {e}");
        std::process::exit(1)
    }
}
