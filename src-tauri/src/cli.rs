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

    let ticket = ginseng.share_files(vec![path.clone()]).await?;

    println!("{}", ticket);

    tokio::signal::ctrl_c().await?;

    Ok(())
}

async fn handle_receive(ginseng: GinsengCore, ticket: String, _output: PathBuf) -> Result<()> {
    let (metadata, actual_download_path) = ginseng.download_files(ticket).await?;

    println!(
        "Downloaded {} files to: {}",
        metadata.files.len(),
        actual_download_path.display()
    );
    for file_info in &metadata.files {
        println!(
            "  - {} ({})",
            file_info.relative_path,
            format_size(file_info.size)
        );
    }

    Ok(())
}

async fn handle_info(ginseng: GinsengCore) -> Result<()> {
    let info = ginseng.node_info().await?;
    println!("{}", info);

    Ok(())
}

fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    let k = 1024u64;
    let sizes = ["B", "KB", "MB", "GB"];
    let i = (bytes as f64).log(k as f64).floor() as usize;
    let size = bytes as f64 / k.pow(i as u32) as f64;
    format!("{:.2} {}", size, sizes[i])
}
