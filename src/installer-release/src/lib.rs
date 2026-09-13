// MODE: DEV
// PACKAGE: PROD
//! Fetch a release archive over HTTP(S) and extract it to a directory. This
//! is the piece the tiny curl-piped bootstrap script hands off to: it knows
//! nothing about skills or manifests, only "get this URL, unpack it here."

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("fetching {url}: {source}")]
    Request {
        url: String,
        #[source]
        source: Box<ureq::Error>,
    },
    #[error("reading response body from {url}: {source}")]
    Body {
        url: String,
        #[source]
        source: io::Error,
    },
    #[error("extracting archive: {0}")]
    Extract(#[from] io::Error),
}

/// Download `url` (a `.tar.gz`) and unpack it under `dest`, creating `dest` if
/// it does not exist. Returns the list of top-level entries `dest` now holds,
/// mirroring the way a release tarball's contents are consumed by the
/// installer binary it also carries.
pub fn fetch_and_extract(url: &str, dest: &Path) -> Result<Vec<PathBuf>, FetchError> {
    let bytes = fetch_bytes(url)?;
    extract_tar_gz(&bytes, dest)?;
    Ok(top_level_entries(dest)?)
}

fn fetch_bytes(url: &str) -> Result<Vec<u8>, FetchError> {
    let response = ureq::get(url).call().map_err(|e| FetchError::Request {
        url: url.to_string(),
        source: Box::new(e),
    })?;
    let mut buf = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut buf)
        .map_err(|e| FetchError::Body {
            url: url.to_string(),
            source: e,
        })?;
    Ok(buf)
}

/// Extracted separately from the fetch so a locally-packed archive (this
/// session's `--pack` output, or a test fixture) can go through the same
/// unpack path a downloaded one does, with no HTTP round trip either needs.
pub fn extract_tar_gz(bytes: &[u8], dest: &Path) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    let gz = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(gz);
    archive.unpack(dest)
}

/// Pack every file under `src_dir` into a gzipped tar at `out_file`, paths
/// relative to `src_dir`. The maintainer side of `extract_tar_gz`: used by
/// the real release build and by a test harness packing a throwaway fixture,
/// so both go through the one format this crate reads back.
pub fn pack_dir_as_tar_gz(src_dir: &Path, out_file: &Path) -> io::Result<()> {
    let file = fs::File::create(out_file)?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut builder = tar::Builder::new(encoder);
    builder.append_dir_all(".", src_dir)?;
    builder.into_inner()?.finish()?;
    Ok(())
}

fn top_level_entries(dest: &Path) -> io::Result<Vec<PathBuf>> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dest)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn pack(files: &[(&str, &str)]) -> Vec<u8> {
        let mut tar_bytes = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut tar_bytes);
            for (name, content) in files {
                let mut header = tar::Header::new_gnu();
                header.set_size(content.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                builder
                    .append_data(&mut header, name, content.as_bytes())
                    .unwrap();
            }
            builder.finish().unwrap();
        }
        let mut gz_bytes = Vec::new();
        {
            let mut encoder =
                flate2::write::GzEncoder::new(&mut gz_bytes, flate2::Compression::default());
            encoder.write_all(&tar_bytes).unwrap();
            encoder.finish().unwrap();
        }
        gz_bytes
    }

    #[test]
    fn extract_writes_files_and_reports_top_level_entries() {
        let work = tempfile::tempdir().unwrap();
        let dest = work.path().join("unpacked");
        let archive = pack(&[
            ("installer", "#!/bin/sh\necho hi\n"),
            ("skills/todo/SKILL.md", "# todo\n"),
        ]);

        extract_tar_gz(&archive, &dest).unwrap();

        assert!(dest.join("installer").is_file());
        assert!(dest.join("skills/todo/SKILL.md").is_file());
        assert_eq!(
            fs::read_to_string(dest.join("skills/todo/SKILL.md")).unwrap(),
            "# todo\n"
        );

        let top = top_level_entries(&dest).unwrap();
        let names: Vec<_> = top
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(names, vec!["installer", "skills"]);
    }

    #[test]
    fn pack_then_extract_round_trips() {
        let src = tempfile::tempdir().unwrap();
        fs::create_dir_all(src.path().join("skills/todo")).unwrap();
        fs::write(src.path().join("skills/todo/SKILL.md"), "# todo\n").unwrap();
        fs::write(src.path().join("installer"), "binary-stand-in").unwrap();

        let archive_path = src.path().join("../release.tar.gz");
        pack_dir_as_tar_gz(src.path(), &archive_path).unwrap();
        let bytes = fs::read(&archive_path).unwrap();

        let dest = tempfile::tempdir().unwrap();
        extract_tar_gz(&bytes, dest.path()).unwrap();

        assert_eq!(
            fs::read_to_string(dest.path().join("skills/todo/SKILL.md")).unwrap(),
            "# todo\n"
        );
        assert_eq!(
            fs::read_to_string(dest.path().join("installer")).unwrap(),
            "binary-stand-in"
        );
    }

    #[test]
    fn extract_creates_dest_when_missing() {
        let work = tempfile::tempdir().unwrap();
        let dest = work.path().join("does/not/exist/yet");
        let archive = pack(&[("marker", "x")]);

        extract_tar_gz(&archive, &dest).unwrap();

        assert!(dest.join("marker").is_file());
    }
}
