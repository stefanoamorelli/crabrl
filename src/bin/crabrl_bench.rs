use crabrl::Parser;
use std::env;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <xbrl_file>", args[0]);
        std::process::exit(1);
    }

    let filepath = &args[1];
    let parser = Parser::new();
    
    let start = Instant::now();
    match parser.parse_file(filepath) {
        Ok(doc) => {
            let elapsed = start.elapsed();
            let ms = elapsed.as_secs_f64() * 1000.0;
            
            println!("crabrl found: {} facts, {} contexts, {} units (in {:.3}ms)",
                     doc.facts.len(),
                     doc.contexts.len(),
                     doc.units.len(),
                     ms);
            
            // Additional stats
            println!("Facts: {}", doc.facts.len());
            println!("Contexts: {}", doc.contexts.len());
            println!("Units: {}", doc.units.len());
            println!("Tuples: {}", doc.tuples.len());
            println!("Footnotes: {}", doc.footnotes.len());
            println!("Time: {:.3}ms", ms);
        }
        Err(e) => {
            eprintln!("Error parsing file: {}", e);
            std::process::exit(1);
        }
    }
}