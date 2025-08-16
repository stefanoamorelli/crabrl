// Schema loading and validation for XBRL
use crate::{Error, Result, model::*};
use compact_str::CompactString;
use std::collections::HashMap;
use std::path::Path;

pub struct SchemaLoader {
    cache: HashMap<CompactString, Schema>,
}

impl SchemaLoader {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn load_schema<P: AsRef<Path>>(&mut self, path: P) -> Result<&Schema> {
        let path_str = path.as_ref().to_string_lossy();
        let key = CompactString::from(path_str.as_ref());
        
        if self.cache.contains_key(&key) {
            return Ok(self.cache.get(&key).unwrap());
        }

        let schema = self.parse_schema_file(path)?;
        self.cache.insert(key.clone(), schema);
        Ok(self.cache.get(&key).unwrap())
    }

    fn parse_schema_file<P: AsRef<Path>>(&self, path: P) -> Result<Schema> {
        let content = std::fs::read(path)?;
        self.parse_schema_bytes(&content)
    }

    fn parse_schema_bytes(&self, data: &[u8]) -> Result<Schema> {
        // Simple XML parsing for schema
        let mut schema = Schema {
            target_namespace: CompactString::new(""),
            elements: HashMap::new(),
            types: HashMap::new(),
            imports: Vec::new(),
        };

        // Skip BOM if present
        let data = if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
            &data[3..]
        } else {
            data
        };

        let text = std::str::from_utf8(data)
            .map_err(|_| Error::Parse("Invalid UTF-8 in schema".to_string()))?;

        // Extract target namespace
        if let Some(ns_start) = text.find("targetNamespace=\"") {
            let ns_start = ns_start + 17;
            if let Some(ns_end) = text[ns_start..].find('"') {
                schema.target_namespace = CompactString::from(&text[ns_start..ns_start + ns_end]);
            }
        }

        // Parse elements
        let mut pos = 0;
        while let Some(elem_start) = text[pos..].find("<xs:element") {
            let elem_start = pos + elem_start;
            pos = elem_start + 1;

            // Find element end
            let elem_end = if let Some(end) = text[elem_start..].find("/>") {
                elem_start + end + 2
            } else if let Some(end) = text[elem_start..].find("</xs:element>") {
                elem_start + end + 13
            } else {
                continue;
            };

            let elem_text = &text[elem_start..elem_end];
            
            // Extract element attributes
            let mut element = SchemaElement {
                name: CompactString::new(""),
                element_type: CompactString::new(""),
                substitution_group: None,
                period_type: None,
                balance: None,
                abstract_element: elem_text.contains("abstract=\"true\""),
                nillable: elem_text.contains("nillable=\"true\""),
            };

            // Extract name
            if let Some(name_start) = elem_text.find("name=\"") {
                let name_start = name_start + 6;
                if let Some(name_end) = elem_text[name_start..].find('"') {
                    element.name = CompactString::from(&elem_text[name_start..name_start + name_end]);
                }
            }

            // Extract type
            if let Some(type_start) = elem_text.find("type=\"") {
                let type_start = type_start + 6;
                if let Some(type_end) = elem_text[type_start..].find('"') {
                    element.element_type = CompactString::from(&elem_text[type_start..type_start + type_end]);
                }
            }

            // Extract substitutionGroup
            if let Some(sg_start) = elem_text.find("substitutionGroup=\"") {
                let sg_start = sg_start + 19;
                if let Some(sg_end) = elem_text[sg_start..].find('"') {
                    element.substitution_group = Some(CompactString::from(&elem_text[sg_start..sg_start + sg_end]));
                }
            }

            // Extract XBRL-specific attributes
            if let Some(pt_start) = elem_text.find("xbrli:periodType=\"") {
                let pt_start = pt_start + 18;
                if let Some(pt_end) = elem_text[pt_start..].find('"') {
                    element.period_type = Some(CompactString::from(&elem_text[pt_start..pt_start + pt_end]));
                }
            }

            if let Some(bal_start) = elem_text.find("xbrli:balance=\"") {
                let bal_start = bal_start + 15;
                if let Some(bal_end) = elem_text[bal_start..].find('"') {
                    element.balance = Some(CompactString::from(&elem_text[bal_start..bal_start + bal_end]));
                }
            }

            if !element.name.is_empty() {
                schema.elements.insert(element.name.clone(), element);
            }
        }

