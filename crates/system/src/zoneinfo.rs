// SPDX-FileCopyrightText: Copyright © 2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0
use std::collections::{HashMap, HashSet};

use fs_err as fs;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    IO(#[from] std::io::Error),
}

const ZONEINFO_BASE: &str = "/usr/share/zoneinfo";

pub struct Registry {
    timezones: Vec<String>,
    timezones_lookup: HashMap<String, Vec<usize>>,
}

impl Registry {
    pub fn new() -> Result<Self, Error> {
        let timezones_table = Self::load_timezones_table()?;
        let mut timezones: Vec<String> = timezones_table
            .values()
            .flat_map(|zones| zones.iter().cloned())
            .collect::<HashSet<String>>()
            .into_iter()
            .collect();
        timezones.sort();

        let timezone_index_lookup: HashMap<&str, usize> =
            timezones.iter().enumerate().map(|(i, v)| (v.as_str(), i)).collect();

        let timezones_lookup: HashMap<String, Vec<usize>> = timezones_table
            .into_iter()
            .map(|(code2, zones)| {
                let indices = zones.iter().map(|tz| timezone_index_lookup[tz.as_str()]).collect();
                (code2, indices)
            })
            .collect();

        Ok(Self {
            timezones,
            timezones_lookup,
        })
    }

    /// Parse the TSV file zone1970.tab with the following structure:
    ///   Field 1: List of 2 character country codes, comma delimited
    ///   Field 2: Latitude/Longitude
    ///   Field 3: Timezone name
    ///   Field 4: Optional Comments
    /// Only fields 1 and 3 are extracted as a HashMap which maps
    /// 2 character country codes to a collection of associated timezones
    fn load_timezones_table() -> Result<HashMap<String, Vec<String>>, std::io::Error> {
        let zone_tab = format!("{ZONEINFO_BASE}/zone1970.tab");
        let contents = fs::read_to_string(zone_tab)?;
        let mut timezones_lookup: HashMap<String, Vec<String>> = HashMap::new();

        for line in contents.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.splitn(4, '\t');
            let (Some(codes), Some(_), Some(timezone)) = (fields.next(), fields.next(), fields.next()) else {
                continue;
            };
            for code2 in codes.split(',') {
                timezones_lookup
                    .entry(code2.to_string())
                    .or_default()
                    .push(timezone.to_string());
            }
        }
        Ok(timezones_lookup)
    }

    pub fn all_timezones(&self) -> &[String] {
        &self.timezones
    }

    pub fn timezones_for_territory(&self, code2: &str) -> Vec<&str> {
        self.timezones_lookup
            .get(code2)
            .map(|indices| indices.iter().map(|&i| self.timezones[i].as_str()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_zone_territory() {
        let r = Registry::new().expect("Failed to initialise registry");

        let test_cases = [("GB", "Europe/London"), ("IE", "Europe/Dublin")];

        for (code2, expected_timezone) in test_cases {
            let timezones = r.timezones_for_territory(code2);
            assert!(timezones.len() == 1);
            assert!(timezones.contains(&expected_timezone));
            eprintln!("Available timezones: {timezones:?}");
        }
    }

    #[test]
    fn test_multi_zone_territory() {
        let r = Registry::new().expect("Failed to initialise registry");

        let test_cases = HashMap::from([
            ("AU", vec!["Australia/Sydney", "Australia/Perth"]),
            ("US", vec!["America/New_York", "America/Los_Angeles"]),
        ]);
        for (code2, expected_timezones) in test_cases {
            let timezones = r.timezones_for_territory(code2);
            for expected in expected_timezones {
                assert!(timezones.contains(&expected));
            }
            eprintln!("Available timezones: {timezones:?}");
        }
    }

    #[test]
    fn test_exclusive_timezones() {
        let r = Registry::new().expect("Failed to initialise registry");

        let test_cases = HashMap::from([
            ("AU", vec!["America/New_York", "America/Los_Angeles"]),
            ("US", vec!["Australia/Sydney", "Australia/Perth"]),
        ]);
        for (code2, expected_timezones) in test_cases {
            let timezones = r.timezones_for_territory(code2);
            for expected in expected_timezones {
                assert!(!timezones.contains(&expected));
            }
            eprintln!("Available timezones: {timezones:?}");
        }
    }
}
