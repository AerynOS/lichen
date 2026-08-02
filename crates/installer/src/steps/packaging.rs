// SPDX-FileCopyrightText: Copyright © 2025 AerynOS Developers
// SPDX-License-Identifier: MPL-2.0

//! Package management encapsulation (moss only)
use std::process::Command;

use super::Context;

/// Add a repository to the target disk
#[derive(Debug)]
pub struct AddRepo {
    pub(crate) name: String,
    pub(crate) uri: String,
    pub(crate) version: String,
    pub(crate) priority: u64,
}

impl<'a> AddRepo {
    /// Basic display title
    pub(super) fn title(&self) -> String {
        format!("Add repo {}", self.name)
    }

    /// Render the action
    pub(super) fn describe(&self) -> String {
        format!(
            "{} (channel=main, version={}) [priority {}]",
            self.uri, self.version, self.priority
        )
    }

    /// Run moss against the target, adding a repo
    pub(super) fn execute(&self, context: &'a impl Context<'a>) -> Result<(), super::Error> {
        let mut cmd = Command::new("moss");
        cmd.arg("-D");
        cmd.arg(context.root());
        cmd.args([
            "repo",
            "add",
            &self.name,
            &self.uri,
            "--root-index",
            &self.version,
            "-p",
        ]);
        cmd.arg(self.priority.to_string());
        cmd.arg("-y");

        // Run
        context.run_command(&mut cmd)
    }
}

/// Install packages to destdir
#[derive(Debug)]
pub struct InstallPackages {
    pub(crate) names: Vec<String>,
}

impl<'a> InstallPackages {
    /// Basic display title
    pub(super) fn title(&self) -> String {
        "Install".into()
    }

    /// Render the action
    pub(super) fn describe(&self) -> String {
        "packages to sysroot".into()
    }

    /// Run moss against the target, adding a repo
    pub(super) fn execute(&self, context: &'a impl Context<'a>) -> Result<(), super::Error> {
        let mut cmd = Command::new("moss");
        cmd.arg("-D");
        cmd.arg(context.root());
        cmd.arg("install");
        cmd.args(&self.names);
        cmd.arg("-y");

        context.run_command(&mut cmd)
    }
}
