use clap::Parser;
use tabook::cli::CliArgs;
use tabook::config::Config;
use tabook::db::LibraryDb;
use tabook::formats::parse_book_file;
use tabook::tui::App;
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

    let db_path = config
        .db_path
        .as_ref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            dirs::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("tabook")
                .join("library.db")
        });

    let db = LibraryDb::new_at_path(&db_path).await?;
    let mut app = App::new(config, db);

    if let Some(file_path) = args.file_path {
        match parse_book_file(&file_path) {
            Ok(book) => {
                app.db.upsert_book(&book, 0, 0.0).await?;
                app.load_book(book);
            }
            Err(e) => {
                eprintln!("Error opening e-book file '{}': {}", file_path.display(), e);
                std::process::exit(1);
            }
        }
    } else if args.library {
        app.mode = tabook::tui::AppMode::Library;
    }

    app.run_tui().await?;

    Ok(())
}
