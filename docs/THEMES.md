# Theme Catalog & Customization

`tabook` features a built-in color theme system powered by Ratatui color tokens (`src/themes/catalog.rs`).

## Built-in Themes

| Theme Name | Style Description | Primary Colors |
| :--- | :--- | :--- |
| **`dracula`** (Default) | Dark vampire theme | Background: `#282A36`, Foreground: `#F8F8F2`, Heading: `#BD93F9`, Accent: `#FF79C6` |
| **`monokai`** | High-contrast dark theme | Background: `#272822`, Foreground: `#F8F8F2`, Heading: `#A6E22E`, Accent: `#F92672` |
| **`github-dark`** | Modern GitHub dark theme | Background: `#0D1117`, Foreground: `#C9D1D9`, Heading: `#58A6FF`, Accent: `#F0883E` |
| **`github-light`** | Clean GitHub light theme | Background: `#FFFFFF`, Foreground: `#24292F`, Heading: `#0969DA`, Accent: `#CF222E` |

## Configuring Themes

In `~/.config/tabook/config.toml`:

```toml
theme = "dracula" # Options: "dracula", "monokai", "github-dark", "github-light"
```

Or pass via CLI flag:

```bash
tabook --theme monokai /path/to/book.epub
```
