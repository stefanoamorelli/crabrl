//! crabrl CLI - High-performance XBRL parser and validator

use anyhow::{Context, Result};
use clap::{Parser as ClapParser, Subcommand};
use colored::*;
use std::path::PathBuf;
use std::time::Instant;

use crabrl::{Parser, ValidationConfig, Validator};

/// High-performance XBRL parser and validator
#[derive(ClapParser)]
#[command(name = "crabrl")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse an XBRL file
    Parse {
        /// Input file
        input: PathBuf,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,

        /// Show statistics
        #[arg(short, long)]
        stats: bool,
    },

    /// Validate an XBRL file
    Validate {
        /// Input file
        input: PathBuf,

        /// Validation profile (generic, sec-edgar)
        #[arg(short, long, default_value = "generic")]
        profile: String,

        /// Treat warnings as errors
        #[arg(long)]
        strict: bool,
    },

    /// Benchmark parsing performance
    Bench {
        /// Input file
        input: PathBuf,

        /// Number of iterations
        #[arg(short, long, default_value = "100")]
        iterations: usize,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse {
            input,
            json: _,
            stats,
        } => {
            let start = Instant::now();
            let parser = Parser::new();
            let doc = parser
                .parse_file(&input)
                .with_context(|| format!("Failed to parse {}", input.display()))?;
            let elapsed = start.elapsed();

            println!("{} {}", "✓".green().bold(), input.display());
            println!("  Facts: {}", doc.facts.len());
            println!("  Contexts: {}", doc.contexts.len());
            println!("  Units: {}", doc.units.len());

            if stats {
                println!("  Time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
                println!(
                    "  Throughput: {:.0} facts/sec",
                    doc.facts.len() as f64 / elapsed.as_secs_f64()
                );
            }
        }

        Commands::Validate {
            input,
            profile,
            strict,
        } => {
            let parser = Parser::new();
            let doc = parser
                .parse_file(&input)
                .with_context(|| format!("Failed to parse {}", input.display()))?;

            let config = match profile.as_str() {
                "sec-edgar" => ValidationConfig::sec_edgar(),
                _ => ValidationConfig::default(),
            };

            let validator = Validator::with_config(config);
            let result = validator.validate(&doc)?;

            if result.is_valid {
                println!(
                    "{} {} - Document is valid",
                    "✓".green().bold(),
                    input.display()
                );
            } else {
                println!(
                    "{} {} - Validation failed",
                    "✗".red().bold(),
                    input.display()
                );
                println!("  Errors: {}", result.errors.len());
                println!("  Warnings: {}", result.warnings.len());

                for error in result.errors.iter().take(5) {
                    println!("  {} {}", "ERROR:".red(), error);
                }

                if result.errors.len() > 5 {
                    println!("  ... and {} more errors", result.errors.len() - 5);
                }

                if strict && !result.warnings.is_empty() {
                    std::process::exit(1);
                }

                if !result.is_valid {
                    std::process::exit(1);
                }
            }
        }

        Commands::Bench { input, iterations } => {
            let parser = Parser::new();

            // Warmup
            for _ in 0..3 {
                let _ = parser.parse_file(&input)?;
            }

            let mut times = Vec::with_capacity(iterations);
            let mut doc_facts = 0;

            for _ in 0..iterations {
                let start = Instant::now();
                let doc = parser.parse_file(&input)?;
                times.push(start.elapsed());
                doc_facts = doc.facts.len();
            }

            times.sort();
            let min = times[0];
            let max = times[times.len() - 1];
            let median = times[times.len() / 2];
            let mean = times.iter().sum::<std::time::Duration>() / times.len() as u32;

            println!("Benchmark Results for {}", input.display());
            println!("  Iterations: {}", iterations);
            println!("  Facts: {}", doc_facts);
            println!("  Min:    {:.3}ms", min.as_secs_f64() * 1000.0);
            println!("  Median: {:.3}ms", median.as_secs_f64() * 1000.0);
            println!("  Mean:   {:.3}ms", mean.as_secs_f64() * 1000.0);
            println!("  Max:    {:.3}ms", max.as_secs_f64() * 1000.0);
            println!(
                "  Throughput: {:.0} facts/sec",
                doc_facts as f64 / mean.as_secs_f64()
            );
        }
    }

    Ok(())
}
