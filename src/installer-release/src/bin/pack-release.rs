// MODE: DEV
//! pack-release SRC_DIR OUT.tar.gz — maintainer tool, never shipped. Packs a
//! directory into the same gzipped-tar format `installer_release::fetch_and_extract`
//! reads back, for building a real release or a local test fixture.

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let [src, out] = match argv.as_slice() {
        [src, out] => [src.clone(), out.clone()],
        _ => {
            eprintln!("usage: pack-release <src-dir> <out.tar.gz>");
            return ExitCode::from(64);
        }
    };
    match installer_release::pack_dir_as_tar_gz(&PathBuf::from(&src), &PathBuf::from(&out)) {
        Ok(()) => {
            println!("wrote {out}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("pack-release: {e}");
            ExitCode::FAILURE
        }
    }
}
