// SPDX-FileCopyrightText: Copyright © 2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Lichen installation steps

use std::{fmt::Debug, process::ExitStatus};
use thiserror::Error;

mod context;
pub use context::Context;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    IO(#[from] std::io::Error),

    #[error("unknown filesystem")]
    UnknownFilesystem,

    #[error("no mountpoint given")]
    NoMountpoint,

    #[error("command `{program}` exited with {status}")]
    CommandFailed { program: String, status: ExitStatus },
}

#[derive(Debug)]
pub enum Step<'a> {
    AddRepo(Box<AddRepo>),
    Bind(Box<BindMount>),
    CreateUser(Box<CreateAccount<'a>>),
    Format(Box<FormatPartition<'a>>),
    Install(Box<InstallPackages>),
    Mount(Box<MountPartition<'a>>),
    SetPassword(Box<SetPassword<'a>>),
    SetLocale(Box<SetLocale<'a>>),
    ResetMachineID(Box<ResetMachineID>),
    SetTimezone(Box<SetTimezone<'a>>),
    WriteFstab(Box<EmitFstab>),
}

impl<'a> Step<'a> {
    /// Create new repo step
    pub fn add_repo(r: AddRepo) -> Self {
        Self::AddRepo(Box::new(r))
    }

    pub fn create_user(u: CreateAccount<'a>) -> Self {
        Self::CreateUser(Box::new(u))
    }

    pub fn install_packages(p: InstallPackages) -> Self {
        Self::Install(Box::new(p))
    }

    /// Create new FormatPartition step
    pub fn format(f: FormatPartition<'a>) -> Self {
        Self::Format(Box::new(f))
    }

    /// Create new MountPartition step
    pub fn mount(m: MountPartition<'a>) -> Self {
        Self::Mount(Box::new(m))
    }

    /// Create new bind mount
    pub fn bind_mount(b: BindMount) -> Self {
        Self::Bind(Box::new(b))
    }

    /// Set system locale
    pub fn set_locale(l: SetLocale<'a>) -> Self {
        Self::SetLocale(Box::new(l))
    }

    /// Set system timezone
    pub fn set_timezone(t: SetTimezone<'a>) -> Self {
        Self::SetTimezone(Box::new(t))
    }

    /// Set an account password
    pub fn set_password(a: SetPassword<'a>) -> Self {
        Self::SetPassword(Box::new(a))
    }

    /// Construct a dbus/systemd machine id
    pub fn set_machine_id() -> Self {
        Self::ResetMachineID(Box::new(ResetMachineID {}))
    }

    // Emit the given fstab
    pub fn emit_fstab(f: EmitFstab) -> Self {
        Self::WriteFstab(Box::new(f))
    }

    /// Return a unique short ID name for the steps
    pub fn name(&self) -> &'static str {
        match &self {
            Step::AddRepo(_) => "add-repo",
            Step::Bind(_) => "bind-mount",
            Step::CreateUser(_) => "create-user",
            Step::Format(_) => "format-partition",
            Step::Install(_) => "install-packages",
            Step::Mount(_) => "mount-partition",
            Step::SetPassword(_) => "set-password",
            Step::SetLocale(_) => "set-locale",
            Step::SetTimezone(_) => "set-timezone",
            Step::ResetMachineID(_) => "reset-machine-id",
            Step::WriteFstab(_) => "write-fstab",
        }
    }

    /// Return the display title for a step
    pub fn title(&self) -> String {
        match &self {
            Step::AddRepo(s) => s.title(),
            Step::Bind(s) => s.title(),
            Step::CreateUser(s) => s.title(),
            Step::Format(s) => s.title(),
            Step::Install(s) => s.title(),
            Step::Mount(s) => s.title(),
            Step::SetPassword(s) => s.title(),
            Step::SetLocale(s) => s.title(),
            Step::SetTimezone(s) => s.title(),
            Step::ResetMachineID(s) => s.title(),
            Step::WriteFstab(s) => s.title(),
        }
    }

    /// Describe the action/context for the step
    pub fn describe(&self) -> String {
        match &self {
            Step::AddRepo(s) => s.describe(),
            Step::Bind(s) => s.describe(),
            Step::CreateUser(s) => s.describe(),
            Step::Format(s) => s.describe(),
            Step::Install(s) => s.describe(),
            Step::Mount(s) => s.describe(),
            Step::SetPassword(s) => s.describe(),
            Step::SetLocale(s) => s.describe(),
            Step::SetTimezone(s) => s.describe(),
            Step::ResetMachineID(s) => s.describe(),
            Step::WriteFstab(s) => s.describe(),
        }
    }

    /// Execute a step asynchronously. Implementations can opt-in to async.
    pub fn execute(&self, context: &'a impl Context<'a>) -> Result<(), Error> {
        match &self {
            Step::AddRepo(s) => Ok(s.execute(context)?),
            Step::Bind(s) => Ok(s.execute(context)?),
            Step::CreateUser(s) => Ok(s.execute(context)?),
            Step::Format(s) => Ok(s.execute(context)?),
            Step::Install(s) => Ok(s.execute(context)?),
            Step::Mount(s) => Ok(s.execute(context)?),
            Step::SetPassword(s) => Ok(s.execute(context)?),
            Step::SetLocale(s) => Ok(s.execute(context)?),
            Step::SetTimezone(s) => Ok(s.execute(context)?),
            Step::ResetMachineID(s) => Ok(s.execute(context)?),
            Step::WriteFstab(s) => Ok(s.execute(context)?),
        }
    }

    /// Determine whether an indeterminate progress spinner is needed
    /// In the CLI frontend this is abused to hide the progressbar when invoking moss.
    pub fn is_indeterminate(&self) -> bool {
        !matches!(self, Step::Install(_))
    }
}

mod partitions;

pub use partitions::{BindMount, FormatPartition, MountPartition, Unmount};

mod packaging;
pub use packaging::{AddRepo, InstallPackages};

mod cleanup;
pub use cleanup::Cleanup;

mod postinstall;
pub use postinstall::{CreateAccount, EmitFstab, FstabEntry, ResetMachineID, SetLocale, SetPassword, SetTimezone};
