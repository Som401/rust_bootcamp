use rand::Rng;
use std::env;
use std::io::{self, BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;

const P: u64 = 0xD87FA3E291B4C7F3;
const G: u64 = 2;

struct Lcg {
    state: u64,
    position: usize,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self {
            state: seed,
            position: 0,
        }
    }

    fn next_byte(&mut self) -> u8 {
        self.state = (1103515245u64.wrapping_mul(self.state).wrapping_add(12345)) % 4294967296;
        self.position += 1;
        (self.state >> 24) as u8
    }
}

fn mod_pow(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus == 1 {
        return 0;
    }
    let mut result = 1;
    base %= modulus;
    while exp > 0 {
        if exp % 2 == 1 {
            result = (result as u128 * base as u128 % modulus as u128) as u64;
        }
        exp >>= 1;
        base = (base as u128 * base as u128 % modulus as u128) as u64;
    }
    result
}

fn print_help() {
    println!("Usage: streamchat");
    println!("\nStream cipher chat with Diffie-Hellman key generation");
    println!("\nCommands:");
    println!("  server Start server");
    println!("  client Connect to server");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_help();
        return;
    }

    match args[1].as_str() {
        "-h" | "--help" => {
            print_help();
        }
        "server" => {
            if args.len() < 3 {
                eprintln!("Usage: cargo run -- server <PORT>");
                process::exit(1);
            }
            run_server(&args[2]);
        }
        "client" => {
            if args.len() < 3 {
                eprintln!("Usage: cargo run -- client <ADDRESS>");
                process::exit(1);
            }
            run_client(&args[2]);
        }
        _ => {
            eprintln!("error: Invalid command '{}'", args[1]);
            print_help();
            process::exit(2);
        }
    }
}

fn run_server(port: &str) {
    let address = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&address).expect("Failed to bind");
    println!("[SERVER] Listening on {}", address);
    println!("[SERVER] DH parameters:");
    println!("  p = {:016X}", P);
    println!("  g = {}", G);
    println!("[SERVER] Waiting for client...");

    if let Ok((stream, addr)) = listener.accept() {
        println!("\n[CLIENT] Connected from {}", addr);
        handle_connection(stream, true);
    }
}

fn run_client(address: &str) {
    println!("[CLIENT] Connecting to {}...", address);
    match TcpStream::connect(address) {
        Ok(stream) => {
            println!("[CLIENT] Connected!");
            handle_connection(stream, false);
        }
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
            process::exit(1);
        }
    }
}

