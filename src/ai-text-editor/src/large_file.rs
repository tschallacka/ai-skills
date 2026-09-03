// MODE: DEV
// PACKAGE: PROD
use crate::index::{IndexBlock, LineIndex};
use crate::search::{matches_with_gradient, SearchMode};
use std::fs::File;
use std::io::Write;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_READ_BYTES: usize = 8 * 1024 * 1024;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct LargeFile {
    pub path: PathBuf,
    pub bytes: u64,
}

impl LargeFile {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let bytes = std::fs::metadata(&path)?.len();
        Ok(Self { path, bytes })
    }

    pub fn read_range(&self, offset: u64, requested: usize) -> io::Result<Range> {
        if offset > self.bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "offset exceeds file size",
            ));
        }
        let length = requested
            .min(DEFAULT_READ_BYTES)
            .min((self.bytes - offset) as usize);
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(offset))?;
        let mut bytes = vec![0; length];
        let read = file.read(&mut bytes)?;
        bytes.truncate(read);
        Ok(Range {
            offset,
            bytes,
            eof: offset + read as u64 >= self.bytes,
        })
    }

    pub fn read_lines(&self, start_line: u64, line_count: usize) -> io::Result<Lines> {
        let start_line = start_line.max(1);
        let mut reader = BufReader::new(File::open(&self.path)?);
        let mut line = Vec::new();
        let mut current = 1;
        while current < start_line {
            line.clear();
            if reader.read_until(b'\n', &mut line)? == 0 {
                return Ok(Lines {
                    start_line,
                    end_line: current.saturating_sub(1),
                    text: String::new(),
                    eof: true,
                });
            }
            current += 1;
        }
        let mut text = Vec::new();
        for _ in 0..line_count {
            line.clear();
            if reader.read_until(b'\n', &mut line)? == 0 {
                return Ok(Lines {
                    start_line,
                    end_line: current.saturating_sub(1),
                    text: String::from_utf8(text).map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, "large text range is not UTF-8")
                    })?,
                    eof: true,
                });
            }
            text.extend_from_slice(&line);
            current += 1;
        }
        Ok(Lines {
            start_line,
            end_line: current - 1,
            text: String::from_utf8(text).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "large text range is not UTF-8")
            })?,
            eof: false,
        })
    }

    pub fn index(&self, granularity: usize) -> io::Result<LineIndex> {
        let granularity = granularity.max(1);
        let mut reader = BufReader::new(File::open(&self.path)?);
        let mut blocks = vec![IndexBlock {
            line: 1,
            byte_offset: 0,
        }];
        let mut line = Vec::new();
        let mut line_number = 1u64;
        let mut offset = 0u64;
        loop {
            line.clear();
            let read = reader.read_until(b'\n', &mut line)?;
            if read == 0 {
                break;
            }
            offset += read as u64;
            line_number += 1;
            if (line_number - 1).is_multiple_of(granularity as u64) {
                blocks.push(IndexBlock {
                    line: line_number as usize,
                    byte_offset: offset as usize,
                });
            }
        }
        Ok(LineIndex {
            granularity,
            bytes: self.bytes as usize,
            lines: line_number as usize,
            blocks,
        })
    }

    pub fn index_prefix(&self, granularity: usize, max_lines: usize) -> io::Result<LineIndex> {
        let granularity = granularity.max(1);
        let max_lines = max_lines.max(1);
        let mut reader = BufReader::new(File::open(&self.path)?);
        let mut blocks = vec![IndexBlock {
            line: 1,
            byte_offset: 0,
        }];
        let mut line_number = 1usize;
        let mut offset = 0usize;
        let mut line = Vec::new();
        while line_number <= max_lines {
            line.clear();
            let read = reader.read_until(b'\n', &mut line)?;
            if read == 0 {
                break;
            }
            offset += read;
            line_number += 1;
            if (line_number - 1).is_multiple_of(granularity) {
                blocks.push(IndexBlock {
                    line: line_number,
                    byte_offset: offset,
                });
            }
        }
        Ok(LineIndex {
            granularity,
            bytes: self.bytes as usize,
            lines: line_number,
            blocks,
        })
    }

    pub fn search_text(
        &self,
        query: &str,
        start_line: u64,
        end_line: Option<u64>,
    ) -> io::Result<Vec<(usize, usize, usize, String)>> {
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let mut reader = BufReader::new(File::open(&self.path)?);
        let mut line = Vec::new();
        let mut line_number = 1usize;
        let mut results = Vec::new();
        loop {
            line.clear();
            if reader.read_until(b'\n', &mut line)? == 0 {
                break;
            }
            if (line_number as u64) >= start_line
                && end_line
                    .map(|end| line_number as u64 <= end)
                    .unwrap_or(true)
            {
                let content = line.strip_suffix(b"\n").unwrap_or(&line);
                let content = content.strip_suffix(b"\r").unwrap_or(content);
                let text = std::str::from_utf8(content).map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "large text search range is not UTF-8",
                    )
                })?;
                for (start, found) in text.match_indices(query) {
                    results.push((
                        line_number,
                        text[..start].chars().count(),
                        text[..start + found.len()].chars().count(),
                        found.to_owned(),
                    ));
                }
            }
            if end_line
                .map(|end| line_number as u64 >= end)
                .unwrap_or(false)
            {
                break;
            }
            line_number += 1;
        }
        Ok(results)
    }

    pub fn search_text_mode(
        &self,
        mode: SearchMode,
        query: &str,
        start_line: u64,
        end_line: u64,
        gradient: Option<f64>,
    ) -> io::Result<Vec<(usize, usize, usize, String)>> {
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let mut reader = BufReader::new(File::open(&self.path)?);
        let mut line = Vec::new();
        let mut line_number = 1usize;
        let mut results = Vec::new();
        loop {
            line.clear();
            if reader.read_until(b'\n', &mut line)? == 0 {
                break;
            }
            if (line_number as u64) >= start_line && (line_number as u64) <= end_line {
                let content = line.strip_suffix(b"\n").unwrap_or(&line);
                let content = content.strip_suffix(b"\r").unwrap_or(content);
                let text = std::str::from_utf8(content).map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "large text search range is not UTF-8",
                    )
                })?;
                let matches =
                    matches_with_gradient(mode, query, text, gradient).map_err(|error| {
                        io::Error::new(io::ErrorKind::InvalidInput, error.to_string())
                    })?;
                results.extend(matches.into_iter().map(|(start, end)| {
                    (
                        line_number,
                        text[..start].chars().count(),
                        text[..end].chars().count(),
                        text[start..end].to_owned(),
                    )
                }));
            }
            if line_number as u64 >= end_line {
                break;
            }
            line_number += 1;
        }
        Ok(results)
    }

    pub fn search_bytes(
        &self,
        query: &[u8],
        offset: u64,
        length: usize,
    ) -> io::Result<Vec<(u64, u64, Vec<u8>)>> {
        if query.is_empty() {
            return Ok(Vec::new());
        }
        if length > DEFAULT_READ_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "large byte search range exceeds the bounded read limit",
            ));
        }
        let range = self.read_range(offset, length)?;
        Ok(range
            .bytes
            .windows(query.len())
            .enumerate()
            .filter(|(_, bytes)| *bytes == query)
            .map(|(start, bytes)| {
                (
                    offset + start as u64,
                    offset + start as u64 + query.len() as u64,
                    bytes.to_vec(),
                )
            })
            .collect())
    }

    /// Rewrite one byte range without materialising the source file in memory.
    /// The replacement is intentionally bounded by the caller; the source and
    /// temporary file remain in the same directory so the final rename is local.
    pub fn rewrite(&self, offset: u64, delete_len: u64, replacement: &[u8]) -> io::Result<Self> {
        let end = offset
            .checked_add(delete_len)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "edit range overflows"))?;
        if end > self.bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "edit range exceeds file size",
            ));
        }
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        let stamp = format!(
            "{}-{}-{}",
            std::process::id(),
            self.bytes,
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let temporary = parent.join(format!(".ai-text-editor-large-{stamp}.tmp"));
        let permissions = std::fs::metadata(&self.path)?.permissions();
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let result = (|| {
            let mut input = File::open(&self.path)?;
            copy_bytes(&mut input, &mut output, offset)?;
            output.write_all(replacement)?;
            input.seek(SeekFrom::Start(end))?;
            io::copy(&mut input, &mut output)?;
            output.set_permissions(permissions)?;
            output.sync_all()?;
            std::fs::rename(&temporary, &self.path)?;
            Self::open(&self.path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }

    pub fn restore_from(&self, source: &Path) -> io::Result<Self> {
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        let stamp = format!(
            "restore-{}-{}-{}",
            std::process::id(),
            self.bytes,
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let temporary = parent.join(format!(".ai-text-editor-{stamp}.tmp"));
        let permissions = std::fs::metadata(&self.path)?.permissions();
        let result = (|| {
            let mut input = File::open(source)?;
            let mut output = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            io::copy(&mut input, &mut output)?;
            output.set_permissions(permissions)?;
            output.sync_all()?;
            std::fs::rename(&temporary, &self.path)?;
            Self::open(&self.path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }
}

fn copy_bytes(input: &mut File, output: &mut File, mut count: u64) -> io::Result<()> {
    let mut buffer = [0u8; 1024 * 1024];
    while count > 0 {
        let wanted = count.min(buffer.len() as u64) as usize;
        let read = input.read(&mut buffer[..wanted])?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "source ended during edit range",
            ));
        }
        output.write_all(&buffer[..read])?;
        count -= read as u64;
    }
    Ok(())
}

#[derive(Debug)]
pub struct Range {
    pub offset: u64,
    pub bytes: Vec<u8>,
    pub eof: bool,
}

#[derive(Debug)]
pub struct Lines {
    pub start_line: u64,
    pub end_line: u64,
    pub text: String,
    pub eof: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn bounded_ranges_and_line_indexes_do_not_load_the_file_as_one_buffer() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("large.txt");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"a\nb\nc\n").unwrap();
        let large = LargeFile::open(&path).unwrap();
        let range = large.read_range(2, 2).unwrap();
        assert_eq!(range.bytes, b"b\n");
        assert_eq!(large.read_lines(2, 1).unwrap().text, "b\n");
        assert_eq!(large.index(2).unwrap().blocks[1].line, 3);
        assert_eq!(large.search_text("b", 1, None).unwrap()[0].0, 2);
    }

    #[test]
    fn bounded_byte_search_returns_absolute_offsets() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bytes");
        std::fs::write(&path, b"zero\0needle\0tail").unwrap();
        let file = LargeFile::open(&path).unwrap();
        let found = file.search_bytes(b"needle", 4, 8).unwrap();
        assert_eq!(found, vec![(5, 11, b"needle".to_vec())]);
    }

    #[test]
    fn bounded_text_search_supports_explicit_non_literal_modes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("large.txt");
        std::fs::write(&path, b"alpha\nbeta\nALPACA\n").unwrap();
        let file = LargeFile::open(&path).unwrap();
        let found = file
            .search_text_mode(SearchMode::RegexRust, "a.*a", 1, 3, None)
            .unwrap();
        assert_eq!(found[0].0, 1);
        let fuzzy = file
            .search_text_mode(SearchMode::FuzzySubsequence, "bt", 1, 3, Some(0.5))
            .unwrap();
        assert_eq!(fuzzy[0].0, 2);
    }

    #[test]
    fn large_text_search_reports_scalar_columns() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("large.txt");
        std::fs::write(&path, "éneedle\n").unwrap();
        let file = LargeFile::open(&path).unwrap();
        let found = file.search_text("needle", 1, None).unwrap();
        assert_eq!(found[0].1, 1);
        assert_eq!(found[0].2, 7);
    }

    #[test]
    fn prefix_index_stops_without_scanning_the_whole_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("large.txt");
        std::fs::write(&path, b"one\ntwo\nthree\nfour\n").unwrap();
        let file = LargeFile::open(&path).unwrap();
        let index = file.index_prefix(2, 2).unwrap();
        assert_eq!(index.bytes, 19);
        assert_eq!(index.lines, 3);
        assert_eq!(index.blocks[1].line, 3);
    }

    #[test]
    fn rewrite_streams_a_range_and_reopens_the_new_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("large.txt");
        std::fs::write(&path, b"before-after").unwrap();
        let large = LargeFile::open(&path).unwrap();
        let updated = large.rewrite(6, 6, b"middle").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"beforemiddle");
        assert_eq!(updated.bytes, 12);
    }
}
