//! Main entrypoint.

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    tracing::info!("Hello, world!");

    Ok(())
}
