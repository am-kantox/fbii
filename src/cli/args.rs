use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "fbii",
    author,
    version,
    about = "A terminal e-book reader for FB2, FB2-in-ZIP and EPUB (2.x / 3.x) with vim-like controls",
    long_about = None
)]
pub struct CliArgs {
    /// Path to e-book file to open (.fb2, .fb2.zip, .epub)
    #[arg(value_name = "FILE")]
    pub file_path: Option<PathBuf>,

    /// Force opening in library mode
    #[arg(short, long)]
    pub library: bool,

    /// Override color theme (e.g. dracula, monokai, ayu-dark, github-dark)
    #[arg(short, long)]
    pub theme: Option<String>,

    /// Custom path to TOML config file
    #[arg(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Recursively scan a directory for e-book files and import them into
    /// the library before starting
    #[arg(long, value_name = "DIR")]
    pub scan_dir: Option<PathBuf>,

    /// Override image rendering graphics protocol (auto, kitty, sixel, iterm2, halfblocks, none)
    #[arg(long = "image-protocol", value_name = "PROTOCOL")]
    pub image_protocol: Option<String>,
}
