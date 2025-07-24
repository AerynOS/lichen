// SPDX-FileCopyrightText: Copyright © 2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Deserialising of selections from JSON files

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    str::FromStr,
};

use itertools::Itertools;
use serde::Deserialize;
use thiserror::Error;

/// Selection handling errors
#[derive(Debug, Error)]
pub enum Error {
    #[error("serde: {0}")]
    Deserialize(#[from] serde_json::Error),

    #[error("unknown group")]
    UnknownGroup(String),
}

#[derive(Debug, Default, Deserialize)]
pub struct Group {
    /// Simple list-selection name for this selection group
    pub name: String,

    /// User-visible summary for this selection group
    pub summary: String,

    /// User visible description for this selection group
    pub description: String,

    /// Whether or not to display the "group" as an installable option
    #[serde(default)]
    pub display: bool,

    /// Priority for sorting desktops (lower priorities are sorted before higher ones)
    #[serde(default)]
    pub priority: u8,

    /// Optionally a set of selection groups forming the basis of this one
    #[serde(default)]
    pub depends: Vec<String>,

    /// A set of package names (moss-encoded) that form this selection
    pub required: Vec<String>,
}

impl Display for Group {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.summary)
    }
}

impl FromStr for Group {
    type Err = Error;

    /// Encapsulate serde_json::from_str()
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let us = serde_json::from_str(s)?;
        Ok(us)
    }
}

/// Simple selections management
#[derive(Default)]
pub struct Manager {
    groups: BTreeMap<String, Group>,
}

impl Manager {
    /// convenience: new Manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Take ownership of some groups
    pub fn with_groups<I: IntoIterator<Item = Group>>(self, groups: I) -> Self {
        Self {
            groups: groups
                .into_iter()
                .map(|g| (g.name.clone(), g))
                .collect::<BTreeMap<_, _>>(),
        }
    }

    /// Add a group to the manager
    pub fn insert(&mut self, g: Group) {
        self.groups.insert(g.name.clone(), g);
    }

    /// Return an iterator of references to the groups
    pub fn groups(&self) -> impl Iterator<Item = &'_ Group> {
        self.groups.values().sorted_by_key(|x| x.priority)
    }

    /// privatwly recurse for string deps
    fn get_deps(&self, name: &str) -> Result<Vec<String>, Error> {
        let group = self.groups.get(name).ok_or_else(|| Error::UnknownGroup(name.into()))?;
        let mut depends = group.depends.clone();

        // Recursively build parent deps
        for parent in group.depends.iter() {
            let parent_deps = self.get_deps(parent)?;
            depends.extend(parent_deps)
        }

        Ok(depends)
    }

    /// Given the selected IDs, what are the total selections?
    pub fn selections_with<'a, I: IntoIterator<Item = &'a str>>(&'a self, ids: I) -> Result<BTreeSet<String>, Error> {
        let mut selected_ids = BTreeSet::new();
        for item in ids.into_iter() {
            let deps = self.get_deps(item)?;
            selected_ids.extend(deps);
            selected_ids.insert(item.into());
        }
        let core = selected_ids.into_iter().filter_map(|id| self.groups.get(&id));
        Ok(core.flat_map(|g| g.required.clone()).collect::<BTreeSet<_>>())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::selections::{Group, Manager};

    #[test]
    fn test_decode() {
        let d = Group::from_str(include_str!("../../../selections/develop.json")).expect("Failed to decode base JSON");
        let b = Group::from_str(include_str!("../../../selections/base.json")).expect("Failed to decode base JSON");

        let manager = Manager::new().with_groups([d, b]);
        let pkgs_partial = manager.selections_with(["base"]).expect("Needed single set of data");
        let pkgs = manager
            .selections_with(["develop"])
            .expect("Needed empty set of base selections");
        assert_eq!(pkgs_partial.len(), 44);
        assert_eq!(pkgs.len(), 48);
    }
}
