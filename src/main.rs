use clap::{Parser, Subcommand};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};
use tracing_tree::HierarchicalLayer;

mod backup;
mod common;
mod elasticsearch;
mod restore;
mod types;

#[derive(Subcommand)]
enum Commands {
    Backup,
    Restore,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Registry::default()
        .with(
            EnvFilter::builder()
                .with_default_directive(Level::INFO.into())
                .from_env_lossy(),
        )
        .with(
            HierarchicalLayer::new(2)
                .with_targets(true)
                .with_bracketed_fields(true),
        )
        .init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Backup => backup::backup().await,
        Commands::Restore => restore::restore().await,
    }
}
