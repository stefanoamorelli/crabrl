use crate::Result;
use compact_str::CompactString;
use std::collections::HashMap;

pub struct Taxonomy {
    pub schemas: Vec<Schema>,
    pub linkbases: Vec<Linkbase>,
}

pub struct Schema {
    pub target_namespace: CompactString,
    pub elements: HashMap<CompactString, Element>,
}

pub struct Element {
    pub name: CompactString,
    pub element_type: CompactString,
    pub substitution_group: Option<CompactString>,
    pub period_type: Option<CompactString>,
}

pub struct Linkbase {
    pub role: CompactString,
    pub arcs: Vec<Arc>,
}

pub struct Arc {
    pub from: CompactString,
    pub to: CompactString,
    pub order: f32,
    pub weight: f32,
}

impl Taxonomy {
    pub fn new() -> Self {
        Self {
            schemas: Vec::new(),
            linkbases: Vec::new(),
        }
    }

    pub fn load_schema(&mut self, _path: &str) -> Result<()> {
        Ok(())
    }

    pub fn load_linkbase(&mut self, _path: &str) -> Result<()> {
        Ok(())
    }
}
