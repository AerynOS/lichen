// SPDX-FileCopyrightText: Copyright © 2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

/// Identifies an account
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Account {
    /// User ID
    pub uid: libc::uid_t,

    /// Group ID
    pub gid: libc::gid_t,

    /// Account name
    pub username: String,

    /// Human username string
    pub gecos: Option<String>,

    /// Home directory
    pub homedir: String,

    /// Which shell to use
    pub shell: String,

    /// New password
    pub password: Option<String>,

    /// Builtin user? (root)
    pub builtin: bool,
}

impl Default for Account {
    fn default() -> Self {
        Self {
            uid: 1000,
            gid: 1000,
            username: "user".into(),
            gecos: None,
            homedir: "/home/user".into(),
            shell: "/usr/bin/bash".into(),
            password: None,
            builtin: false,
        }
    }
}

impl Account {
    /// Return an account definition for the root account
    pub fn root() -> Self {
        Self {
            uid: 0,
            gid: 0,
            username: "root".to_string(),
            homedir: "/root".to_string(),
            builtin: true,
            ..Default::default()
        }
    }

    /// New account with the given username
    pub fn new<S: AsRef<str>>(username: S) -> Self {
        Self {
            username: username.as_ref().to_string(),
            ..Default::default()
        }
    }

    /// Update the IDs
    pub fn with_id(self, uid: libc::uid_t, gid: libc::gid_t) -> Self {
        Self { uid, gid, ..self }
    }

    /// Update the gecos
    pub fn with_gecos<S: AsRef<str>>(self, gecos: S) -> Self {
        Self {
            gecos: Some(gecos.as_ref().to_string()),
            ..self
        }
    }

    /// Update the shell
    pub fn with_shell<S: AsRef<str>>(self, shell: S) -> Self {
        Self {
            shell: shell.as_ref().to_string(),
            ..self
        }
    }

    /// Update the password
    pub fn with_password<P: AsRef<str>>(self, p: P) -> Self {
        Self {
            password: Some(p.as_ref().to_string()),
            ..self
        }
    }
}
