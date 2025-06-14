// SPDX-FileCopyrightText: Copyright © 2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Partition APIs

use std::fmt::Display;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use fs_err as fs;
use gpt::disk::LogicalBlockSize;
use gpt::partition_types;
use superblock::Superblock;

/// Partition on a GPT disk
#[derive(Debug, Clone, Default)]
pub struct Partition {
    pub path: PathBuf,
    pub kind: Kind,
    pub size: u64,
    pub uuid: String,
    pub sb: Option<super::SuperblockKind>,
}

impl Display for Partition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.path.display().to_string())
    }
}
/// Specialised type of partition
#[derive(Debug, Clone, Default)]
#[allow(clippy::upper_case_acronyms)]
pub enum Kind {
    ESP,
    XBOOTLDR,
    #[default]
    Regular,
}

/// Superblock scanning, self contained
fn scan_superblock(path: &PathBuf) -> Result<superblock::Kind, super::Error> {
    let fi = fs::File::open(path)?;
    let mut buffer: Vec<u8> = Vec::with_capacity(2 * 1024 * 1024);
    fi.take(2 * 1024 * 1024).read_to_end(&mut buffer)?;
    let mut cursor = Cursor::new(&buffer);
    let sb = Superblock::from_reader(&mut cursor)?;
    Ok(sb.kind())
}

impl Partition {
    /// Construct new Partition from the given GPT Partition and block size
    pub fn from(value: &gpt::partition::Partition, block_size: &LogicalBlockSize) -> Result<Self, super::Error> {
        let uuid = value.part_guid.hyphenated().to_string();
        let path = fs::canonicalize(format!("/dev/disk/by-partuuid/{uuid}"))?;
        let kind = match value.part_type_guid {
            partition_types::EFI => Kind::ESP,
            partition_types::FREEDESK_BOOT => Kind::XBOOTLDR,
            _ => Kind::Regular,
        };
        let sb = scan_superblock(&path).ok();
        let size = value.bytes_len(*block_size)?;
        Ok(Self {
            path,
            kind,
            size,
            uuid,
            sb,
        })
    }
}
