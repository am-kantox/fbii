# Configuration System

`fbii` reads configuration from `~/.config/fbii/config.toml` (or `XDG_CONFIG_HOME/fbii/config.toml`).

## Example Configuration

```toml
theme = "dracula"
db_path = "~/.config/fbii/library.db"

[typography]
font_family = "sans-serif"
font_size = 14
line_height = 1.4
measure = 80 # Clamped between 30 and 200
paragraph_indent = 2
paragraph_spacing = 1

[display]
simplified_mode = false
show_progress_bar = true
show_status_line = true

[keymap.bindings]
"j" = "ScrollDown"
"k" = "ScrollUp"
"ctrl+d" = "HalfPageDown"
"ctrl+u" = "HalfPageUp"
"gg" = "GotoTop"
"G" = "GotoBottom"
"/" = "Search"
"n" = "NextMatch"
"N" = "PrevMatch"
"t" = "Toc"
"b" = "AddBookmark"
"B" = "ListBookmarks"
"S" = "ToggleSimpleMode"
"q" = "Quit"
```
