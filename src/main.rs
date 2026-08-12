use clap::Parser;
use fbii::cli::CliArgs;
use fbii::config::Config;
use fbii::db::LibraryDb;
use fbii::tui::App;
use fbii::utils::Result;

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
    if let Some(protocol_override) = args.image_protocol {
        config.display.image_protocol = protocol_override;
    }

    let db_path = config
        .db_path
        .as_ref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            dirs::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("fbii")
                .join("library.db")
        });

    let db = LibraryDb::new_at_path(&db_path).await?;

    if let Some(scan_dir) = &args.scan_dir {
        let summary = fbii::library::scan_and_import(&db, scan_dir).await;
        println!(
            "Scanned '{}': {} imported, {} already known, {} failed",
            scan_dir.display(),
            summary.imported,
            summary.skipped,
            summary.failed
        );
    }

    let mut app = App::new(config, db, config_path);

    if let Some(file_path) = args.file_path {
        let uri_str = file_path.to_string_lossy();
        match fbii::formats::parse_book_uri_async(&uri_str).await {
            Ok(book) => {
                app.db.upsert_book(&book, 0, 0.0).await?;
                app.load_book(book).await;
            }
            Err(e) => {
                eprintln!("Error opening e-book URI '{}': {}", uri_str, e);
                std::process::exit(1);
            }
        }
    } else if args.library {
        app.mode = fbii::tui::AppMode::Library;
    }

    app.run_tui().await?;

    Ok(())
}
