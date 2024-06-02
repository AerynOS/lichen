// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Theme definitions

use std::sync::OnceLock;

use ratatui::style::palette::tailwind;
use ratatui::style::Color;

pub struct Theme {
    /// Text colour
    pub color_text: Color,

    /// Selection colour
    pub color_selection: Color,

    /// Highlight (hover) colour
    pub color_highlight: Color,

    /// Inactive (not-focused) colour
    pub color_inactive: Color,
}

/// Basic theme for tty/non-256/emoji use
pub static BASIC: Theme = Theme {
    color_text: Color::White,
    color_selection: Color::LightBlue,
    color_highlight: Color::White,
    color_inactive: Color::DarkGray,
};

/// Refined theme for desktop use
pub static REFINED: Theme = Theme {
    color_text: Color::White,
    color_selection: tailwind::BLUE.c300,
    color_highlight: tailwind::SLATE.c400,
    color_inactive: tailwind::SLATE.c500,
};

/// Access the default theme
pub fn current() -> &'static Theme {
    static RES: OnceLock<&'static Theme> = OnceLock::new();
    RES.get_or_init(|| match crossterm::style::available_color_count() {
        x if x > 255 => &REFINED,
        _ => &BASIC,
    })
}
