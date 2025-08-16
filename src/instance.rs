use crate::model::Document;
use crate::Result;

pub struct InstanceValidator {
    strict: bool,
}

impl InstanceValidator {
    pub fn new() -> Self {
        Self { strict: false }
    }

    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    pub fn validate(&self, _document: &Document) -> Result<()> {
        Ok(())
    }
}