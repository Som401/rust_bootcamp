use clap::{Parser, Subcommand};
use rand::Rng;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

const P: u64 = 0xD87FA3E291B4C7F3;
const G: u64 = 2; 

const LCG_A: u32 = 1103515245;
const LCG_C: u32 = 12345;

#[derive(Parser)]
#[command(name = "streamchat")]
#[command(about = "Stream cipher chat with Diffie-Hellman key generation", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Server { port: u16 },
    Client { address: String },
}

fn mod_exp(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus == 1 {
        return 0;
    }

    let mut result = 1u128;
    let modulus_128 = modulus as u128;

    base %= modulus;

    while exp > 0 {
        if exp % 2 == 1 {
            result = (result * (base as u128)) % modulus_128;
        }
        exp >>= 1;
        base = ((base as u128 * base as u128) % modulus_128) as u64;
    }

    result as u64
}

fn generate_keypair() -> (u64, u64) {
    let mut rng = rand::thread_rng();
    let private_key: u64 = rng.gen();
    let public_key = mod_exp(G, private_key, P);
    (private_key, public_key)
}

fn compute_shared_secret(their_public: u64, our_private: u64) -> u64 {
    mod_exp(their_public, our_private, P)
}

struct LcgKeystream {
    state: u32,
}

impl LcgKeystream {
    fn new(seed: u64) -> Self {
        LcgKeystream {
            state: (seed & 0xFFFFFFFF) as u32,
        }
    }

    fn next_byte(&mut self) -> u8 {
        self.state = LCG_A.wrapping_mul(self.state).wrapping_add(LCG_C);
        (self.state >> 24) as u8
    }

    fn get_bytes(&mut self, count: usize) -> Vec<u8> {
        (0..count).map(|_| self.next_byte()).collect()
    }
}

fn format_hex_u64(value: u64) -> String {
    format!(
        "{:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
        (value >> 56) & 0xFF,
        (value >> 48) & 0xFF,
        (value >> 40) & 0xFF,
        (value >> 32) & 0xFF,
        (value >> 24) & 0xFF,
        (value >> 16) & 0xFF,
        (value >> 8) & 0xFF,
        value & 0xFF
    )
}

fn format_hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn send_message(stream: &mut TcpStream, data: &[u8]) -> io::Result<()> {
    let len = data.len() as u32;
    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(data)?;
    stream.flush()?;
    Ok(())
}

fn receive_message(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes) as usize;

    let mut buffer = vec![0u8; len];
    stream.read_exact(&mut buffer)?;
    Ok(buffer)
}

fn encrypt(plaintext: &[u8], keystream: &mut LcgKeystream) -> Vec<u8> {
    let key_bytes = keystream.get_bytes(plaintext.len());
    plaintext
        .iter()
        .zip(key_bytes.iter())
        .map(|(p, k)| p ^ k)
        .collect()
}

fn decrypt(ciphertext: &[u8], keystream: &mut LcgKeystream) -> Vec<u8> {
    encrypt(ciphertext, keystream)
}

