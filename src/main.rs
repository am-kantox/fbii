use clap::Parser;
use tabook::cli::CliArgs;
use tabook::config::Config;
use tabook::utils::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArgs::parse();

    let config_path = args
        .config
        .clone()
        .unwrap_or_else(Config::default_config_path);

    let mut config = Config::load_from_file(&config_path)?;
    if let Some(theme_override) = args.theme {
        config.theme = theme_override;
    }

    println!("tabook v{}", env!("CARGO_PKG_VERSION"));
    if let Some(file_path) = args.file_path {
        println!("Opening file: {}", file_path.display());
    } else {
        println!("Opening library mode...");
    }

    Ok(())
}
