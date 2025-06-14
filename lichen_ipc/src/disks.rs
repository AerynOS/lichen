// SPDX-FileCopyrightText: Copyright © 2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

use log::debug;

use crate::disks_ipc;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Disk Service struct for Lichen
pub struct Service {
    disks_cache: Arc<RwLock<Vec<disks_ipc::Disk>>>,
    parts_cache: Arc<RwLock<HashMap<String, Vec<disks_ipc::Partition>>>>,
}

impl Default for Service {
    fn default() -> Self {
        Self::new()
    }
}

impl Service {
    /// Creates a new instance of Service
    pub fn new() -> Self {
        Self {
            disks_cache: Arc::new(RwLock::new(Vec::new())),
            parts_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl disks_ipc::VarlinkInterface for Service {
    /// Retrieves the list of disks
    fn get_disks(&self, call: &mut dyn disks_ipc::Call_GetDisks) -> varlink::Result<()> {
        if let Ok(read_cell) = self.disks_cache.read() {
            if !read_cell.is_empty() {
                debug!("restoring from disk cache");
                call.reply(read_cell.clone())?;
                return Ok(());
            }
        }

        // Acquire a lock
        let mut cell = match self.disks_cache.write() {
            Ok(c) => c,
            Err(_) => return call.reply_disk_error("Cannot lock cache".to_string()),
        };

        match system::disk::Disk::discover() {
            Ok(disks) => {
                let ret = disks
                    .iter()
                    .map(|d| disks_ipc::Disk {
                        kind: match d.kind {
                            system::disk::DiskKind::HDD => disks_ipc::Disk_kind::hdd,
                            system::disk::DiskKind::SSD => disks_ipc::Disk_kind::ssd,
                        },
                        path: d.path.to_string_lossy().to_string(),
                        model: d.model.clone(),
                        vendor: d.vendor.clone(),
                        size: d.size as i64,
                        block_size: d.block_size as i64,
                    })
                    .collect::<Vec<_>>();
                cell.extend(ret.clone());
                call.reply(ret)?;
            }
            Err(e) => return call.reply_disk_error(e.to_string()),
        }

        Ok(())
    }

    /// Retrieves the list of partitions for a given disk
    fn get_partitions(&self, call: &mut dyn disks_ipc::Call_GetPartitions, disk: String) -> varlink::Result<()> {
        if let Ok(read_cell) = self.parts_cache.read() {
            if let Some(cache) = read_cell.get(&disk) {
                debug!("restoring from partition cache");
                call.reply(cache.clone())?;
                return Ok(());
            }
        }

        // Not cached, so acquire write lock
        let mut cell = match self.parts_cache.write() {
            Ok(c) => c,
            Err(e) => return call.reply_disk_error(e.to_string()),
        };

        match system::disk::Disk::from_sysfs_path(disk.replace("/dev/", "/sys/class/block/")) {
            Ok(disk) => match disk.partitions() {
                Ok(partitions) => {
                    let res = partitions
                        .iter()
                        .map(|p| disks_ipc::Partition {
                            path: p.path.to_string_lossy().to_string(),
                            kind: match p.kind {
                                system::disk::PartitionKind::ESP => disks_ipc::Partition_kind::esp,
                                system::disk::PartitionKind::XBOOTLDR => disks_ipc::Partition_kind::xbootldr,
                                system::disk::PartitionKind::Regular => disks_ipc::Partition_kind::regular,
                            },
                            size: p.size as i64,
                            uuid: p.uuid.clone(),
                            superblock_kind: if let Some(sb) = p.sb.as_ref() {
                                match sb {
                                    system::disk::SuperblockKind::Btrfs => disks_ipc::Partition_superblock_kind::btrfs,
                                    system::disk::SuperblockKind::Ext4 => disks_ipc::Partition_superblock_kind::ext4,
                                    system::disk::SuperblockKind::Luks2 => disks_ipc::Partition_superblock_kind::luks2,
                                    system::disk::SuperblockKind::F2FS => disks_ipc::Partition_superblock_kind::f2fs,
                                    system::disk::SuperblockKind::Xfs => disks_ipc::Partition_superblock_kind::xfs,
                                    // FIXME: introduce FAT kind?
                                    system::disk::SuperblockKind::Fat => disks_ipc::Partition_superblock_kind::unknown,
                                }
                            } else {
                                disks_ipc::Partition_superblock_kind::unknown
                            },
                        })
                        .collect::<Vec<_>>();

                    // Cache the partitions
                    cell.insert(disk.path.to_string_lossy().to_string(), res.clone());
                    call.reply(res)?;
                }
                Err(e) => return call.reply_disk_error(e.to_string()),
            },
            Err(e) => return call.reply_disk_error(e.to_string()),
        }

        Ok(())
    }
}
