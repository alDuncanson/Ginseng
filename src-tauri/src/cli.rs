use anyhow::Result;
use clap::{Parser, Subcommand};
use ginseng_lib::{
    core::{FileInfo, ShareMetadata, ShareType},
    GinsengCore,
};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "ginseng-cli")]
#[command(about = "Ginseng CLI — peer-to-peer file sharing via Iroh", long_about = None)]
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
        #[arg(value_name = "PATH", required = true)]
        paths: Vec<PathBuf>,

        #[arg(long)]
        files_only: bool,
    },
    Receive {
        #[arg(value_name = "TICKET")]
        ticket: String,
    },
    Info,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Err(error) = run(args).await {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
}

async fn run(args: Args) -> Result<()> {
    let ginseng = GinsengCore::new().await?;

    match args.command {
        Commands::Send { paths, files_only } => handle_send(ginseng, paths, files_only).await,
        Commands::Receive { ticket } => handle_receive(ginseng, ticket).await,
        Commands::Info => handle_info(ginseng).await,
    }
}

async fn handle_send(ginseng: GinsengCore, paths: Vec<PathBuf>, files_only: bool) -> Result<()> {
    validate_paths_exist(&paths)?;

    if files_only {
        validate_paths_are_files(&paths)?;
    }

    display_sharing_summary(&paths);

    println!("\nGenerating share ticket...");
    let ticket = ginseng.share_files(paths).await?;

    display_share_ticket(&ticket);

    tokio::signal::ctrl_c().await?;
    println!("\nStopped sharing.");

    Ok(())
}

async fn handle_receive(ginseng: GinsengCore, ticket: String) -> Result<()> {
    println!("🔄 Downloading files from ticket...");

    let (metadata, download_path) = ginseng.download_files(ticket).await?;

    display_download_summary(&metadata, &download_path);

    Ok(())
}

async fn handle_info(ginseng: GinsengCore) -> Result<()> {
    let info = ginseng.node_info().await?;
    println!("🔧 Node Information:");
    println!("{}", info);
    Ok(())
}

fn validate_paths_exist(paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        if !path.exists() {
            anyhow::bail!("Path does not exist: {}", path.display());
        }
    }
    Ok(())
}

fn validate_paths_are_files(paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        if !path.is_file() {
            anyhow::bail!(
                "Path is not a file (use without --files-only to share directories): {}",
                path.display()
            );
        }
    }
    Ok(())
}

fn display_sharing_summary(paths: &[PathBuf]) {
    if paths.len() == 1 {
        display_single_path_summary(&paths[0]);
    } else {
        display_multiple_paths_summary(paths);
    }
}

fn display_single_path_summary(path: &PathBuf) {
    if path.is_file() {
        println!("Sharing file: {}", path.display());
    } else if path.is_dir() {
        println!("Sharing directory: {}", path.display());
        if let Ok(summary) = calculate_directory_summary(path) {
            println!(
                "  Contains {} files, total size: {}",
                summary.file_count,
                format_file_size(summary.total_size)
            );
        }
    }
}

fn display_multiple_paths_summary(paths: &[PathBuf]) {
    println!("Sharing {} items:", paths.len());
    for path in paths {
        let icon = if path.is_file() { "📄" } else { "📁" };
        println!("  {} {}", icon, path.display());
    }
}

fn display_share_ticket(ticket: &str) {
    println!("\n🎫 Share Ticket:");
    println!("{}", ticket);
    println!("\nShare this ticket with the recipient. Press Ctrl+C to stop sharing.");
}

fn display_download_summary(metadata: &ShareMetadata, download_path: &Path) {
    println!("✅ Successfully downloaded {} files!", metadata.files.len());
    println!("📁 Location: {}", download_path.display());

    display_share_type_info(&metadata.share_type);
    println!("📊 Total size: {}", format_file_size(metadata.total_size));

    display_file_listing(&metadata.files);
}

fn display_share_type_info(share_type: &ShareType) {
    let type_description = match share_type {
        ShareType::SingleFile => "Single file".to_string(),
        ShareType::MultipleFiles => "Multiple files".to_string(),
        ShareType::Directory { name } => format!("Directory ({})", name),
    };
    println!("📄 Type: {}", type_description);
}

fn display_file_listing(files: &[FileInfo]) {
    if files.len() <= 10 {
        println!("\n📋 Files:");
        for file_info in files {
            println!(
                "  • {} ({})",
                file_info.relative_path,
                format_file_size(file_info.size)
            );
        }
    } else {
        println!("\n📋 Files (showing first 10 of {}):", files.len());
        for file_info in files.iter().take(10) {
            println!(
                "  • {} ({})",
                file_info.relative_path,
                format_file_size(file_info.size)
            );
        }
        println!("  ... and {} more files", files.len() - 10);
    }
}

struct DirectorySummary {
    file_count: usize,
    total_size: u64,
}

fn calculate_directory_summary(dir: &PathBuf) -> Result<DirectorySummary> {
    use walkdir::WalkDir;

    let mut file_count = 0;
    let mut total_size = 0u64;

    for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        if entry.path().is_file() {
            file_count += 1;
            if let Ok(metadata) = std::fs::metadata(entry.path()) {
                total_size += metadata.len();
            }
        }
    }

    Ok(DirectorySummary {
        file_count,
        total_size,
    })
}

fn format_file_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    let k = 1024u64;
    let sizes = ["B", "KB", "MB", "GB", "TB"];
    let i = ((bytes as f64).log(k as f64).floor() as usize).min(sizes.len() - 1);
    let size = bytes as f64 / k.pow(i as u32) as f64;
    format!("{:.2} {}", size, sizes[i])
}
