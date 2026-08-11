use ratatui::style::{Color, Style, Modifier};

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

    pub fn get_by_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "monokai" => Self::monokai(),
            "github-dark" | "github_dark" => Self::github_dark(),
            "github-light" | "github_light" => Self::github_light(),
            _ => Self::dracula(),
        }
    }

    pub fn base_style(&self) -> Style {
        Style::default().bg(self.background).fg(self.foreground)
    }

    pub fn heading_style(&self) -> Style {
        Style::default().bg(self.background).fg(self.heading).add_modifier(Modifier::BOLD)
    }

    pub fn status_style(&self) -> Style {
        Style::default().bg(self.status_bg).fg(self.status_fg).add_modifier(Modifier::BOLD)
    }

    pub fn highlight_style(&self) -> Style {
        Style::default().bg(self.highlight).fg(self.background).add_modifier(Modifier::BOLD)
    }
}
