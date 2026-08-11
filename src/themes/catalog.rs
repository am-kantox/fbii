use ratatui::style::{Color, Modifier, Style};

pub const THEME_NAMES: &[&str] = &[
    "nord-dark",
    "nord-light",
    "auto",
    "dracula",
    "monokai",
    "github-dark",
    "github-light",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub name: String,
    pub background: Color,
    pub foreground: Color,
    pub heading: Color,
    pub accent: Color,
    pub selection: Color,
    pub highlight: Color,
    pub status_bg: Color,
    pub status_fg: Color,
    pub border: Color,
}

impl Theme {
    pub fn nord_dark() -> Self {
        Self {
            name: "nord-dark".to_string(),
            background: Color::Rgb(46, 52, 64),    // nord0
            foreground: Color::Rgb(236, 239, 244), // nord6
            heading: Color::Rgb(143, 188, 187),    // nord7 (Frost)
            accent: Color::Rgb(136, 192, 208),     // nord8 (Frost)
            selection: Color::Rgb(67, 76, 94),     // nord2
            highlight: Color::Rgb(235, 203, 139),  // nord13 (Yellow)
            status_bg: Color::Rgb(59, 66, 82),     // nord1
            status_fg: Color::Rgb(236, 239, 244),  // nord6
            border: Color::Rgb(129, 161, 193),     // nord9
        }
    }

    pub fn nord_light() -> Self {
        Self {
            name: "nord-light".to_string(),
            background: Color::Rgb(236, 239, 244), // nord6
            foreground: Color::Rgb(46, 52, 64),    // nord0
            heading: Color::Rgb(94, 129, 172),     // nord10
            accent: Color::Rgb(191, 97, 106),      // nord11 (Aurora red)
            selection: Color::Rgb(229, 233, 240),  // nord5
            highlight: Color::Rgb(208, 135, 112),  // nord12 (Orange)
            status_bg: Color::Rgb(216, 222, 233),  // nord4
            status_fg: Color::Rgb(46, 52, 64),     // nord0
            border: Color::Rgb(143, 188, 187),     // nord7
        }
    }

    pub fn dracula() -> Self {
        Self {
            name: "dracula".to_string(),
            background: Color::Rgb(40, 42, 54),
            foreground: Color::Rgb(248, 248, 242),
            heading: Color::Rgb(189, 147, 249),
            accent: Color::Rgb(255, 121, 198),
            selection: Color::Rgb(68, 71, 90),
            highlight: Color::Rgb(241, 250, 140),
            status_bg: Color::Rgb(98, 114, 164),
            status_fg: Color::Rgb(248, 248, 242),
            border: Color::Rgb(98, 114, 164),
        }
    }

    pub fn monokai() -> Self {
        Self {
            name: "monokai".to_string(),
            background: Color::Rgb(39, 40, 34),
            foreground: Color::Rgb(248, 248, 242),
            heading: Color::Rgb(166, 226, 46),
            accent: Color::Rgb(249, 38, 114),
            selection: Color::Rgb(73, 72, 62),
            highlight: Color::Rgb(230, 219, 116),
            status_bg: Color::Rgb(73, 72, 62),
            status_fg: Color::Rgb(248, 248, 242),
            border: Color::Rgb(166, 226, 46),
        }
    }

    pub fn github_dark() -> Self {
        Self {
            name: "github-dark".to_string(),
            background: Color::Rgb(13, 17, 23),
            foreground: Color::Rgb(201, 209, 217),
            heading: Color::Rgb(88, 166, 255),
            accent: Color::Rgb(240, 136, 62),
            selection: Color::Rgb(33, 38, 45),
            highlight: Color::Rgb(210, 153, 34),
            status_bg: Color::Rgb(22, 27, 34),
            status_fg: Color::Rgb(201, 209, 217),
            border: Color::Rgb(48, 54, 61),
        }
    }

    pub fn github_light() -> Self {
        Self {
            name: "github-light".to_string(),
            background: Color::Rgb(255, 255, 255),
            foreground: Color::Rgb(36, 41, 47),
            heading: Color::Rgb(9, 105, 218),
            accent: Color::Rgb(207, 34, 46),
            selection: Color::Rgb(234, 238, 242),
            highlight: Color::Rgb(154, 103, 0),
            status_bg: Color::Rgb(246, 248, 250),
            status_fg: Color::Rgb(36, 41, 47),
            border: Color::Rgb(208, 215, 222),
        }
    }

    pub fn system_default() -> Self {
        match dark_light::detect() {
            dark_light::Mode::Light => Self::nord_light(),
            _ => Self::nord_dark(),
        }
    }

    pub fn get_by_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "nord" | "nord-dark" | "nord_dark" => Self::nord_dark(),
            "nord-light" | "nord_light" => Self::nord_light(),
            "dracula" => Self::dracula(),
            "monokai" => Self::monokai(),
            "github-dark" | "github_dark" => Self::github_dark(),
            "github-light" | "github_light" => Self::github_light(),
            "system" | "default" | "auto" => Self::system_default(),
            _ => Self::system_default(),
        }
    }

    pub fn base_style(&self) -> Style {
        Style::default().bg(self.background).fg(self.foreground)
    }

    pub fn heading_style(&self) -> Style {
        Style::default()
            .bg(self.background)
            .fg(self.heading)
            .add_modifier(Modifier::BOLD)
    }

    pub fn status_style(&self) -> Style {
        Style::default()
            .bg(self.status_bg)
            .fg(self.status_fg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn highlight_style(&self) -> Style {
        Style::default()
            .bg(self.highlight)
            .fg(self.background)
            .add_modifier(Modifier::BOLD)
    }
}
