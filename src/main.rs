use anyhow::{Context, Result};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use rfd::FileDialog;
use serde_json::{from_str, to_string_pretty, Value};
use std::{
    env,
    fs,
    io::{self, Read, Write},
    path::PathBuf,
};

fn main() -> Result<()> {
    let result = run();
    
    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }
    
    wait_for_enter();
    Ok(())
}

fn wait_for_enter() {
    let mut input = String::new();
    println!("\nPress Enter to exit...");
    let _ = io::stdin().read_line(&mut input);
}

fn run() -> Result<()> {
    let input_path: PathBuf = {
        let mut args = env::args().skip(1);
        if let Some(arg) = args.next() {
            let p = PathBuf::from(arg);
            if !p.exists() {
                return Err(anyhow::anyhow!(
                    "Input file does not exist: {}",
                    p.display()
                ));
            }
            p
        } else {
            println!("Select the file to be processed...");
            let picked = open_file_dialog().context("Failed to select file")?;
            if picked.to_str().is_none() {
                println!("File is not selected");
                return Ok(());
            }
            picked
        }
    };

    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());
    match ext.as_deref() {
        Some("wbox") | Some("wbax") | Some("json") => {
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported file extension: {:?}. Allowed extensions are .wbox, .wbax, .json",
                input_path.extension().and_then(|e| e.to_str())
            ));
        }
    }

    let input_size = fs::metadata(&input_path)
        .with_context(|| format!("Failed to get the file size {}", input_path.display()))?
        .len();
    println!("\n▌ File selected: {}", input_path.display());
    println!("▌ File size: {} bytes", input_size);

    let is_compressed = is_file_compressed(&input_path)?;
    println!(
        "▌ File {} compressed",
        if is_compressed { "is" } else { "is not" }
    );

    let default_extension = if is_compressed { "json" } else { "wbox" };
    let suggested_name = format!(
        "{}.{}",
        input_path.file_stem().unwrap().to_str().unwrap(),
        default_extension
    );

    println!("\nSpecify the path to save the file...");
    let output_path = save_file_dialog(&suggested_name).context("Failed to save file")?;
    if output_path.to_str().is_none() {
        println!("▌ File is not saved");
        return Ok(());
    }

    if is_compressed {
        let compressed_data = fs::read(&input_path)?;
        let decompressed_data = decompress(&compressed_data)?;
        let formatted_json = format_json(&decompressed_data);

        fs::write(&output_path, formatted_json)
            .with_context(|| format!("File writing error in {}", output_path.display()))?;
        
        let output_size = fs::metadata(&output_path)
            .with_context(|| format!("Failed to verify file size {}", output_path.display()))?
            .len();
        
        println!("\n▌ File has been successfully decompressed!");
        println!("▌ Original size: {} bytes", input_size);
        println!("▌ Size after decompressing: {} bytes", output_size);
        println!("▌ The result is saved in: {}", output_path.display());
    } else {
        let text = fs::read_to_string(&input_path)?;
        let compressed_data = compress(&text)?;

        fs::write(&output_path, compressed_data)
            .with_context(|| format!("File writing error in {}", output_path.display()))?;

        let output_size = fs::metadata(&output_path)
            .with_context(|| format!("Failed to verify file size {}", output_path.display()))?
            .len();
        
        println!("\n▌ File has been successfully compressed!");
        println!("▌ Original size: {} bytes", input_size);
        println!("▌ Size after compressing: {} bytes", output_size);
        println!("▌ The result is saved in: {}", output_path.display());
    }

    Ok(())
}


fn open_file_dialog() -> Option<PathBuf> {
    FileDialog::new()
        .add_filter("Files", &["wbox", "wbax", "json"])
        .pick_file()
}

fn save_file_dialog(suggested_name: &str) -> Option<PathBuf> {
    FileDialog::new()
        .set_file_name(suggested_name)
        .save_file()
}

fn is_file_compressed(path: &PathBuf) -> Result<bool> {
    let data = fs::read(path)
        .with_context(|| format!("File reading error {}", path.display()))?;
    
    Ok(decompress(&data).is_ok())
}

fn decompress(data: &[u8]) -> Result<String> {
    let mut decoder = ZlibDecoder::new(data);
    let mut result = String::new();
    decoder.read_to_string(&mut result)?;
    Ok(result)
}

fn compress(text: &str) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(text.as_bytes())?;
    Ok(encoder.finish()?)
}

fn format_json(json_str: &str) -> String {
    match from_str::<Value>(json_str) {
        Ok(parsed) => match to_string_pretty(&parsed) {
            Ok(pretty) => pretty,
            Err(_) => json_str.to_string(),
        },
        Err(_) => json_str.to_string(),
    }
}