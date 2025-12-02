use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::process;

fn print_help() {
    println!("Usage: hextool [OPTIONS]");
    println!("\\nRead and write binary files in hexadecimal");
    println!("\\nOptions:");
    println!("  -f, --file    Target file");
    println!("  -r, --read    Read mode (display hex)");
    println!("  -w, --write   Write mode (hex string to write)");
    println!("  -o, --offset  Offset in bytes (decimal or 0x hex)");
    println!("  -s, --size    Number of bytes to read");
    println!("  -h, --help    Print help");
}

fn parse_offset(s: &str) -> u64 {
    if let Some(stripped) = s.strip_prefix("0x") {
        u64::from_str_radix(stripped, 16).unwrap_or(0)
    } else {
        s.parse().unwrap_or(0)
    }
}

fn hex_string_to_bytes(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("Hex string must have an even length".to_string());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "Invalid hex character".to_string())
        })
        .collect()
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut file_path: Option<String> = None;
    let mut read_mode = false;
    let mut write_data: Option<String> = None;
    let mut offset: u64 = 0;
    let mut size: Option<usize> = None;

    let mut i = 1;
    if args.len() == 1 {
        print_help();
        return;
    }

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-f" | "--file" => {
                if i + 1 < args.len() {
                    file_path = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "-r" | "--read" => {
                read_mode = true;
                i += 1;
            }
            "-w" | "--write" => {
                if i + 1 < args.len() {
                    write_data = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "-o" | "--offset" => {
                if i + 1 < args.len() {
                    offset = parse_offset(&args[i + 1]);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "-s" | "--size" => {
                if i + 1 < args.len() {
                    size = args[i + 1].parse().ok();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                eprintln!("Error: Invalid option {}", args[i]);
                process::exit(2);
            }
        }
    }

    let path = match file_path {
        Some(p) => p,
        None => {
            eprintln!("Error: No file specified");
            process::exit(1);
        }
    };

    if let Some(hex_str) = write_data {
        let bytes_to_write = match hex_string_to_bytes(&hex_str) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Error parsing hex string: {}", e);
                process::exit(1);
            }
        };

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .expect("Failed to open file for writing");

        file.seek(SeekFrom::Start(offset)).expect("Failed to seek");
        file.write_all(&bytes_to_write)
            .expect("Failed to write bytes");

        println!(
            "Writing {} bytes at offset 0x{:08x}",
            bytes_to_write.len(),
            offset
        );

        print!("Hex:");
        for b in &bytes_to_write {
            print!(" {:02x}", b);
        }
        println!();

        print!("ASCII: ");
        for b in &bytes_to_write {
            if *b >= 0x20 && *b <= 0x7E {
                print!("{}", *b as char);
            } else {
                print!(".");
            }
        }
        println!("\\n✓ Successfully written");
        return;
    }

    if read_mode {
        let mut file = File::open(&path).expect("File not found");

        let file_len = file.metadata().unwrap().len();
        if offset > file_len {
            return;
        }

        file.seek(SeekFrom::Start(offset)).expect("Failed to seek");

        let bytes_to_read = match size {
            Some(s) => s,
            None => (file_len - offset) as usize,
        };

        let mut buffer = vec![0; bytes_to_read];
        let bytes_read = file.read(&mut buffer).expect("Failed to read file");

        for (line_idx, chunk) in buffer[..bytes_read].chunks(16).enumerate() {
            let current_offset = offset + (line_idx * 16) as u64;

            print!("{:08x}:", current_offset);

            for byte in chunk {
                print!(" {:02x}", byte);
            }

            if chunk.len() < 16 {
                for _ in 0..(16 - chunk.len()) {
                    print!("   ");
                }
            }

            print!(" |");

            for byte in chunk {
                if *byte >= 0x20 && *byte <= 0x7E {
                    print!("{}", *byte as char);
                } else {
                    print!(".");
                }
            }
            println!("|");
        }
    }
}
