use clap::Parser;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "hextool")]
#[command(about = "Read and write binary files in hexadecimal", long_about = None)]
struct Cli {
    #[arg(short, long, value_name = "file")]
    file: Option<PathBuf>,

    #[arg(short, long)]
    read: bool,

    #[arg(short, long, value_name = "hex")]
    write: Option<String>,

    #[arg(short, long, value_name = "off")]
    offset: Option<String>,

    #[arg(short, long, value_name = "n")]
    size: Option<usize>,
}

fn parse_offset(s: &str) -> Result<u64, String> {
    if s.starts_with("0x") || s.starts_with("0X") {
        u64::from_str_radix(&s[2..], 16).map_err(|e| format!("Invalid hex offset: {}", e))
    } else {
        s.parse::<u64>()
            .map_err(|e| format!("Invalid decimal offset: {}", e))
    }
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    let hex = hex.trim();
    if !hex.len().is_multiple_of(2) {
        return Err("Hex string must have even length".to_string());
    }

    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| format!("Invalid hex at position {}: {}", i, e))
        })
        .collect()
}

fn print_hex_dump(data: &[u8], start_offset: u64) {
    for (i, chunk) in data.chunks(16).enumerate() {
        let offset = start_offset + (i * 16) as u64;
        print!("{:08x}: ", offset);

        for (j, byte) in chunk.iter().enumerate() {
            print!("{:02x} ", byte);
            if j == 7 {
                print!(" ");
            }
        }

        if chunk.len() < 16 {
            for j in chunk.len()..16 {
                print!("   ");
                if j == 7 {
                    print!(" ");
                }
            }
        }

        print!(" |");
        for byte in chunk {
            let c = if (0x20..=0x7E).contains(byte) {
                *byte as char
            } else {
                '.'
            };
            print!("{}", c);
        }
        println!("|");
    }
}

fn read_mode(file_path: &PathBuf, offset: u64, size: usize) -> io::Result<()> {
    let mut file = File::open(file_path)?;
    file.seek(SeekFrom::Start(offset))?;

    let mut buffer = vec![0u8; size];
    let bytes_read = file.read(&mut buffer)?;
    buffer.truncate(bytes_read);

    print_hex_dump(&buffer, offset);
    Ok(())
}

fn write_mode(file_path: &PathBuf, hex_string: &str, offset: u64) -> io::Result<()> {
    let bytes =
        hex_to_bytes(hex_string).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(file_path)?;

    file.seek(SeekFrom::Start(offset))?;
    file.write_all(&bytes)?;

    println!("Writing {} bytes at offset 0x{:08x}", bytes.len(), offset);
    print!("  Hex:   ");
    for byte in &bytes {
        print!("{:02x} ", byte);
    }
    println!();
    print!("  ASCII: ");
    for byte in &bytes {
        let c = if (0x20..=0x7E).contains(byte) {
            *byte as char
        } else {
            '.'
        };
        print!("{}", c);
    }
    println!();
    println!("✓ Successfully written");

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    let file_path = match cli.file {
        Some(path) => path,
        None => {
            eprintln!("Error: --file is required");
            std::process::exit(1);
        }
    };

    let offset = match cli.offset {
        Some(ref s) => match parse_offset(s) {
            Ok(off) => off,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },
        None => 0,
    };

    if cli.read {
        let size = cli.size.unwrap_or(256);
        if let Err(e) = read_mode(&file_path, offset, size) {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    } else if let Some(hex_string) = cli.write {
        if let Err(e) = write_mode(&file_path, &hex_string, offset) {
            eprintln!("Error writing file: {}", e);
            std::process::exit(1);
        }
    } else {
        eprintln!("Error: Either --read or --write must be specified");
        std::process::exit(1);
    }
}
