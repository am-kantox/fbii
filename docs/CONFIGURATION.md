# Configuration Guide

`tabook` reads configuration from `~/.config/tabook/config.toml` (or `XDG_CONFIG_HOME/tabook/config.toml`).

## Example Configuration

```toml
theme = "dracula"
db_path = "~/.config/tabook/library.db"

[typography]
measure = 80              # Max characters per line (clamped between 30 and 200)
line_spacing = 1          # Spacing between text lines
paragraph_indent = 2      # Spaces to indent first line of paragraphs
paragraph_spacing = 1     # Lines between paragraphs
hyphenation = false       # Enable soft word hyphenation

[display]
simplified_mode = false   # Render text without complex block styling
respect_epub_css = true   # Honor publisher CSS alignment & styles in EPUB
image_protocol = "auto"   # Options: "auto", "kitty", "sixel", "iterm2", "halfblocks", "off"

[keymap.bindings]
"j" = "scroll_down"
"k" = "scroll_up"
"ctrl+f" = "page_down"
"ctrl+b" = "page_up"
"ctrl+d" = "half_page_down"
"ctrl+u" = "half_page_up"
"gg" = "goto_top"
"G" = "goto_bottom"
"/" = "search"
"n" = "next_match"
"N" = "prev_match"
"o" = "open_file"
"s" = "save_to_library"
"b" = "add_bookmark"
"B" = "list_bookmarks"
"t" = "toc"
"i" = "info"
"?" = "help"
"q" = "quit"
":" = "command"
"S" = "toggle_simple_mode"
"C" = "toggle_css"
```

## Keybinding Conflict Validation

Keys are normalized (e.g. `Ctrl+d` $\rightarrow$ `ctrl+d`). Binding the same key sequence to two conflicting actions will produce a configuration error on startup.
