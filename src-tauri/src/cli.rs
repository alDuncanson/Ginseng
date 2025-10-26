use anyhow::Result;
use clap::{Parser, Subcommand};
use ginseng_lib::GinsengCore;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ginseng-cli")]
#[command(about = "Gensing CLI — peer-to-peer file sharing via Iroh", long_about = None)]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    Send {
        #[arg(value_name = "PATH")]
        path: PathBuf,
    },
    Receive {
        #[arg(value_name = "TICKET")]
        ticket: String,

        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },
    Info,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let ginseng = GinsengCore::new().await?;

    match args.command {
        Commands::Send { path } => {
            handle_send(ginseng, path).await?;
        }
        Commands::Receive { ticket, output } => {
            handle_receive(ginseng, ticket, output).await?;
        }
        Commands::Info => {
            handle_info(ginseng).await?;
        }
    }

    Ok(())
}

async fn handle_send(ginseng: GinsengCore, path: PathBuf) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("Path does not exist: {}", path.display());
    }

    let ticket = ginseng.share_file(path.clone()).await?;

    println!("{}", ticket);

    tokio::signal::ctrl_c().await?;

    Ok(())
}

async fn handle_receive(ginseng: GinsengCore, ticket: String, output: PathBuf) -> Result<()> {
    let output_abs = if output.is_absolute() {
        output
    } else {
        std::env::current_dir()?.join(output)
    };

    let download_path = if output_abs.is_dir() || output_abs == std::env::current_dir()?.join(".") {
        if !output_abs.exists() {
            tokio::fs::create_dir_all(&output_abs).await?;
        }
        output_abs.join("downloaded_file")
    } else {
        if let Some(parent) = output_abs.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        output_abs
    };

    ginseng.download_file(ticket, download_path).await?;

    Ok(())
}

async fn handle_info(ginseng: GinsengCore) -> Result<()> {
    ginseng.node_info().await?;

    Ok(())
}
