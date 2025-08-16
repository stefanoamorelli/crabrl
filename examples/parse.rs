//! Basic parsing example

use crabrl::Parser;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <xbrl-file>", args[0]);
        std::process::exit(1);
    }

    let parser = Parser::new();
    let doc = parser.parse_file(&args[1])?;
    
    println!("Parsed {} successfully", args[1]);
    println!("  Facts: {}", doc.facts.len());
    println!("  Contexts: {}", doc.contexts.len());
    println!("  Units: {}", doc.units.len());
    
    // Show first 5 facts
    for fact in doc.facts.iter().take(5) {
        println!("  - {}: {}", fact.name, fact.value);
    }
    
    Ok(())
}