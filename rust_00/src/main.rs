use std::env;

fn print_help() {
    println!(
        "Usage: hello [OPTIONS] [NAME]\n\
Arguments:\n\
  [NAME] Name to greet [default: World]\n\
Options:\n\
  --upper Convert to uppercase\n\
  --repeat Repeat greeting N times [default: 1]\n\
  -h, --help Print help"
    );
}

fn main() {
    let mut name = String::from("World");
    let mut upper = false;
    let mut repeat: usize = 1;

    let mut args = env::args().skip(1).peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "--upper" => upper = true,
            "--repeat" => {
                let val = match args.next() {
                    Some(v) => v,
                    None => {
                        eprintln!("Missing value for --repeat");
                        eprintln!("Try '--help' for usage");
                        std::process::exit(2);
                    }
                };
                repeat = match val.parse::<usize>() {
                    Ok(n) if n > 0 => n,
                    _ => {
                        eprintln!("--repeat expects a positive integer");
                        std::process::exit(2);
                    }
                };
            }
            s if s.starts_with('-') => {
                eprintln!("Unknown option: {}", s);
                eprintln!("Try '--help' for usage");
                std::process::exit(2);
            }
            s => name = s.to_string(),
        }
    }

    let mut msg = format!("Hello, {}!", name);
    if upper {
        msg = msg.to_uppercase();
    }
    for _ in 0..repeat {
        println!("{}", msg);
    }
}