fn run_server(port: u16) -> io::Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
    println!("\n[SERVER] Listening on 0.0.0.0:{}", port);
    println!("[SERVER] Waiting for client...\n");
    let (mut stream, addr) = listener.accept()?;
    println!("[CLIENT] Connected from {}\n", addr);
    println!("[DH] Starting key exchange...");
    println!("[DH] Using hardcoded DH parameters:");
    println!("  p = {} (64-bit prime - public)", format_hex_u64(P));
    println!("  g = {} (generator - public)\n", G);
    println!("[DH] Generating our keypair...");
    let (our_private, our_public) = generate_keypair();
    println!("  private_key = {:016X} (random 64-bit)", our_private);
    println!("  public_key  = g^private mod p");
    println!("              = {}^{:016X} mod p", G, our_private);
    println!("              = {:016X}\n", our_public);
    println!("[DH] Exchanging keys...");
    println!("[NETWORK] Sending public key (8 bytes)...");
    println!("  → Send our public:     {:016X}", our_public);
    send_message(&mut stream, &our_public.to_be_bytes())?;
    println!("[NETWORK] Received public key (8 bytes) ✓");
    let their_public_bytes = receive_message(&mut stream)?;
    let their_public = u64::from_be_bytes(their_public_bytes.try_into().unwrap());
    println!("  ← Receive their public: {:016X}\n", their_public);
    println!("[DH] Computing shared secret...");
    println!("  Formula: secret = (their_public)^(our_private) mod p\n");
    let shared_secret = compute_shared_secret(their_public, our_private);
    println!(
        "  secret = ({:016X})^({:016X}) mod p",
        their_public, our_private
    );
    println!("         = {:016X}\n", shared_secret);
    println!("[VERIFY] Both sides computed the same secret ✓\n");
    println!("[STREAM] Generating keystream from secret...");
    println!("  Algorithm: LCG (a={}, c={}, m=2^32)", LCG_A, LCG_C);
    println!("  Seed: secret = {:016X}\n", shared_secret);
    let mut keystream = LcgKeystream::new(shared_secret);
    let preview: Vec<u8> = (0..14).map(|_| keystream.next_byte()).collect();
    println!("  Keystream: {} ...\n", format_hex_bytes(&preview));
    let mut send_keystream = LcgKeystream::new(shared_secret);
    let mut recv_keystream = LcgKeystream::new(shared_secret);
    println!("✓ Secure channel established!\n");
    let stdin = io::stdin();
    let mut stdin_reader = BufReader::new(stdin);

    loop {
        println!("[CHAT] Type message:");
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        stdin_reader.read_line(&mut input)?;
        let message = input.trim();

        if message.is_empty() {
            continue;
        }

        if message == "quit" || message == "exit" {
            break;
        }

        println!("\n[ENCRYPT]");
        let plaintext = message.as_bytes();
        println!(
            "  Plain:  {}  (\"{}\")",
            format_hex_bytes(plaintext),
            message
        );

        let ciphertext = encrypt(plaintext, &mut send_keystream);
        let key_preview: Vec<u8> = {
            let mut temp_ks = LcgKeystream::new(shared_secret);
            temp_ks.get_bytes(plaintext.len())
        };
        println!(
            "  Key:    {}  (keystream position: 0)",
            format_hex_bytes(&key_preview)
        );
        println!("  Cipher: {}\n", format_hex_bytes(&ciphertext));

        println!(
            "[NETWORK] Sending encrypted message ({} bytes)...",
            ciphertext.len()
        );
        send_message(&mut stream, &ciphertext)?;
        println!("[→] Sent {} bytes\n", ciphertext.len());

        // Receive response
        println!("[NETWORK] Receiving encrypted message...");
        let received_cipher = receive_message(&mut stream)?;
        println!("[←] Received {} bytes\n", received_cipher.len());

        println!("[DECRYPT]");
        println!("  Cipher: {}", format_hex_bytes(&received_cipher));
        let decrypted = decrypt(&received_cipher, &mut recv_keystream);
        let key_used: Vec<u8> = {
            let mut temp_ks = LcgKeystream::new(shared_secret);
            temp_ks.get_bytes(plaintext.len());
            temp_ks.get_bytes(received_cipher.len())
        };
        println!(
            "  Key:    {}  (keystream position: {})",
            format_hex_bytes(&key_used),
            plaintext.len()
        );
        println!(
            "  Plain:  {}  → \"{}\"\n",
            format_hex_bytes(&decrypted),
            String::from_utf8_lossy(&decrypted)
        );

        println!("[CLIENT] {}\n", String::from_utf8_lossy(&decrypted));
    }

    Ok(())
}

