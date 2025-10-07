use std::path::PathBuf;
use sync_dis_boi::yt_music::YtMusicApi;
use std::io::{self, BufRead};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    
    let args: Vec<String> = std::env::args().collect();
    
    let output_path = if args.len() > 1 {
        Some(PathBuf::from(&args[1]))
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
    
    println!("YouTube Music Browser Authentication Setup");
    println!("===========================================\n");
    println!("This tool will help you set up browser-based authentication for YouTube Music.");
    println!("You need to copy your browser's request headers.\n");
    println!("Instructions:");
    println!("1. Open Firefox or Chrome and go to https://music.youtube.com");
    println!("2. Make sure you're logged in");
    println!("3. Open Developer Tools (F12)");
    println!("4. Go to the Network tab");
    println!("5. Click on any request to music.youtube.com/youtubei/v1/browse");
    println!("6. Right-click on the request and select 'Copy' > 'Copy Request Headers'");
    println!("7. Paste the headers below\n");
    
    if let Some(ref path) = output_path {
        println!("Headers will be saved to: {}\n", path.display());
    }
    
    // Read headers from stdin manually to avoid atty check
    let eof = if cfg!(windows) { "'Enter, Ctrl-Z, Enter'" } else { "Ctrl-D" };
    println!("Paste your headers and press {} to continue:", eof);
    
    let stdin = io::stdin();
    let mut headers_lines = Vec::new();
    
    for line in stdin.lock().lines() {
        match line {
            Ok(l) => headers_lines.push(l),
            Err(_) => break,
        }
    }
    
    if headers_lines.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No headers provided. Please paste the headers from your browser."
        ));
    }
    
    let headers_raw = headers_lines.join("\n");
    let _headers_json = YtMusicApi::setup_browser_from_raw(&headers_raw, output_path.clone())?;
    
    println!("\n✓ Headers saved successfully!");
    
    if let Some(path) = output_path {
        println!("✓ You can now use: --headers {}", path.display());
    }
    
    println!("\nExample usage:");
    println!("  sync_dis_boi yt-music --headers ytmusic_headers.json --owner \"YourName\" \\");
    println!("    spotify --client-id ... --client-secret ... --owner \"YourName\"");
    
    Ok(())
}
