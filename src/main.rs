use clap::Parser;
use color_eyre::eyre::Result;

mod app;
use app::{cli, gui};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct App {
    #[command(subcommand)]
    command: Option<cli::Command>,
}

fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_PKG_NAME")).into()),
        )
        .with(fmt::layer())
        .init();

    color_eyre::install()?;

    let App { command } = App::parse();

    if let Some(command) = command {
        cli::run(command);
    } else {
        gui::run()?;
    }

    Ok(())
}
