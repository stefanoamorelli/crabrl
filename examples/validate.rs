//! Validation example

use crabrl::{Parser, Validator};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <xbrl-file>", args[0]);
        std::process::exit(1);
    }

    // Parse
    let parser = Parser::new();
    let doc = parser.parse_file(&args[1])?;

    // Validate
    let validator = Validator::new();
    match validator.validate(&doc) {
        Ok(_) => {
            println!("✓ Document is valid");
        }
        Err(e) => {
            println!("✗ Validation failed: {}", e);
        }
    }

    Ok(())
}