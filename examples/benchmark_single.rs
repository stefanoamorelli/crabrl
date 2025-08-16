//! Single file benchmark

use crabrl::Parser;
use std::env;
use std::fs;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <xbrl-file>", args[0]);
        std::process::exit(1);
    }

    let content = fs::read(&args[1]).expect("Failed to read file");

    let parser = Parser::new();
    let start = Instant::now();

    match parser.parse_bytes(&content) {
        Ok(document) => {
            let elapsed = start.elapsed();
            println!(
                "Parsed in {:.3}ms: {} facts, {} contexts, {} units",
                elapsed.as_secs_f64() * 1000.0,
                document.facts.len(),
                document.contexts.len(),
                document.units.len()
            );
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
            std::process::exit(1);
        }
    }
}