        // Parse imports
        pos = 0;
        while let Some(import_start) = text[pos..].find("<xs:import") {
            let import_start = pos + import_start;
            pos = import_start + 1;

            if let Some(import_end) = text[import_start..].find("/>") {
                let import_text = &text[import_start..import_start + import_end];
                
                let mut import = SchemaImport {
                    namespace: CompactString::new(""),
                    schema_location: CompactString::new(""),
                };

                if let Some(ns_start) = import_text.find("namespace=\"") {
                    let ns_start = ns_start + 11;
                    if let Some(ns_end) = import_text[ns_start..].find('"') {
                        import.namespace = CompactString::from(&import_text[ns_start..ns_start + ns_end]);
                    }
                }

                if let Some(loc_start) = import_text.find("schemaLocation=\"") {
                    let loc_start = loc_start + 16;
                    if let Some(loc_end) = import_text[loc_start..].find('"') {
                        import.schema_location = CompactString::from(&import_text[loc_start..loc_start + loc_end]);
                    }
                }

                schema.imports.push(import);
            }
        }

        Ok(schema)
    }

    pub fn validate_element(&self, name: &str, value: &str, schema: &Schema) -> Result<()> {
        if let Some(element) = schema.elements.get(name) {
            // Check if element is abstract
            if element.abstract_element {
                return Err(Error::Validation(format!("Element {} is abstract", name)));
            }

            // Validate type
            if let Some(type_def) = schema.types.get(&element.element_type) {
                self.validate_type(value, type_def)?;
            }

            Ok(())
        } else {
            // Element not found in schema - might be from imported schema
            Ok(())
        }
    }

    fn validate_type(&self, value: &str, type_def: &SchemaType) -> Result<()> {
        for restriction in &type_def.restrictions {
            match restriction {
                TypeRestriction::MinInclusive(min) => {
                    if let (Ok(val), Ok(min_val)) = (value.parse::<f64>(), min.parse::<f64>()) {
                        if val < min_val {
                            return Err(Error::Validation(format!("Value {} is less than minimum {}", val, min_val)));
                        }
                    }
                }
                TypeRestriction::MaxInclusive(max) => {
                    if let (Ok(val), Ok(max_val)) = (value.parse::<f64>(), max.parse::<f64>()) {
                        if val > max_val {
                            return Err(Error::Validation(format!("Value {} is greater than maximum {}", val, max_val)));
                        }
                    }
                }
                TypeRestriction::Pattern(pattern) => {
                    // Simple pattern matching - could use regex for complex patterns
                    if !value.contains(pattern) {
                        return Err(Error::Validation(format!("Value {} doesn't match pattern {}", value, pattern)));
                    }
                }
                TypeRestriction::MinLength(min) => {
                    if value.len() < *min {
                        return Err(Error::Validation(format!("Value length {} is less than minimum {}", value.len(), min)));
                    }
                }
                TypeRestriction::MaxLength(max) => {
                    if value.len() > *max {
                        return Err(Error::Validation(format!("Value length {} is greater than maximum {}", value.len(), max)));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

// Schema validator for documents
pub struct SchemaValidator {
    schemas: Vec<Schema>,
}

impl SchemaValidator {
    pub fn new() -> Self {
        Self {
            schemas: Vec::new(),
        }
    }

    pub fn add_schema(&mut self, schema: Schema) {
        self.schemas.push(schema);
    }

    pub fn validate_document(&self, doc: &Document) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Validate facts against schemas
        for i in 0..doc.facts.len() {
            if let Some(_fact) = doc.facts.get(i) {
                // Would need to map fact concept_id back to concept name
                // and validate against schema
                // This is simplified for now
            }
        }

        // Check for required elements
        for schema in &self.schemas {
            for (name, element) in &schema.elements {
                if !element.nillable && !element.abstract_element {
                    // Check if this required element exists in document
                    // This would require reverse mapping from concept names to facts
                    let _found = false;
                    // if !found {
                    //     errors.push(ValidationError::MissingRequiredElement {
                    //         element: name.to_string(),
                    //     });
                    // }
                }
            }
        }

        errors
    }
}