fn run_client(address: &str) -> io::Result<()> {
    println!("\n[CLIENT] Connecting to {}...", address);
    let mut stream = TcpStream::connect(address)?;
    println!("[CLIENT] Connected!\n");
    println!("[DH] Starting key exchange...");
    println!("[DH] Using hardcoded DH parameters:");
    println!("  p = {} (64-bit prime - public)", format_hex_u64(P));
    println!("  g = {} (generator - public)\n", G);
    println!("[DH] Generating our keypair...");
    let (our_private, our_public) = generate_keypair();
    println!("  private_key = {:016X} (random 64-bit)", our_private);
    println!("  public_key  = g^private mod p");
    println!("              = {}^{:016X} mod p", G, our_private);
    println!("              = {:016X}\n", our_public);
    println!("[DH] Exchanging keys...");
    println!("[NETWORK] Received public key (8 bytes) ✓");
    let their_public_bytes = receive_message(&mut stream)?;
    let their_public = u64::from_be_bytes(their_public_bytes.try_into().unwrap());
    println!("  ← Receive their public: {:016X}", their_public);
    println!("[NETWORK] Sending public key (8 bytes)...");
    println!("  → Send our public:     {:016X}\n", our_public);
    send_message(&mut stream, &our_public.to_be_bytes())?;
    println!("[DH] Computing shared secret...");
    println!("  Formula: secret = (their_public)^(our_private) mod p\n");
    let shared_secret = compute_shared_secret(their_public, our_private);
    println!(
        "  secret = ({:016X})^({:016X}) mod p",
        their_public, our_private
    );
    println!("         = {:016X}\n", shared_secret);
    println!("[VERIFY] Both sides computed the same secret ✓\n");
    println!("[STREAM] Generating keystream from secret...");
    println!("  Algorithm: LCG (a={}, c={}, m=2^32)", LCG_A, LCG_C);
    println!("  Seed: secret = {:016X}\n", shared_secret);
    let mut keystream = LcgKeystream::new(shared_secret);
    let preview: Vec<u8> = (0..14).map(|_| keystream.next_byte()).collect();
    println!("  Keystream: {} ...\n", format_hex_bytes(&preview));
    let mut send_keystream = LcgKeystream::new(shared_secret);
    let mut recv_keystream = LcgKeystream::new(shared_secret);
    println!("✓ Secure channel established!\n");
    let stdin = io::stdin();
    let mut stdin_reader = BufReader::new(stdin);
    loop {
        println!("[NETWORK] Waiting for message...");
        let received_cipher = receive_message(&mut stream)?;
        println!("[←] Received {} bytes\n", received_cipher.len());
        println!("[DECRYPT]");
        println!("  Cipher: {}", format_hex_bytes(&received_cipher));
        let decrypted = decrypt(&received_cipher, &mut recv_keystream);
        let key_used: Vec<u8> = {
            let mut temp_ks = LcgKeystream::new(shared_secret);
            temp_ks.get_bytes(received_cipher.len())
        };
        println!(
            "  Key:    {}  (keystream position: 0)",
            format_hex_bytes(&key_used)
        );
        println!(
            "  Plain:  {}  → \"{}\"\n",
            format_hex_bytes(&decrypted),
            String::from_utf8_lossy(&decrypted)
        );
        println!("[SERVER] {}\n", String::from_utf8_lossy(&decrypted));
        println!("[CHAT] Type message:");
        print!("> ");
        io::stdout().flush()?;
        let mut input = String::new();
        stdin_reader.read_line(&mut input)?;
        let message = input.trim();

        if message.is_empty() {
            continue;
        }

        if message == "quit" || message == "exit" {
            break;
        }

        println!("\n[ENCRYPT]");
        let plaintext = message.as_bytes();
        println!(
            "  Plain:  {}  (\"{}\")",
            format_hex_bytes(plaintext),
            message
        );

        let ciphertext = encrypt(plaintext, &mut send_keystream);
        let key_preview: Vec<u8> = {
            let mut temp_ks = LcgKeystream::new(shared_secret);
            temp_ks.get_bytes(received_cipher.len());
            temp_ks.get_bytes(plaintext.len())
        };
        println!(
            "  Key:    {}  (keystream position: {})",
            format_hex_bytes(&key_preview),
            received_cipher.len()
        );
        println!("  Cipher: {}\n", format_hex_bytes(&ciphertext));

        println!(
            "[NETWORK] Sending encrypted message ({} bytes)...",
            ciphertext.len()
        );
        send_message(&mut stream, &ciphertext)?;
        println!("[→] Sent {} bytes\n", ciphertext.len());
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Server { port } => run_server(port),
        Commands::Client { address } => run_client(&address),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}