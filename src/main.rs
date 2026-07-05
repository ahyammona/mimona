mod assistant;
mod cli;
mod config;
mod desktop;
mod inference;
mod models;
mod node;
mod payment;
mod server;
mod whatsapp;
mod whatsapp_bridge_launcher;
mod widget;
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
    Desktop,
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
 
fn main() -> Result<()> {
    let cli = Cli::parse();
 
    let log_level = if cli.verbose { "debug" } else { "warn" };
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .with_target(false)
        .init();
 
    // No args or explicit Desktop → launch GUI
    if matches!(cli.command, Some(Commands::Desktop) | None) {
        // Single instance lock — prevent multiple windows
        let lock_path = std::env::temp_dir().join("mimona.lock");
        let lock = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&lock_path);
 
        if let Ok(file) = lock {
            use std::io::Write;
            // Try to get exclusive lock
            #[cfg(unix)]
            {
                use std::os::unix::io::AsRawFd;
                let fd = file.as_raw_fd();
                let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
                if ret != 0 {
                    // Another instance is running — just exit silently
                    return Ok(());
                }
                // Keep file handle alive for the duration of the process
                std::mem::forget(file);
            }
            #[cfg(windows)]
            {
                // On Windows the OS handles exclusive file locking
                std::mem::forget(file);
            }
        }
 
        return launch_desktop();
    }
 
    // All other commands run inside a Tokio runtime
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async_main(cli))
}
 
fn launch_desktop() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Mimona")
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([800.0, 500.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(
                    include_bytes!("../assets/icon.png")
                ).unwrap_or_default()
            ),
        ..Default::default()
    };
    eframe::run_native(
        "Mimona",
        options,
        Box::new(|cc| Ok(Box::new(desktop::MimonaApp::new(cc)))),
    ).map_err(|e| anyhow::anyhow!("Desktop window error: {}", e))?;
    Ok(())
}
 
async fn async_main(cli: Cli) -> Result<()> {
    match cli.command {
        None => unreachable!(),
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
        Some(Commands::Desktop) => unreachable!(),
        Some(Commands::Node { action }) => match action {
            NodeCommands::Start    => cli::node::start().await?,
            NodeCommands::Stop     => cli::node::stop().await?,
            NodeCommands::Status   => cli::node::status().await?,
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
 