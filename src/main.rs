mod assistant;
mod cli;
mod config;
mod inference;
mod models;
mod node;
mod payment;
mod server;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "mimona",
    about = "Run AI models locally with blockchain payments",
    version = env!("CARGO_PKG_VERSION"),
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        model: String,
        #[arg(long)]
        system: Option<String>,
        #[arg(long, default_value = "0.7")]
        temperature: f32,
        #[arg(long, default_value = "2048")]
        max_tokens: u32,
    },
    Pull {
        model: String,
        #[arg(long)]
        force: bool,
    },
    List,
    Show { model: String },
    Rm { model: String },
    Serve {
        #[arg(long, default_value = "11435")]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
    Node {
        #[command(subcommand)]
        action: NodeCommands,
    },
    Wallet {
        #[command(subcommand)]
        action: WalletCommands,
    },
}

#[derive(Subcommand)]
enum NodeCommands {
    Start,
    Stop,
    Status,
    Earnings,
}

#[derive(Subcommand)]
enum WalletCommands {
    Create,
    Balance,
    Address,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.verbose { "debug" } else { "warn" };
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .with_target(false)
        .init();

    match cli.command {
        // No subcommand → launch interactive assistant
        None => {
            assistant::run_assistant().await?;
        }

        Some(Commands::Run { model, system, temperature, max_tokens }) => {
            cli::run::handle(model, system, temperature, max_tokens).await?;
        }
        Some(Commands::Pull { model, force }) => {
            cli::pull::handle(model, force).await?;
        }
        Some(Commands::List) => {
            cli::list::handle().await?;
        }
        Some(Commands::Show { model }) => {
            cli::show::handle(model).await?;
        }
        Some(Commands::Rm { model }) => {
            cli::rm::handle(model).await?;
        }
        Some(Commands::Serve { port, host }) => {
            cli::serve::handle(host, port).await?;
        }
        Some(Commands::Node { action }) => match action {
            NodeCommands::Start  => cli::node::start().await?,
            NodeCommands::Stop   => cli::node::stop().await?,
            NodeCommands::Status => cli::node::status().await?,
            NodeCommands::Earnings => cli::node::earnings().await?,
        },
        Some(Commands::Wallet { action }) => match action {
            WalletCommands::Create  => cli::wallet::create().await?,
            WalletCommands::Balance => cli::wallet::balance().await?,
            WalletCommands::Address => cli::wallet::address().await?,
        },
    }

    Ok(())
}
