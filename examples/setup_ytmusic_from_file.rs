use std::path::PathBuf;
use sync_dis_boi::yt_music::YtMusicApi;

/// Non-interactive setup example for Docker/CI environments
/// 
/// Usage:
///   cargo run --example setup_ytmusic_from_file <input_raw_headers.txt> [output_headers.json]
/// 
/// This reads raw browser headers from a text file and converts them to JSON format.
/// Perfect for Docker containers where you can mount a headers file.

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <input_raw_headers.txt> [output_headers.json]", args[0]);
        eprintln!();
        eprintln!("Non-interactive browser authentication setup for YouTube Music");
        eprintln!();
        eprintln!("Arguments:");
        eprintln!("  input_raw_headers.txt   - File containing raw browser headers (required)");
        eprintln!("  output_headers.json     - Output JSON file (optional, defaults to config dir)");
        eprintln!();
        eprintln!("Example raw headers file format:");
        eprintln!("  cookie: YOUR_COOKIE_VALUE");
        eprintln!("  authorization: YOUR_AUTH_VALUE");
        eprintln!("  x-goog-authuser: 0");
        eprintln!();
        eprintln!("Docker Usage:");
        eprintln!("  1. Create raw_headers.txt on your host");
        eprintln!("  2. docker run -v ./raw_headers.txt:/app/raw_headers.txt \\");
        eprintln!("       sync_dis_boi setup-ytmusic-from-file /app/raw_headers.txt /app/headers.json");
        std::process::exit(1);
    }
    
    let input_file = PathBuf::from(&args[1]);
    
    if !input_file.exists() {
        return Err(color_eyre::eyre::eyre!("Input file not found: {}", input_file.display()));
    }
    
    let output_file = if args.len() > 2 {
        Some(PathBuf::from(&args[2]))
    } else {
        // Default to config directory
        let config_dir = dirs::config_dir()
            .ok_or_else(|| color_eyre::eyre::eyre!("couldn't find system config dir"))?
            .join("SyncDisBoi");
        
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir)?;
        }
        
        Some(config_dir.join("ytmusic_headers.json"))
    };
    
    println!("YouTube Music Browser Authentication Setup (Non-Interactive)");
    println!("=============================================================\n");
    println!("Reading raw headers from: {}", input_file.display());
    
    let _headers_json = YtMusicApi::setup_browser_from_file(&input_file, output_file.clone())?;
    
    println!("\n✓ Headers processed successfully!");
    
    if let Some(ref path) = output_file {
        println!("✓ Saved to: {}", path.display());
        println!("\nYou can now use: --headers {}", path.display());
    }
    
    let output_path_str = output_file
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "ytmusic_headers.json".to_string());

    println!("\nExample usage:");
    println!("  sync_dis_boi yt-music --headers {} --owner \"YourName\" \\", 
             output_path_str);
    println!("    spotify --client-id ... --client-secret ... --owner \"YourName\"");
    
    println!("\nDocker usage:");
    println!("  docker run -v {}:/app/ytmusic_headers.json \\",
             output_path_str);
    println!("    -v ~/.config/SyncDisBoi:/root/.config/SyncDisBoi \\");
    println!("    ghcr.io/kamdzy/sync_dis_boi:latest \\");
    println!("    yt-music --headers /app/ytmusic_headers.json --owner \"YourName\" \\");
    println!("    spotify --client-id ... --client-secret ... --owner \"YourName\"");
    
    Ok(())
}
