// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Partititon formatting

use system::disk::Partition;

use super::{Context, Step};

/// Format a partition
#[derive(Debug)]
pub struct FormatPartition<'a> {
    /// What partition are we formatting
    pub(crate) partition: &'a Partition,

    /// What filesystem would you like it to have
    pub(crate) filesystem: String,
}

impl<'a> Step for FormatPartition<'a> {
    fn name(&self) -> &'static str {
        "format-partition"
    }

    fn execute(&self, _context: &mut Context) {
        let fs = self.filesystem.to_lowercase();
        let command = match fs.as_str() {
            "ext4" => ("mkfs.ext4", [&self.partition.path.display().to_string()]),
            _ => unimplemented!(),
        };
        log::info!("Formatting {} as {}", self.partition.path.display(), self.filesystem);
        log::trace!("Running: {command:?}");
    }

    fn title(&self) -> String {
        "Format partition".into()
    }

    fn describe(&self) -> String {
        // TODO: More than ext4 xD
        format!("{} as ext4", self.partition.path.display())
    }
}

/// Mount a given partition
#[derive(Debug)]
pub struct MountPartition<'a> {
    /// Which partition?
    pub(crate) partition: &'a Partition,

    /// Where are we mounting it?
    pub(crate) mountpoint: String,
}

impl<'a> Step for MountPartition<'a> {
    fn name(&self) -> &'static str {
        "mount-partition"
    }

    fn execute(&self, _context: &mut Context) {
        log::info!("Mounting {} to {}", self.partition.path.display(), self.mountpoint);
    }

    fn title(&self) -> String {
        "Mount filesystem".into()
    }

    fn describe(&self) -> String {
        format!("{} as {}", self.partition.path.display(), &self.mountpoint)
    }
}
