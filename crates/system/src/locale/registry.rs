// SPDX-FileCopyrightText: Copyright © 2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Registry of languages and territories.

use std::collections::HashMap;

use fs_err as fs;

use super::{iso_3166, iso_639_2, iso_639_3, Error, Language, Locale, Territory};

/// All ISO codes are expected to live in this location
const ISO_CODES_BASE: &str = "/usr/share/iso-codes/json";

/// Manage locales + territories
pub struct Registry {
    places: Vec<Territory>,
    places_lookup: HashMap<String, usize>,
    languages: Vec<Language>,
    languages_lookup: HashMap<String, usize>,
}

impl Registry {
    /// Create a new locale registry from the system iso-code JSON definitions
    pub fn new() -> Result<Self, Error> {
        let places = Self::load_territories()?;
        let mut places_lookup = HashMap::new();
        for (index, item) in places.iter().enumerate() {
            places_lookup.insert(item.code2.to_lowercase(), index);
            places_lookup.insert(item.code.to_lowercase(), index);
        }

        //  Convert all languages into usable ones with mapping
        let mut languages = Self::load_languages_2()?;
        languages.extend(Self::load_languages_3()?);
        let mut languages_lookup = HashMap::new();
        for (index, language) in languages.iter().enumerate() {
            if let Some(code2) = language.code2.as_ref() {
                languages_lookup.insert(code2.to_lowercase(), index);
            }
            languages_lookup.insert(language.code.to_lowercase(), index);
        }

        Ok(Self {
            places,
            places_lookup,
            languages,
            languages_lookup,
        })
    }

    /// Load all the territories
    fn load_territories() -> Result<Vec<Territory>, Error> {
        // Load the territories in
        let territories = format!("{ISO_CODES_BASE}/iso_3166-1.json");
        let contents = fs::read_to_string(territories)?;
        let parser = serde_json::from_str::<iso_3166::Document<'_>>(&contents)?;

        Ok(parser.entries.iter().map(|e| e.into()).collect::<Vec<_>>())
    }

    /// Load the 2 DB
    fn load_languages_2() -> Result<Vec<Language>, Error> {
        let languages = format!("{ISO_CODES_BASE}/iso_639-2.json");
        let contents = fs::read_to_string(languages)?;
        let parser = serde_json::from_str::<iso_639_2::Document<'_>>(&contents)?;

        Ok(parser.entries.iter().map(|e| e.into()).collect::<Vec<_>>())
    }

    /// Load the 3 DB
    fn load_languages_3() -> Result<Vec<Language>, Error> {
        let languages = format!("{ISO_CODES_BASE}/iso_639-3.json");
        let contents = fs::read_to_string(languages)?;
        let parser = serde_json::from_str::<iso_639_3::Document<'_>>(&contents)?;

        Ok(parser.entries.iter().map(|e| e.into()).collect::<Vec<_>>())
    }

    /// Retrieve the territory for the given (lower-case) code
    pub fn territory(&self, id: impl AsRef<str>) -> Option<&Territory> {
        if let Some(idx) = self.places_lookup.get(id.as_ref()) {
            self.places.get(*idx)
        } else {
            None
        }
    }

    /// Retrieve the language for the given (lower-case) code
    pub fn language(&self, id: impl AsRef<str>) -> Option<&Language> {
        if let Some(idx) = self.languages_lookup.get(id.as_ref()) {
            self.languages.get(*idx)
        } else {
            None
        }
    }

    /// Attempt to retrieve a locale combination
    pub fn locale(&self, id: impl AsRef<str>) -> Option<Locale<'_>> {
        let id = id.as_ref().to_lowercase();

        // Handle .codeset
        let (left, codeset) = if let Some(idx) = id.find('.') {
            id.split_at(idx)
        } else {
            (id.as_str(), "")
        };

        // Fix "utf8" codeset
        let codeset = if codeset.is_empty() {
            None
        } else {
            Some(
                codeset
                    .replace("utf8", "UTF-8")
                    .chars()
                    .skip(1)
                    .collect::<String>()
                    .to_uppercase(),
            )
        };

        // Now handle a modifier
        let (code, modifier) = if let Some(idx) = left.find('@') {
            left.split_at(idx)
        } else {
            (left, "")
        };
        let modifier = if modifier.is_empty() {
            None
        } else {
            Some(modifier.chars().skip(1).collect::<String>().to_uppercase())
        };

        // Split on '_' and map into language/territory
        let (l_code, t_code) = code.split_once('_')?;
        let language = self.language(l_code)?;
        let territory = self.territory(t_code)?;

        // Cook functioning names/ids with fixed formatting
        let display_name = format!("{} ({})", &language.display_name, &territory.display_name);
        let mut new_id = Vec::new();
        new_id.push(l_code.into());
        new_id.push("_".into());
        new_id.push(t_code.to_uppercase());
        if let Some(m) = modifier.as_ref() {
            new_id.push(format!("@{m}"));
        }
        if let Some(codeset) = codeset.as_ref() {
            new_id.push(format!(".{codeset}"));
        }

        Some(Locale {
            name: new_id.into_iter().collect(),
            display_name,
            language,
            territory,
            codeset,
            modifier,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::Registry;

    #[test]
    fn test_territory() {
        let r = Registry::new().expect("Failed to initialise registry");
        let ie = r.territory("ie").expect("Cannot find Ireland by ie");
        let irl = r.territory("irl").expect("Cannot find Ireland by irl");
        assert_eq!(ie, irl);
        assert_eq!(irl.display_name, "Ireland");

        let dk = r.territory("dk").expect("Cannot find Denmark by dk");
        assert_eq!(dk.display_name, "Kingdom of Denmark");
        eprintln!("dk = {dk:?}");
    }

    #[test]
    fn test_language() {
        let r = Registry::new().expect("Failed to initialise registry");
        let en = r.language("en").expect("Cannot find English by en");
        assert_eq!(en.display_name, "English");

        let dan = r.language("dan").expect("Cannot find Danish by dan");
        let dn = r.language("da").expect("Cannot find Danish by dn");
        assert_eq!(dan, dn);
    }

    #[test]
    fn test_locale() {
        let r = Registry::new().expect("Failed to initialise registry");
        let en_ie = r.locale("en_IE.UTF-8").expect("Failed to find en_IE.UTF-8");
        assert_eq!(en_ie.display_name, "English (Ireland)");
        let ga_ie = r.locale("ga_IE.UTF-8").expect("Failed to find ga_IE.UTF-8");
        assert_eq!(ga_ie.display_name, "Irish (Ireland)");

        eprintln!("en_IE = {en_ie:?}");
        eprintln!("ga_IE = {ga_ie:?}");
    }

    #[test]
    fn test_get_locales() {
        let r = Registry::new().expect("Failed to initialise registry");
        let output = Command::new("localectl")
            .arg("list-locales")
            .output()
            .expect("Failed to run localectl");
        let output = String::from_utf8(output.stdout).expect("Cannot decode output");
        for line in output.lines() {
            if line == "C.UTF-8" {
                continue;
            }
            eprintln!("looking up {line}");
            let locale = r
                .locale(line)
                .unwrap_or_else(|| panic!("Failed to find a predefined locale {line}"));
            eprintln!("locale {line} = {locale:?}");
        }
    }
}
