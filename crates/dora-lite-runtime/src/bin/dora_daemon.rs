use std::path::PathBuf;
use clap::{Parser, Subcommand};
use dora_lite_core::config::DataflowDescriptor;
use dora_lite_runtime::coordinator::Coordinator;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "dora-daemon")]
#[command(about = "dora-lite: High-performance robotics dataflow runtime", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a declarative dataflow pipeline
    Run {
        /// Path to the dataflow YAML configuration file
        #[arg(short, long)]
        dataflow: PathBuf,
    },
    /// Validate dataflow YAML syntax and port connections
    Validate {
        /// Path to the dataflow YAML configuration file
        #[arg(short, long)]
        dataflow: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { dataflow } => {
            info!("Validating dataflow descriptor at {:?}", dataflow);
            let descriptor = DataflowDescriptor::from_file(&dataflow)?;
            info!(
                "Validation passed! Pipeline '{}' has {} nodes.",
                descriptor.id,
                descriptor.nodes.len()
            );
        }
        Commands::Run { dataflow } => {
            info!("Loading dataflow descriptor from {:?}", dataflow);
            let descriptor = DataflowDescriptor::from_file(&dataflow)?;
            info!(
                "Starting pipeline '{}' with {} configured nodes",
                descriptor.id,
                descriptor.nodes.len()
            );

            let coordinator = Coordinator::new(descriptor);

            // Spawn coordinator
            if let Err(err) = coordinator.run().await {
                error!("Coordinator execution error: {:?}", err);
                return Err(err.into());
            }

            // Wait for Ctrl-C termination
            tokio::signal::ctrl_c().await?;
            info!("Received shutdown signal. Stopping dataflow...");
            coordinator.shutdown().await;
        }
    }

    Ok(())
}
