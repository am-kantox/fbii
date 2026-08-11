use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "tabook",
    author,
    version,
    about = "A terminal e-book reader for FB2, FB2-in-ZIP and EPUB with vim-like controls built in Rust",
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
}
