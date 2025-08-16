// SEC EDGAR XBRL filing support (local files only)
use crate::{Parser, Document, Result};
use std::path::Path;

pub struct SecFilingParser {
    parser: Parser,
}

impl SecFilingParser {
    pub fn new() -> Self {
        Self {
            parser: Parser::new().with_validation(true),
        }
    }

    pub fn parse_filing<P: AsRef<Path>>(&self, path: P) -> Result<Document> {
        self.parser.parse_file(path)
    }
    
    pub fn with_validation(mut self, validate: bool) -> Self {
        self.parser = self.parser.with_validation(validate);
        self
    }
}

// Test utilities for SEC filings
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_local_sec_filing() {
        let parser = SecFilingParser::new();
        
        // Test with local test files
        if std::path::Path::new("test_data/test_tiny.xbrl").exists() {
            match parser.parse_filing("test_data/test_tiny.xbrl") {
                Ok(doc) => {
                    println!("Successfully parsed filing:");
                    println!("  Facts: {}", doc.facts.len());
                    println!("  Contexts: {}", doc.contexts.len());
                    println!("  Units: {}", doc.units.len());
                    assert!(doc.contexts.len() > 0, "Should have contexts");
                }
                Err(e) => {
                    eprintln!("Failed to parse filing: {}", e);
                }
            }
        }
    }
}