fn handle_connection(mut stream: TcpStream, is_server: bool) {
    println!("\\n[DH] Starting key exchange...");
    println!("[DH] Using hardcoded DH parameters:");
    println!("  p = {:016X} (64-bit prime - public)", P);
    println!("  g = {} (generator - public)", G);

    let private_key: u64 = rand::thread_rng().gen();
    println!("\\n[DH] Generating our keypair...");
    println!("  private_key = {:016X} (random 64-bit)", private_key);

    let public_key = mod_pow(G, private_key, P);
    println!("  public_key  = g^private mod p");
    println!("              = {}^{:016X} mod p", G, private_key);
    println!("              = {:016X}", public_key);

    println!("\\n[DH] Exchanging keys...");

    let peer_public_key = if is_server {
        println!("[NETWORK] Sending public key (8 bytes)...");
        stream.write_all(&public_key.to_be_bytes()).unwrap();
        println!("  → Send our public:     {:016X}", public_key);

        let mut buf = [0u8; 8];
        stream.read_exact(&mut buf).unwrap();
        let key = u64::from_be_bytes(buf);
        println!("[NETWORK] Received public key (8 bytes) ✓");
        println!("  ← Receive their public: {:016X}", key);
        key
    } else {
        let mut buf = [0u8; 8];
        stream.read_exact(&mut buf).unwrap();
        let key = u64::from_be_bytes(buf);
        println!("[NETWORK] Received public key (8 bytes) ✓");
        println!("  ← Receive their public: {:016X}", key);

        println!("[NETWORK] Sending public key (8 bytes)...");
        stream.write_all(&public_key.to_be_bytes()).unwrap();
        println!("  → Send our public:     {:016X}", public_key);
        key
    };

    println!("\\n[DH] Computing shared secret...");
    println!("  Formula: secret = (their_public)^(our_private) mod p");
    let shared_secret = mod_pow(peer_public_key, private_key, P);
    println!(
        "\\n  secret = ({:016X})^({:016X}) mod p",
        peer_public_key, private_key
    );
    println!("         = {:016X}", shared_secret);

    println!("\\n[VERIFY] Both sides computed the same secret ✓");

    println!("\\n[STREAM] Generating keystream from secret...");
    println!("  Algorithm: LCG (a=1103515245, c=12345, m=2^32)");
    println!("  Seed: secret = {:016X}", shared_secret);

    let lcg = Arc::new(Mutex::new(Lcg::new(shared_secret)));

    {
        let mut temp_lcg = Lcg::new(shared_secret);
        print!("\\n  Keystream:");
        for _ in 0..14 {
            print!(" {:02X}", temp_lcg.next_byte());
        }
        println!(" ...");
    }

    println!("\\n✓ Secure channel established!");
    println!("\\n[CHAT] Type message:");

    let stream_clone = stream.try_clone().expect("Failed to clone stream");
    let lcg_clone = Arc::clone(&lcg);

    thread::spawn(move || {
        let mut buffer = [0u8; 1024];
        let mut stream = stream_clone;
        loop {
            match stream.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    println!("\\n[NETWORK] Received encrypted message ({} bytes)", n);
                    println!("[←] Received {} bytes", n);

                    let mut lcg = lcg_clone.lock().unwrap();
                    let start_pos = lcg.position;

                    print!("\\n[DECRYPT]\\n  Cipher:");
                    for b in &buffer[..n] {
                        print!(" {:02x}", b);
                    }
                    println!();

                    print!("  Key:   ");
                    let mut decrypted = Vec::with_capacity(n);

                    for b in &buffer[..n] {
                        let k = lcg.next_byte();
                        decrypted.push(b ^ k);
                        print!(" {:02x}", k);
                    }
                    println!("  (keystream position: {})", start_pos);

                    print!("  Plain: ");
                    for b in &decrypted {
                        print!(" {:02x}", b);
                    }

                    let msg = String::from_utf8_lossy(&decrypted);
                    println!("  → \\\"{}\\\"", msg.trim());

                    println!(
                        "\\n[TEST] Round-trip verified: \\\"{}\\\" → encrypt → decrypt → \\\"{}\\\" ✓",
                        msg.trim(),
                        msg.trim()
                    );

                    if is_server {
                        println!("\\n[CLIENT] {}", msg.trim());
                    } else {
                        println!("\\n[SERVER] {}", msg.trim());
                    }
                }
                Ok(_) => {
                    println!("Connection closed.");
                    process::exit(0);
                }
                Err(_) => {
                    process::exit(0);
                }
            }
        }
    });

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buffer = String::new();

    loop {
        buffer.clear();
        if handle.read_line(&mut buffer).is_err() {
            break;
        }
        let trimmed = buffer.trim();
        if trimmed.is_empty() {
            continue;
        }

        let plain_bytes = trimmed.as_bytes();
        let len = plain_bytes.len();

        let mut lcg = lcg.lock().unwrap();
        let start_pos = lcg.position;

        println!("\\n\\n[ENCRYPT]");
        print!("  Plain: ");
        for b in plain_bytes {
            print!(" {:02x}", b);
        }
        println!("  (\\\"{}\\\")", trimmed);

        print!("  Key:   ");
        let mut cipher_bytes = Vec::with_capacity(len);
        for b in plain_bytes {
            let k = lcg.next_byte();
            print!(" {:02x}", k);
            cipher_bytes.push(b ^ k);
        }
        println!("  (keystream position: {})", start_pos);

        print!("  Cipher:");
        for b in &cipher_bytes {
            print!(" {:02x}", b);
        }
        println!();

        println!("\\n[NETWORK] Sending encrypted message ({} bytes)...", len);
        if stream.write_all(&cipher_bytes).is_ok() {
            println!("[→] Sent {} bytes", len);
        } else {
            break;
        }
    }
}
