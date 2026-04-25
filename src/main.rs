use color_eyre::eyre::Result;

mod app;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_PKG_NAME")).into()),
        )
        .with(fmt::layer())
        .init();

    color_eyre::install()?;

    app::run()
}
