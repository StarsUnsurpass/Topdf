mod converter;
mod ui;

use iced::Result;
use ui::App;

fn main() -> Result {
    if let Err(e) = setup_logger() {
        eprintln!("Failed to initialize logger: {}", e);
    }
    log::info!("Application started");

    iced::application(App::new, App::update, App::view)
        .run()
}

fn setup_logger() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let now = chrono::Local::now();
    std::fs::create_dir_all("logs")?;
    let log_filename = format!("logs/topdf_{}.log", now.format("%Y-%m-%d_%H-%M-%S"));

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d %H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file(log_filename)?)
        .apply()?;
    Ok(())
}