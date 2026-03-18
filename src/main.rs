use clap::{Parser, Subcommand};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};
use tracing_tree::HierarchicalLayer;

use c8_backup::types::StorageMode;
use c8_backup::{create, list, restore};

#[derive(Subcommand)]
enum Commands {
    List,
    Create,
    Restore {
        /// Point-in-time restore target (ISO 8601 timestamp, RDBMS mode only)
        #[arg(long)]
        to: Option<String>,
        /// Explicit backup ID to restore from
        #[arg(long)]
        backup_id: Option<u64>,
    },
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Secondary storage type of the Camunda deployment
    #[arg(long, value_enum, default_value_t = StorageMode::Elasticsearch)]
    storage_mode: StorageMode,
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
        Commands::List => list::list(cli.storage_mode).await,
        Commands::Create => create::create(cli.storage_mode).await,
        Commands::Restore { to, backup_id } => {
            restore::restore(cli.storage_mode, to, backup_id).await
        }
    }
}
