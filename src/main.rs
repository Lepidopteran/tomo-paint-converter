use clap::Parser;
use color_eyre::eyre::Result;

mod cli;
mod gui;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct App {
    #[command(subcommand)]
    command: Option<cli::Command>,
}

fn main() -> Result<()> {
    let App { command } = App::parse();

    if let Some(command) = command {
        cli::run(command);
    } else {
        gui::run()?;
    }

    Ok(())
}
