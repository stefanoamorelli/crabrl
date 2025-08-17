//! crabrl - High-performance XBRL parser and validator
//!
//! Licensed under AGPL-3.0

pub mod model;
pub mod simple_parser;
pub mod validator;

// Use simple parser for now
pub use simple_parser::Parser;

// Re-export main types
pub use model::{Context, Document, Fact, Unit};

// Create validator wrapper for the CLI
pub struct Validator {
    inner: validator::XbrlValidator,
    #[allow(dead_code)]
    strict: bool,
}

impl Validator {
    pub fn new() -> Self {
        Self {
            inner: validator::XbrlValidator::new(),
            strict: false,
        }
    }

    pub fn with_config(config: ValidationConfig) -> Self {
        let mut inner = validator::XbrlValidator::new();
        if config.strict {
            inner = inner.strict();
        }
        Self {
            inner,
            strict: config.strict,
        }
    }

    pub fn sec_edgar() -> Self {
        Self {
            inner: validator::XbrlValidator::new().strict(),
            strict: true,
        }
    }

    pub fn validate(&self, doc: &Document) -> Result<ValidationResult> {
        let start = std::time::Instant::now();

        // Clone doc for validation (validator mutates it)
        let mut doc_copy = doc.clone();

        // Run validation
        let is_valid = self.inner.validate(&mut doc_copy).is_ok();

        Ok(ValidationResult {
            is_valid,
            errors: if is_valid {
                Vec::new()
            } else {
                vec!["Validation failed".to_string()]
            },
            warnings: Vec::new(),
            stats: ValidationStats {
                facts_validated: doc.facts.len(),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        })
    }
}

/// Simple validation config for CLI
pub struct ValidationConfig {
    pub strict: bool,
}

impl ValidationConfig {
    pub fn sec_edgar() -> Self {
        Self { strict: true }
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self { strict: false }
    }
}

/// Simple validation result for CLI
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub stats: ValidationStats,
}

pub struct ValidationStats {
    pub facts_validated: usize,
    pub duration_ms: u64,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),
}
