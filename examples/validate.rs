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
    let result = validator.validate(&doc)?;
    
    if result.is_valid {
        println!("✓ Document is valid");
    } else {
        println!("✗ Document has {} errors", result.errors.len());
        for error in result.errors.iter().take(5) {
            println!("  - {}", error);
        }
    }
    
    println!("\nValidation stats:");
    println!("  Facts validated: {}", result.stats.facts_validated);
    println!("  Time: {}ms", result.stats.duration_ms);
    
    Ok(())
}