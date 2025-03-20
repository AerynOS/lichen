// SPDX-FileCopyrightText: Copyright © 2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Concrete implementation of the isntaller

use std::path::Path;

use system::{
    disk::{self, Disk},
    locale::{self, Locale},
};
use thiserror::Error;
use topology::disk::Builder;

use crate::{
    steps::{
        self, AddRepo, BindMount, Cleanup, Context, CreateAccount, EmitFstab, FormatPartition, FstabEntry,
        InstallPackages, MountPartition, SetLocale, SetPassword, Step, Unmount,
    },
    BootPartition, Model, SystemPartition,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("disk: {0}")]
    Disk(#[from] disk::Error),

    #[error("locale: {0}")]
    Locale(#[from] locale::Error),

    #[error("missing mandatory partition: {0}")]
    MissingPartition(&'static str),

    #[error("steps: {0}")]
    Steps(#[from] steps::Error),

    #[error("unknown locale code: {0}")]
    UnknownLocale(String),

    #[error("topology: {0}")]
    Topology(#[from] topology::disk::Error),
}

/// The installer does some initial probing and is used with a Model
/// to build an execution routine
pub struct Installer {
    /// Complete locale registry
    locale_registry: locale::Registry,

    /// Boot partitions
    boot_parts: Vec<BootPartition>,

    /// System partitions
    system_parts: Vec<SystemPartition>,
}

impl Installer {
    /// Return a newly initialised installer
    pub fn new() -> Result<Self, Error> {
        let locale_registry = locale::Registry::new()?;
        let disks = Disk::discover()?;

        // Figure out where we live right now and exclude the rootfs
        let probe = Builder::default().build()?;
        let root_nodes = match probe.get_rootfs_device("/") {
            Ok(device) => {
                let mut nodes = probe.get_device_chain(&device.path).unwrap_or_default();
                nodes.push(device.path.into());
                nodes
            }
            _ => {
                vec![]
            }
        };

        // Exclude parent block devices related to `/` partition
        let parents = root_nodes
            .iter()
            .filter_map(|n| probe.get_device_parent(n))
            .collect::<Vec<_>>();

        let mut boot_parts = vec![];
        let mut system_parts = vec![];
        for disk in disks.iter().filter(|d| !parents.iter().any(|r| *r == d.path)) {
            let parts = match disk.partitions() {
                Ok(parts) => parts,
                Err(e) => {
                    log::trace!("Failed to get partitions for `{disk}`: {e}");
                    continue;
                }
            };

            // Exclude partitions related to `/` partition
            let parts = parts
                .into_iter()
                .filter(|p| !root_nodes.iter().any(|r| *r == p.path))
                .collect::<Vec<_>>();
            if let Some(esp) = parts
                .iter()
                .find(|p| matches!(p.kind, disk::PartitionKind::ESP))
                .cloned()
            {
                let xbootldr = parts
                    .iter()
                    .find(|p| matches!(p.kind, disk::PartitionKind::XBOOTLDR))
                    .cloned();
                boot_parts.push(BootPartition {
                    esp,
                    xbootldr,
                    parent_desc: disk.to_string(),
                })
            }
            let others = parts
                .iter()
                .filter(|p| matches!(p.kind, disk::PartitionKind::Regular))
                .cloned();
            system_parts.extend(others.map(|p| SystemPartition {
                partition: p,
                mountpoint: None,
                parent_desc: disk.to_string(),
            }));
        }

        Ok(Self {
            locale_registry,
            system_parts,
            boot_parts,
        })
    }

    /// Allow access to locale registry (mapping IDs)
    pub fn locales(&self) -> &locale::Registry {
        &self.locale_registry
    }

    /// Generate/load the locale map
    pub fn locales_for_ids<S: IntoIterator<Item = impl AsRef<str>>>(&self, ids: S) -> Result<Vec<Locale>, Error> {
        let res = ids
            .into_iter()
            .filter_map(|id| self.locale_registry.locale(id))
            .collect::<Vec<_>>();

        Ok(res)
    }

    /// Return references to the discovered boot partitions
    pub fn boot_partitions(&self) -> &[BootPartition] {
        &self.boot_parts
    }

    /// Return references to the discovered system partitions
    pub fn system_partitions(&self) -> &[SystemPartition] {
        &self.system_parts
    }

    /// build the model into a set of install steps
    pub fn compile_to_steps<'a>(
        &'a self,
        model: &'a Model,
        context: &'a impl Context<'a>,
    ) -> Result<(Vec<Cleanup>, Vec<Step<'a>>), Error> {
        let mut s: Vec<Step<'a>> = vec![];
        let mut c: Vec<Cleanup> = vec![];
        let boot_part = &model.boot_partition.esp;

        let root_partition = model
            .partitions
            .iter()
            .find(|p| {
                if let Some(mount) = p.mountpoint.as_ref() {
                    mount == "/"
                } else {
                    false
                }
            })
            .ok_or(Error::MissingPartition("/"))?;

        // Must format and mount `/` before we can add more mounts
        s.push(Step::format(FormatPartition {
            partition: &root_partition.partition,
            filesystem: model.rootfs_type.clone(),
        }));
        s.push(Step::mount(MountPartition {
            partition: &root_partition.partition,
            mountpoint: context.root().clone(),
        }));
        c.push(Cleanup::unmount(Unmount {
            mountpoint: context.root().clone(),
        }));

        // Mount the ESP
        s.push(Step::mount(MountPartition {
            partition: boot_part,
            mountpoint: context.root().join("efi"),
        }));
        c.push(Cleanup::unmount(Unmount {
            mountpoint: context.root().join("efi"),
        }));

        // Mount xbootldr at `/boot` if present
        if let Some(xbootldr) = model.boot_partition.xbootldr.as_ref() {
            s.push(Step::mount(MountPartition {
                partition: xbootldr,
                mountpoint: context.root().join("boot"),
            }));
            c.push(Cleanup::unmount(Unmount {
                mountpoint: context.root().join("boot"),
            }));
        };

        // Populate vfs bind mounts
        let (mounts, unmounts) = self.create_vfs_mounts(context.root());
        s.extend(mounts);
        c.extend(unmounts);

        // HAX:
        s.push(Step::add_repo(AddRepo {
            uri: "https://packages.aerynos.com/volatile/x86_64/stone.index".into(),
            name: "volatile".into(),
            priority: 0,
        }));
        s.push(Step::install_packages(InstallPackages {
            names: model.packages.iter().cloned().collect::<Vec<_>>(),
        }));

        // Update any passwords
        for account in model.accounts.iter() {
            if !account.builtin {
                s.push(Step::create_user(CreateAccount { account }));
            }
            if let Some(password) = account.password.clone() {
                s.push(Step::set_password(SetPassword { account, password }));
            }
        }

        // System locale
        if let Some(locale) = model.locale {
            s.push(Step::set_locale(SetLocale { locale }));
        }

        // System timezone
        if let Some(timezone) = model.timezone.as_ref() {
            s.push(Step::set_timezone(steps::SetTimezone { timezone }));
        }

        // Ensure we get a machine-id..
        s.push(Step::set_machine_id());

        let (options, pass) = match model.rootfs_type.as_str() {
            "ext4" => ("rw,errors=remount-ro".to_string(), 1),
            _ => ("defaults,rw".to_string(), 0),
        };

        // Write the fstab
        let fstab = EmitFstab::default().with_entries([
            FstabEntry::Comment(format!(
                "{} at time of installation",
                root_partition.partition.path.display()
            )),
            FstabEntry::Device {
                fs: root_partition.partition.uuid.clone(),
                mountpoint: "/".into(),
                kind: model.rootfs_type.clone(),
                opts: options,
                dump: 0,
                pass,
            },
        ]);
        s.push(Step::emit_fstab(fstab));

        // Get the sync call in for unmounts
        c.push(Cleanup::sync_fs());
        // Lastly, flip cleanups to front in reverse (due to mounts)
        c.reverse();
        Ok((c, s))
    }

    fn create_vfs_mounts(&self, prefix: &Path) -> (Vec<Step>, Vec<Cleanup>) {
        const PARTS: &[(&str, &str); 5] = &[
            ("/dev", "dev"),
            ("/dev/shm", "dev/shm"),
            ("/dev/pts", "dev/pts"),
            ("/proc", "proc"),
            ("/sys", "sys"),
        ];
        PARTS
            .iter()
            .map(|(source, dest)| {
                (
                    Step::bind_mount(BindMount {
                        source: source.into(),
                        dest: prefix.join(dest),
                    }),
                    Cleanup::unmount(Unmount {
                        mountpoint: prefix.join(dest),
                    }),
                )
            })
            .collect::<(Vec<_>, Vec<_>)>()
    }
}
