use std::collections::HashMap;
use std::env;
use std::io::{self, Read};

fn print_help() {
    println!(
        "Usage: wordfreq [OPTIONS]\n\
Count word frequency in text\n\
Arguments:\n\
  Text to analyze (or use stdin)\n\
Options:\n\
  --top Show top N words [default: 10]\n\
  --min-length Ignore words shorter than N [default: 1]\n\
  --ignore-case Case insensitive counting\n\
  -h, --help"
    );
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::new();
    let mut first_len = bytes.len() % 3;
    if first_len == 0 {
        first_len = 3;
    }
    out.push_str(&s[0..first_len]);
    let mut i = first_len;
    while i < bytes.len() {
        out.push(',');
        let next = i + 3;
        out.push_str(&s[i..next]);
        i = next;
    }
    out
}

fn collect_text_from_stdin() -> io::Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

fn main() {
    let mut top_n: usize = 10;
    let mut min_len: usize = 1;
    let mut ignore_case = false;

    let mut args = env::args().skip(1).peekable();

    let mut text_parts: Vec<String> = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "--ignore-case" => ignore_case = true,
            "--top" => {
                let v = match args.next() {
                    Some(v) => v,
                    None => {
                        eprintln!("Missing value for --top");
                        std::process::exit(2);
                    }
                };
                top_n = match v.parse::<usize>() {
                    Ok(n) if n > 0 => n,
                    _ => {
                        eprintln!("--top expects a positive integer");
                        std::process::exit(2);
                    }
                };
            }
            "--min-length" => {
                let v = match args.next() {
                    Some(v) => v,
                    None => {
                        eprintln!("Missing value for --min-length");
                        std::process::exit(2);
                    }
                };
                min_len = match v.parse::<usize>() {
                    Ok(n) if n > 0 => n,
                    _ => {
                        eprintln!("--min-length expects a positive integer");
                        std::process::exit(2);
                    }
                };
            }
            s if s.starts_with('-') => {
                eprintln!("error: Unknown option: {}", s);
                eprintln!("error: Try '--help' for usage");
                std::process::exit(2);
            }
            s => text_parts.push(s.to_string()),
        }
    }

    let input = if text_parts.is_empty() {
        match collect_text_from_stdin() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Failed to read stdin: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        text_parts.join(" ")
    };
    let from_stdin = text_parts.is_empty();

    let mut counts: HashMap<String, usize> = HashMap::new();
    for token in input.split(|c: char| !c.is_alphanumeric()) {
        if token.is_empty() {
            continue;
        }
        let word = if ignore_case {
            token.to_lowercase()
        } else {
            token.to_string()
        };
        if word.chars().count() < min_len {
            continue;
        }
        *counts.entry(word).or_insert(0) += 1;
    }

    let mut items: Vec<(String, usize)> = counts.into_iter().collect();
    items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let to_show: Vec<_> = items.into_iter().take(top_n).collect();

    if from_stdin {
        // Single-line output for stdin: "foo: 2  bar: 1"
        let parts: Vec<String> = to_show.iter().map(|(w, n)| format!("{}: {}", w, n)).collect();
        println!("{}", parts.join("  "));
    } else {
        if top_n == 10 {
            println!("Word frequency:");
        } else {
            println!("Top {} words:", top_n);
        }

        for (w, n) in to_show {
            println!("{}: {}", w, format_number(n));
        }
    }
}
