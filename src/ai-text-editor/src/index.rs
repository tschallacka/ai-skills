// MODE: DEV
// PACKAGE: PROD
use serde::{Deserialize, Serialize};

pub const DEFAULT_GRANULARITY: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexBlock {
    pub line: usize,
    pub byte_offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LineIndex {
    pub granularity: usize,
    pub bytes: usize,
    pub lines: usize,
    pub blocks: Vec<IndexBlock>,
}

impl LineIndex {
    pub fn build(bytes: &[u8], granularity: usize) -> Self {
        let granularity = granularity.max(1);
        let mut blocks = vec![IndexBlock {
            line: 1,
            byte_offset: 0,
        }];
        let mut line = 1;
        for (offset, byte) in bytes.iter().enumerate() {
            if *byte == b'\n' {
                line += 1;
                if line % granularity == 1 {
                    blocks.push(IndexBlock {
                        line,
                        byte_offset: offset + 1,
                    });
                }
            }
        }
        Self {
            granularity,
            bytes: bytes.len(),
            lines: line,
            blocks,
        }
    }

    pub fn block_for_line(&self, line: usize) -> &IndexBlock {
        self.blocks
            .iter()
            .rev()
            .find(|block| block.line <= line)
            .unwrap_or(&self.blocks[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_index_records_line_byte_offsets() {
        let mut bytes = Vec::new();
        for _ in 0..20_001 {
            bytes.extend_from_slice(b"x\n");
        }
        let index = LineIndex::build(&bytes, DEFAULT_GRANULARITY);
        assert_eq!(index.lines, 20_002);
        assert_eq!(index.block_for_line(10_001).byte_offset, 20_000);
    }
    #[test]
    fn explicit_granularity_and_zero_are_safe() {
        let index = LineIndex::build(b"a\nb\nc\n", 2);
        assert_eq!(
            index.blocks,
            vec![
                IndexBlock {
                    line: 1,
                    byte_offset: 0
                },
                IndexBlock {
                    line: 3,
                    byte_offset: 4
                }
            ]
        );
        assert_eq!(LineIndex::build(b"", 0).granularity, 1);
    }
}
