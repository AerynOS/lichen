// SPDX-FileCopyrightText: Copyright © 2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Disk management

use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use fs_err as fs;
use gpt::GptConfig;

use super::{Error, Partition};

/// Indicates type of disk device
#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum Kind {
    /// Hard disk drive
    HDD,

    /// Solid State device
    SSD,
}

impl Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Kind::HDD => f.write_str("HDD"),
            Kind::SSD => f.write_str("SSD"),
        }
    }
}

/// Basic physical device mapping
#[derive(Debug)]
pub struct Disk {
    pub path: PathBuf,
    pub kind: Kind,
    pub model: Option<String>,
    pub vendor: Option<String>,
    pub block_size: u64,
    pub size: u64,
}

impl Display for Disk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let vendor = self.vendor.as_ref().map(|v| format!("{v} ")).unwrap_or_default();

        let description = if let Some(model) = self.model.as_ref() {
            format!("{vendor}{model}")
        } else {
            vendor.to_string()
        };

        f.write_fmt(format_args!("{} ({})", description, &self.kind))
    }
}

impl Disk {
    /// Build a Disk from the given sysfs path
    pub fn from_sysfs_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let device_link = path.join("device");
        let slavedir = path.join("slaves");

        let file_name = path.file_name().ok_or(Error::InvalidDisk)?;

        // Ensure the device link is present (no virtual ram0 device, etc)
        if !device_link.exists() {
            return Err(Error::InvalidDisk);
        }

        // Root level devices, not interested in child partitions as yet.
        let ancestors = fs::read_dir(slavedir)?.filter_map(|m| m.ok()).collect::<Vec<_>>();
        if !ancestors.is_empty() {
            return Err(Error::InvalidDisk);
        }

        // SSD or HDD?
        let rotational = path.join("queue").join("rotational");
        let kind = if rotational.exists() {
            match str::parse::<i32>(fs::read_to_string(rotational)?.trim())? {
                0 => Kind::SSD,
                _ => Kind::HDD,
            }
        } else {
            Kind::HDD
        };

        // additioal metadata.

        let vendor = fs::read_to_string(device_link.join("vendor"))
            .ok()
            .map(|f| f.trim().to_string());
        let model = fs::read_to_string(device_link.join("model"))
            .ok()
            .map(|f| f.trim().to_string());
        let block_size = str::parse::<u64>(fs::read_to_string(path.join("queue").join("physical_block_size"))?.trim())?;
        let size = str::parse::<u64>(fs::read_to_string(path.join("size"))?.trim())?;

        let path = PathBuf::from("/dev").join(file_name);

        Ok(Self {
            path,
            kind,
            vendor,
            model,
            block_size,
            size,
        })
    }

    /// Discover all disks on the system
    pub fn discover() -> Result<Vec<Self>, Error> {
        let disks = fs::read_dir("/sys/class/block")?
            .filter_map(|f| {
                let f = f.ok()?;
                Disk::from_sysfs_path(f.path()).ok()
            })
            .collect::<Vec<_>>();
        Ok(disks)
    }

    /// Return all partitions on the disk if it is GPT
    pub fn partitions(&self) -> Result<Vec<Partition>, Error> {
        let path = self.path.clone();
        let path: &PathBuf = &path;
        let device = Box::new(fs::File::open(path)?);
        let table = GptConfig::default().writable(false).open_from_device(device)?;
        let block_size = table.logical_block_size();
        let mut parts = vec![];
        for (_, part) in table.partitions().iter() {
            parts.push(Partition::from(part, block_size)?)
        }
        Ok(parts)
    }
}
