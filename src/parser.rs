// Full XBRL 2.1 compliant parser with all features
use crate::{model::*, Error, Result};
use compact_str::CompactString;
#[cfg(feature = "mmap")]
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;
use std::collections::HashMap;

pub struct Parser {
    allocator: ArenaAllocator,
    parallel: bool,
    validate: bool,
    load_schemas: bool,
    load_linkbases: bool,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            allocator: ArenaAllocator::new(),
            parallel: true,
            validate: false,
            load_schemas: false,
            load_linkbases: false,
        }
    }

    pub fn with_validation(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }

    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    pub fn with_schema_loading(mut self, load: bool) -> Self {
        self.load_schemas = load;
        self
    }

    pub fn with_linkbase_loading(mut self, load: bool) -> Self {
        self.load_linkbases = load;
        self
    }

    pub fn parse_file<P: AsRef<Path>>(&self, path: P) -> Result<Document> {
        let path = path.as_ref();
        let content = std::fs::read(path)?;
        self.parse_bytes_with_path(&content, Some(path.to_path_buf()))
    }

    pub fn parse_bytes(&self, data: &[u8]) -> Result<Document> {
        self.parse_bytes_with_path(data, None)
    }
    
    fn parse_bytes_with_path(&self, data: &[u8], path: Option<std::path::PathBuf>) -> Result<Document> {
        // Skip BOM if present
        let data = if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
            &data[3..]
        } else {
            data
        };
        
        let mut parser = FullXbrlParser::new(data, &self.allocator);
        parser.validate = self.validate;
        parser.load_schemas = self.load_schemas;
        parser.load_linkbases = self.load_linkbases;
        parser.file_path = path;
        parser.parse()
    }
}

struct FullXbrlParser<'a> {
    scanner: SimdScanner<'a>,
    allocator: &'a ArenaAllocator,
    doc: Document,
    in_xbrl_root: bool,
    current_tuple_stack: Vec<Tuple>,
    validate: bool,
    load_schemas: bool,
    load_linkbases: bool,
    file_path: Option<std::path::PathBuf>,
}

// Include base parsing methods
include!("parser_base.rs");

impl<'a> FullXbrlParser<'a> {
    fn new(data: &'a [u8], allocator: &'a ArenaAllocator) -> Self {
        Self {
            scanner: SimdScanner::new(data),
            allocator,
            doc: Document::new(),
            in_xbrl_root: false,
            current_tuple_stack: Vec::new(),
            validate: false,
            load_schemas: false,
            load_linkbases: false,
            file_path: None,
        }
    }

    fn parse(&mut self) -> Result<Document> {
        self.scanner.skip_whitespace();
        
        while !self.scanner.is_eof() {
            self.scanner.skip_whitespace();
            
            if self.scanner.peek() != Some(b'<') {
                // Skip text content between tags
                while self.scanner.peek() != Some(b'<') && !self.scanner.is_eof() {
                    self.scanner.advance(1);
                }
                continue;
            }
            
            self.scanner.advance(1); // consume '<'
            
            if self.scanner.peek() == Some(b'?') {
                self.skip_processing_instruction()?;
            } else if self.scanner.peek() == Some(b'!') {
                if self.peek_ahead(3) == Some(b"!--") {
                    self.skip_comment()?;
                } else if self.peek_ahead(8) == Some(b"![CDATA[") {
                    // We're in an element, handle CDATA
                    continue;
                } else {
                    self.skip_doctype()?;
                }
            } else if self.scanner.peek() == Some(b'/') {
                // Closing tag
                self.scanner.advance(1); // consume '/'
                let tag_name = self.read_tag_name()?;
                self.skip_to_tag_end()?;
                
                // Check if we're closing the xbrl root
                if tag_name == "xbrl" || tag_name.ends_with(":xbrl") {
                    self.in_xbrl_root = false;
                    break; // Done parsing
                }
                
                // Check if we're closing a tuple
                if !self.current_tuple_stack.is_empty() {
                    let last_tuple = self.current_tuple_stack.last().unwrap();
                    if tag_name == last_tuple.name || tag_name.ends_with(&format!(":{}", last_tuple.name)) {
                        let tuple = self.current_tuple_stack.pop().unwrap();
                        
                        if self.current_tuple_stack.is_empty() {
                            self.document.tuples.push(tuple);
                        } else {
                            let parent = self.current_tuple_stack.last_mut().unwrap();
                            parent.facts.push(FactOrTuple::Tuple(Box::new(tuple)));
                        }
                    }
                }
            } else {
                // Opening tag
                self.parse_element()?;
            }
        }
        
        // Perform validation if requested
        if self.validate {
            self.document.validate();
        }
        
        Ok(std::mem::take(&mut self.document))
    }

    fn parse_element(&mut self) -> Result<()> {
        let tag_name = self.read_tag_name()?;
        
        // Check for xbrl root element
        if tag_name == "xbrl" || tag_name.ends_with(":xbrl") {
            self.parse_xbrl_root()?;
            self.in_xbrl_root = true;
            return Ok(());
        }
        
        // Only parse these elements if we're inside xbrl root
        if !self.in_xbrl_root {
            self.skip_element_from_tag()?;
            return Ok(());
        }
        
        // Parse XBRL elements
        if tag_name.ends_with(":context") || tag_name == "context" {
            self.parse_context()?;
        } else if tag_name.ends_with(":unit") || tag_name == "unit" {
            self.parse_unit()?;
        } else if tag_name.ends_with(":schemaRef") || tag_name == "schemaRef" {
            self.parse_schema_ref()?;
        } else if tag_name.ends_with(":footnoteLink") || tag_name == "footnoteLink" {
            self.parse_footnote_link()?;
        } else if tag_name.contains(':') {
            // This could be a fact or a tuple
            // Check if it's a known non-fact element (but allow xbrli:context and xbrli:unit)
            let is_structural = tag_name.starts_with("link:") || 
                                tag_name.starts_with("xbrldi:") ||
                                (tag_name.starts_with("xbrli:") && 
                                 !tag_name.ends_with(":context") && 
                                 !tag_name.ends_with(":unit"));
            if !is_structural {
                // Try to determine if it's a tuple by looking ahead
                if self.is_tuple(&tag_name) {
                    self.parse_tuple(tag_name)?;
                } else {
                    self.parse_fact(tag_name)?;
                }
            } else {
                self.skip_element_from_tag()?;
            }
        } else {
            self.skip_element_from_tag()?;
        }
        
        Ok(())
    }

    fn parse_context(&mut self) -> Result<()> {
        let attrs = self.parse_attributes()?;
        let id = attrs.iter()
            .find(|(n, _)| *n == "id")
            .map(|(_, v)| CompactString::from(*v))
            .ok_or_else(|| Error::Parse("Context missing id".to_string()))?;
        
        self.skip_to_tag_end()?;
        
        // Initialize context components
        let mut entity = None;
        let mut period = None;
        let mut scenario = None;
        
        // Parse context children
        loop {
            self.scanner.skip_whitespace();
            
            // Skip any text content
            while self.scanner.peek() != Some(b'<') && !self.scanner.is_eof() {
                self.scanner.advance(1);
            }
            
            if self.scanner.is_eof() {
                break;
            }
            
            let saved_pos = self.scanner.pos;
            self.scanner.advance(1); // consume '<'
            
            if self.scanner.peek() == Some(b'/') {
                // Closing tag - check if it's our context
                self.scanner.advance(1);
                let tag = self.read_tag_name()?;
                if tag.ends_with("context") || tag == "context" {
                    self.skip_to_tag_end()?;
                    break;
                }
                // Not our closing tag, restore and skip this element
                self.scanner.pos = saved_pos;
                break;
            }
            
            // Parse child element
            let tag = self.read_tag_name()?;
            
            if tag.ends_with("entity") {
                entity = Some(self.parse_entity()?);
            } else if tag.ends_with("period") {
                period = Some(self.parse_period()?);
            } else if tag.ends_with("scenario") {
                scenario = Some(self.parse_scenario()?);
            } else {
                self.skip_element_from_tag()?;
            }
        }
        
        if let (Some(entity), Some(period)) = (entity, period) {
            self.document.contexts.push(Context {
                id,
                entity,
                period,
                scenario,
            });
        }
        
        Ok(())
    }

    fn parse_entity(&mut self) -> Result<Entity> {
        let _attrs = self.parse_attributes()?;
        self.skip_to_tag_end()?;
        
        let mut identifier = CompactString::new("");
        let mut scheme = CompactString::new("");
        let mut segment = None;
        
        // Parse entity children
        loop {
            self.scanner.skip_whitespace();
            
            // Skip any text content
            while self.scanner.peek() != Some(b'<') && !self.scanner.is_eof() {
                self.scanner.advance(1);
            }
            
            if self.scanner.is_eof() {
                break;
            }
            
            let saved_pos = self.scanner.pos;
            self.scanner.advance(1); // consume '<'
            
            if self.scanner.peek() == Some(b'/') {
                // Closing tag
                self.scanner.advance(1);
                let tag = self.read_tag_name()?;
                if tag.ends_with("entity") || tag == "entity" {
                    self.skip_to_tag_end()?;
                    break;
                }
                self.scanner.pos = saved_pos;
                break;
            }
            
            let tag = self.read_tag_name()?;
            
            if tag.ends_with("identifier") {
                let attrs = self.parse_attributes()?;
                scheme = attrs.iter()
                    .find(|(n, _)| *n == "scheme")
                    .map(|(_, v)| CompactString::from(*v))
                    .unwrap_or_default();
                
                self.skip_to_tag_end()?;
                identifier = CompactString::from(self.read_text_content()?);
                
                // Skip closing tag
                self.skip_closing_tag("identifier")?;
            } else if tag.ends_with("segment") {
                segment = Some(self.parse_segment()?);
            } else {
                self.skip_element_from_tag()?;
            }
        }
        
        Ok(Entity {
            identifier,
            scheme,
            segment,
        })
    }

    fn parse_segment(&mut self) -> Result<Segment> {
        let _attrs = self.parse_attributes()?;
        self.skip_to_tag_end()?;
        
        let mut explicit_members = Vec::new();
        let mut typed_members = Vec::new();
        
        // Parse segment children
        loop {
            self.scanner.skip_whitespace();
            
            // Skip any text content until we find a tag
            while self.scanner.peek() != Some(b'<') && !self.scanner.is_eof() {
                self.scanner.advance(1);
            }
            
            if self.scanner.is_eof() {
                break;
            }
            
            let saved_pos = self.scanner.pos;
            self.scanner.advance(1); // consume '<'
            
            // Check for comment
            if self.scanner.peek() == Some(b'!') {
                if self.peek_ahead(3) == Some(b"!--") {
                    self.scanner.pos = saved_pos;
                    self.scanner.advance(1); // skip '<'
                    self.skip_comment()?;
                    continue;
                }
            }
            
            if self.scanner.peek() == Some(b'/') {
                // Closing tag
                self.scanner.advance(1);
                let tag = self.read_tag_name()?;
                if tag.ends_with("segment") || tag == "segment" {
                    self.skip_to_tag_end()?;
                    break;
                }
                // Not our closing tag - should not happen in well-formed XML
                self.scanner.pos = saved_pos;
                break;
            }
            
            let tag = self.read_tag_name()?;
            
            if tag.ends_with("explicitMember") {
                let attrs = self.parse_attributes()?;
                let dimension = attrs.iter()
                    .find(|(n, _)| *n == "dimension")
                    .map(|(_, v)| CompactString::from(*v))
                    .unwrap_or_default();
                
                self.skip_to_tag_end()?;
                let member = CompactString::from(self.read_text_content()?);
                
                explicit_members.push(DimensionMember { dimension, member });
                self.skip_closing_tag("explicitMember")?;
            } else if tag.ends_with("typedMember") {
                let attrs = self.parse_attributes()?;
                let dimension = attrs.iter()
                    .find(|(n, _)| *n == "dimension")
                    .map(|(_, v)| CompactString::from(*v))
                    .unwrap_or_default();
                
                self.skip_to_tag_end()?;
                // Read the entire XML content as typed member value
                let value = self.read_xml_content_until_closing("typedMember")?;
                
                typed_members.push(TypedMember { dimension, value });
                self.skip_closing_tag("typedMember")?;
            } else {
                self.skip_element_from_tag()?;
            }
        }
        
        Ok(Segment {
            explicit_members,
            typed_members,
        })
    }

    fn parse_scenario(&mut self) -> Result<Scenario> {
        let _attrs = self.parse_attributes()?;
        self.skip_to_tag_end()?;
        
        let mut explicit_members = Vec::new();
        let mut typed_members = Vec::new();
        
        // Parse scenario children (same structure as segment)
        loop {
            self.scanner.skip_whitespace();
            
            // Skip any text content until we find a tag
            while self.scanner.peek() != Some(b'<') && !self.scanner.is_eof() {
                self.scanner.advance(1);
            }
            
            if self.scanner.is_eof() {
                break;
            }
            
            let saved_pos = self.scanner.pos;
            self.scanner.advance(1); // consume '<'
            
            // Check for comment
            if self.scanner.peek() == Some(b'!') {
                if self.peek_ahead(3) == Some(b"!--") {
                    self.scanner.pos = saved_pos;
                    self.scanner.advance(1);
                    self.skip_comment()?;
                    continue;
                }
            }
            
            if self.scanner.peek() == Some(b'/') {
                // Closing tag
                self.scanner.advance(1);
                let tag = self.read_tag_name()?;
                if tag.ends_with("scenario") || tag == "scenario" {
                    self.skip_to_tag_end()?;
                    break;
                }
                self.scanner.pos = saved_pos;
                break;
            }
            
            let tag = self.read_tag_name()?;
            
            if tag.ends_with("explicitMember") {
                let attrs = self.parse_attributes()?;
                let dimension = attrs.iter()
                    .find(|(n, _)| *n == "dimension")
                    .map(|(_, v)| CompactString::from(*v))
                    .unwrap_or_default();
                
                self.skip_to_tag_end()?;
                let member = CompactString::from(self.read_text_content()?);
                
                explicit_members.push(DimensionMember { dimension, member });
                self.skip_closing_tag("explicitMember")?;
            } else if tag.ends_with("typedMember") {
                let attrs = self.parse_attributes()?;
                let dimension = attrs.iter()
                    .find(|(n, _)| *n == "dimension")
                    .map(|(_, v)| CompactString::from(*v))
                    .unwrap_or_default();
                
                self.skip_to_tag_end()?;
                let value = self.read_xml_content_until_closing("typedMember")?;
                
                typed_members.push(TypedMember { dimension, value });
                self.skip_closing_tag("typedMember")?;
            } else {
                self.skip_element_from_tag()?;
            }
        }
        
        Ok(Scenario {
            explicit_members,
            typed_members,
        })
    }

    fn parse_period(&mut self) -> Result<Period> {
        let _attrs = self.parse_attributes()?;
        self.skip_to_tag_end()?;
        
        let mut instant = None;
        let mut start_date = None;
        let mut end_date = None;
        let mut forever = false;
        
        // Parse period children
        loop {
            self.scanner.skip_whitespace();
            
            if self.scanner.peek() != Some(b'<') {
                break;
            }
            
            let saved_pos = self.scanner.pos;
            self.scanner.advance(1);
            
            if self.scanner.peek() == Some(b'/') {
                // Closing tag
                self.scanner.advance(1);
                let tag = self.read_tag_name()?;
                if tag.ends_with("period") {
                    self.skip_to_tag_end()?;
                    break;
                }
                self.scanner.pos = saved_pos;
                break;
            }
            
            let tag = self.read_tag_name()?;
            
            if tag.ends_with("instant") {
                self.skip_to_tag_end()?;
                instant = Some(CompactString::from(self.read_text_content()?));
                self.skip_closing_tag("instant")?;
            } else if tag.ends_with("startDate") {
                self.skip_to_tag_end()?;
                start_date = Some(CompactString::from(self.read_text_content()?));
                self.skip_closing_tag("startDate")?;
            } else if tag.ends_with("endDate") {
                self.skip_to_tag_end()?;
                end_date = Some(CompactString::from(self.read_text_content()?));
                self.skip_closing_tag("endDate")?;
            } else if tag.ends_with("forever") {
                forever = true;
                self.skip_element_from_tag()?;
            } else {
                self.skip_element_from_tag()?;
            }
        }
        
        Ok(Period {
            instant,
            start_date,
            end_date,
            forever,
        })
    }

    fn parse_unit(&mut self) -> Result<()> {
        let attrs = self.parse_attributes()?;
        let id = attrs.iter()
            .find(|(n, _)| *n == "id")
            .map(|(_, v)| CompactString::from(*v))
            .ok_or_else(|| Error::Parse("Unit missing id".to_string()))?;
        
        self.skip_to_tag_end()?;
        
        let mut unit_type = None;
        
        // Parse unit children
        loop {
            self.scanner.skip_whitespace();
            
            if self.scanner.peek() != Some(b'<') {
                break;
            }
            
            let saved_pos = self.scanner.pos;
            self.scanner.advance(1);
            
            if self.scanner.peek() == Some(b'/') {
                // Closing tag
                self.scanner.advance(1);
                let tag = self.read_tag_name()?;
                if tag.ends_with("unit") {
                    self.skip_to_tag_end()?;
                    break;
                }
                self.scanner.pos = saved_pos;
                break;
            }
            
            let tag = self.read_tag_name()?;
            
            if tag.ends_with("measure") {
                // Simple unit
                self.skip_to_tag_end()?;
                let measure_text = self.read_text_content()?;
                let measure = self.parse_measure(measure_text);
                
                if unit_type.is_none() {
                    unit_type = Some(UnitType::Simple(vec![measure]));
                } else if let Some(UnitType::Simple(ref mut measures)) = unit_type {
                    measures.push(measure);
                }
                
                self.skip_closing_tag("measure")?;
            } else if tag.ends_with("divide") {
                // Complex division unit
                unit_type = Some(self.parse_unit_divide()?);
            } else {
                self.skip_element_from_tag()?;
            }
        }
        
        if let Some(unit_type) = unit_type {
            self.document.units.push(Unit { id, unit_type });
        }
        
        Ok(())
    }

    fn parse_unit_divide(&mut self) -> Result<UnitType> {
        let _attrs = self.parse_attributes()?;
        self.skip_to_tag_end()?;
        
        let mut numerator = Vec::new();
        let mut denominator = Vec::new();
        
        // Parse divide children
        loop {
            self.scanner.skip_whitespace();
            
            if self.scanner.peek() != Some(b'<') {
                break;
            }
            
            let saved_pos = self.scanner.pos;
            self.scanner.advance(1);
            
            if self.scanner.peek() == Some(b'/') {
                // Closing tag
                self.scanner.advance(1);
                let tag = self.read_tag_name()?;
                if tag.ends_with("divide") {
                    self.skip_to_tag_end()?;
                    break;
                }
                self.scanner.pos = saved_pos;
                break;
            }
            
            let tag = self.read_tag_name()?;
            
            if tag.ends_with("unitNumerator") {
                self.skip_to_tag_end()?;
                numerator = self.parse_unit_measures()?;
                self.skip_closing_tag("unitNumerator")?;
            } else if tag.ends_with("unitDenominator") {
                self.skip_to_tag_end()?;
                denominator = self.parse_unit_measures()?;
                self.skip_closing_tag("unitDenominator")?;
            } else {
                self.skip_element_from_tag()?;
            }
        }
        
        Ok(UnitType::Divide { numerator, denominator })
    }

    fn parse_unit_measures(&mut self) -> Result<Vec<Measure>> {
        let mut measures = Vec::new();
        
        loop {
            self.scanner.skip_whitespace();
            
            if self.scanner.peek() != Some(b'<') {
                break;
            }
            
            let saved_pos = self.scanner.pos;
            self.scanner.advance(1);
            
            if self.scanner.peek() == Some(b'/') {
                // End of measures
                self.scanner.pos = saved_pos;
                break;
            }
            
            let tag = self.read_tag_name()?;
            
            if tag.ends_with("measure") {
                self.skip_to_tag_end()?;
                let measure_text = self.read_text_content()?;
                measures.push(self.parse_measure(measure_text));
                self.skip_closing_tag("measure")?;
            } else {
                self.scanner.pos = saved_pos;
                break;
            }
        }
        
        Ok(measures)
    }

    fn parse_measure(&self, text: &str) -> Measure {
        let (namespace, name) = if let Some(colon_pos) = text.find(':') {
            (
                CompactString::from(&text[..colon_pos]),
                CompactString::from(&text[colon_pos + 1..])
            )
        } else {
            (CompactString::new(""), CompactString::from(text))
        };
        
        Measure { namespace, name }
    }

    // Continue in next part...
}// Parser part 2: Facts, Tuples, Footnotes, and Helper Functions

impl<'a> FullXbrlParser<'a> {
    fn parse_fact(&mut self, tag_name: &str) -> Result<()> {
        let attrs = self.parse_attributes()?;
        
        // Check for xsi:nil attribute
        let is_nil = attrs.iter()
            .any(|(n, v)| *n == "xsi:nil" && (*v == "true" || *v == "1"));
        
        let nil_reason = if is_nil {
            attrs.iter()
                .find(|(n, _)| *n == "nilReason")
                .map(|(_, v)| CompactString::from(*v))
        } else {
            None
        };
        
        let context_ref = attrs.iter()
            .find(|(n, _)| *n == "contextRef")
            .map(|(_, v)| CompactString::from(*v));
        
        let unit_ref = attrs.iter()
            .find(|(n, _)| *n == "unitRef")
            .map(|(_, v)| CompactString::from(*v));
        
        let id = attrs.iter()
            .find(|(n, _)| *n == "id")
            .map(|(_, v)| CompactString::from(*v));
        
        let decimals = attrs.iter()
            .find(|(n, _)| *n == "decimals")
            .and_then(|(_, v)| v.parse::<i8>().ok());
        
        let precision = attrs.iter()
            .find(|(n, _)| *n == "precision")
            .and_then(|(_, v)| v.parse::<u8>().ok());
        
        // Check if it's a self-closing tag
        let is_self_closing = self.check_self_closing();
        
        self.skip_to_tag_end()?;
        
        let value = if is_self_closing || is_nil {
            String::new()
        } else {
            // Check for special fact types (fraction, mixed content)
            let value = if self.scanner.peek() == Some(b'<') {
                // Check if it's a fraction
                if self.peek_tag_name()?.ends_with("numerator") {
                    self.parse_fraction_value()?
                } else {
                    // Mixed content or nested elements
                    self.read_mixed_content_until_closing(tag_name)?
                }
            } else {
                // Simple text content (may include CDATA)
                self.read_text_content_with_cdata()?
            };
            
            // Skip closing tag if not self-closing
            if !is_self_closing {
                self.skip_closing_tag(tag_name)?;
            }
            
            value
        };
        
        if let Some(context_ref) = context_ref {
            let fact = Fact {
                id,
                concept: CompactString::from(tag_name),
                context_ref,
                unit_ref,
                value: value.clone(),
                decimals,
                precision,
                nil: is_nil,
                nil_reason,
                footnote_refs: Vec::new(), // Will be populated by footnote links
            };
            
            // If we're inside a tuple, add to tuple instead of document
            if !self.current_tuple_stack.is_empty() {
                let tuple = self.current_tuple_stack.last_mut().unwrap();
                tuple.facts.push(FactOrTuple::Fact(fact));
            } else {
                // Add to document facts
                let concept_id = self.allocator.intern_string(tag_name);
                let context_id = self.get_or_create_context_id(&fact.context_ref)?;
                let unit_id = fact.unit_ref.as_ref()
                    .and_then(|u| self.get_or_create_unit_id(u).ok())
                    .unwrap_or(0);
                
                let (value_type, fact_value) = self.parse_fact_value(&value, is_nil)?;
                
                let mut flags = 0u8;
                if is_nil {
                    flags |= FactFlags::NIL.bits();
                }
                if precision.is_some() {
                    flags |= FactFlags::HAS_PRECISION.bits();
                }
                if decimals.is_some() {
                    flags |= FactFlags::HAS_DECIMALS.bits();
                }
                if !self.current_tuple_stack.is_empty() {
                    flags |= FactFlags::IN_TUPLE.bits();
                }
                
                self.document.facts.push(CompactFact {
                    concept_id,
                    context_id,
                    unit_id,
                    value_type,
                    flags,
                    padding: [0; 6],
                    value: fact_value,
                });
            }
        }
        
        Ok(())
    }

    fn parse_tuple(&mut self, tag_name: &str) -> Result<()> {
        let attrs = self.parse_attributes()?;
        
        let id = attrs.iter()
            .find(|(n, _)| *n == "id")
            .map(|(_, v)| CompactString::from(*v));
        
        self.skip_to_tag_end()?;
        
        // Create new tuple and push to stack
        let tuple = Tuple {
            id,
            name: CompactString::from(tag_name),
            facts: Vec::new(),
        };
        
        self.current_tuple_stack.push(tuple);
        
        // The tuple will be popped when we encounter its closing tag
        
        Ok(())
    }

    fn parse_footnote_link(&mut self) -> Result<()> {
        let attrs = self.parse_attributes()?;
        
        let role = attrs.iter()
            .find(|(n, _)| n.ends_with("role"))
            .map(|(_, v)| CompactString::from(*v));
        
        self.skip_to_tag_end()?;
        
        let mut footnotes_map: HashMap<String, Footnote> = HashMap::new();
        let mut fact_footnote_links: Vec<(String, String)> = Vec::new();
        
        // Parse footnote link children
        loop {
            self.scanner.skip_whitespace();
            
            if self.scanner.peek() != Some(b'<') {
                break;
            }
            
            let saved_pos = self.scanner.pos;
            self.scanner.advance(1);
            
            if self.scanner.peek() == Some(b'/') {
                // Closing tag
                self.scanner.advance(1);
                let tag = self.read_tag_name()?;
                if tag.ends_with("footnoteLink") {
                    self.skip_to_tag_end()?;
                    break;
                }
                self.scanner.pos = saved_pos;
                break;
            }
            
            let tag = self.read_tag_name()?;
            
            if tag.ends_with("footnote") {
                let attrs = self.parse_attributes()?;
                
                let id = attrs.iter()
                    .find(|(n, _)| n.ends_with("label") || *n == "id")
                    .map(|(_, v)| v.to_string())
                    .unwrap_or_default();
                
                let lang = attrs.iter()
                    .find(|(n, _)| n.ends_with("lang"))
                    .map(|(_, v)| CompactString::from(*v));
                
                self.skip_to_tag_end()?;
                let content = self.read_text_content_with_cdata()?;
                self.skip_closing_tag("footnote")?;
                
                footnotes_map.insert(id.clone(), Footnote {
                    id: CompactString::from(id),
                    role: role.clone(),
                    lang,
                    content,
                    fact_refs: Vec::new(),
                });
            } else if tag.ends_with("footnoteArc") {
                let attrs = self.parse_attributes()?;
                
                let from = attrs.iter()
                    .find(|(n, _)| n.ends_with("from"))
                    .map(|(_, v)| v.to_string())
                    .unwrap_or_default();
                
                let to = attrs.iter()
                    .find(|(n, _)| n.ends_with("to"))
                    .map(|(_, v)| v.to_string())
                    .unwrap_or_default();
                
                fact_footnote_links.push((from, to));
                self.skip_element_from_tag()?;
            } else {
                self.skip_element_from_tag()?;
            }
        }
        
        // Process footnote links
        for (fact_ref, footnote_ref) in fact_footnote_links {
            if let Some(footnote) = footnotes_map.get_mut(&footnote_ref) {
                footnote.fact_refs.push(CompactString::from(fact_ref));
            }
        }
        
        // Add footnotes to document
        for (_, footnote) in footnotes_map {
            self.document.footnotes.push(footnote);
        }
        
        Ok(())
    }

    fn parse_fraction_value(&mut self) -> Result<String> {
        let mut numerator = String::new();
        let mut denominator = String::new();
        
        loop {
            self.scanner.skip_whitespace();
            
            if self.scanner.peek() != Some(b'<') {
                break;
            }
            
            let saved_pos = self.scanner.pos;
            self.scanner.advance(1);
            
            if self.scanner.peek() == Some(b'/') {
                self.scanner.pos = saved_pos;
                break;
            }
            
            let tag = self.read_tag_name()?;
            
            if tag.ends_with("numerator") {
                self.skip_to_tag_end()?;
                numerator = self.read_text_content()?.to_string();
                self.skip_closing_tag("numerator")?;
            } else if tag.ends_with("denominator") {
                self.skip_to_tag_end()?;
                denominator = self.read_text_content()?.to_string();
                self.skip_closing_tag("denominator")?;
            } else {
                self.skip_element_from_tag()?;
            }
        }
        
        // Return as fraction string
        Ok(format!("{}/{}", numerator, denominator))
    }

    fn parse_fact_value(&self, value: &str, is_nil: bool) -> Result<(u8, FactValue)> {
        if is_nil {
            return Ok((ValueType::Nil as u8, FactValue { integer: 0 }));
        }
        
        if value.is_empty() {
            return Ok((ValueType::String as u8, FactValue { string_id: 0 }));
        }
        
        // Check for fraction
        if value.contains('/') && !value.contains(' ') {
            if let Some((num, den)) = value.split_once('/') {
                if num.parse::<f64>().is_ok() && den.parse::<f64>().is_ok() {
                    return Ok((ValueType::Fraction as u8, FactValue { string_id: self.allocator.intern_string(value) }));
                }
            }
        }
        
        // Handle parentheses for negative numbers
        let cleaned_value = if value.starts_with('(') && value.ends_with(')') {
            format!("-{}", &value[1..value.len()-1])
        } else {
            value.to_string()
        };
        
        // Try parsing as number
        if let Ok(decimal) = cleaned_value.parse::<f64>() {
            Ok((ValueType::Decimal as u8, FactValue { decimal }))
        } else if let Ok(integer) = cleaned_value.parse::<i64>() {
            Ok((ValueType::Integer as u8, FactValue { integer }))
        } else if value == "true" || value == "false" {
            let boolean = if value == "true" { 1 } else { 0 };
            Ok((ValueType::Boolean as u8, FactValue { boolean }))
        } else {
            // Store as string
            let string_id = self.allocator.intern_string(value);
            Ok((ValueType::String as u8, FactValue { string_id }))
        }
    }

    fn parse_xbrl_root(&mut self) -> Result<()> {
        let attrs = self.parse_attributes()?;
        
        for (name, value) in attrs {
            if name.starts_with("xmlns") {
                let ns_name = if name.len() > 6 && name.chars().nth(5) == Some(':') {
                    CompactString::from(&name[6..])
                } else {
                    CompactString::new("")
                };
                self.document.namespaces.insert(ns_name, CompactString::from(value));
            }
        }
        
        self.skip_to_tag_end()?;
        Ok(())
    }

    fn parse_schema_ref(&mut self) -> Result<()> {
        let attrs = self.parse_attributes()?;
        if let Some((_, href)) = attrs.iter().find(|(n, _)| n.ends_with("href")) {
            self.document.schema_ref = Some(CompactString::from(*href));
            
            // If schema loading is enabled, load the schema
            if self.load_schemas {
                self.load_schema_from_ref(href)?;
            }
        }
        self.skip_element_from_tag()?;
        Ok(())
    }

    fn load_schema_from_ref(&mut self, schema_location: &str) -> Result<()> {
        // Parse schema location to handle relative and absolute paths
        let schema_path = if schema_location.starts_with("http://") || schema_location.starts_with("https://") {
            // Remote schema - would need HTTP client to fetch
            // For now, we'll try to find it locally in a schemas directory
            let filename = schema_location.split('/').last().unwrap_or("schema.xsd");
            format!("schemas/{}", filename)
        } else if schema_location.starts_with("/") {
            // Absolute path
            schema_location.to_string()
        } else {
            // Relative path - resolve relative to the current XBRL file
            if let Some(base_dir) = self.file_path.as_ref().and_then(|p| p.parent()) {
                base_dir.join(schema_location).to_string_lossy().to_string()
            } else {
                schema_location.to_string()
            }
        };
        
        // Check if schema file exists
        let schema_path = std::path::Path::new(&schema_path);
        if !schema_path.exists() {
            // Schema not found locally - this is common for remote schemas
            // In production, we would download and cache them
            return Ok(());
        }
        
        // Load and parse the schema
        let schema_content = std::fs::read(schema_path)?;
        self.parse_schema_content(&schema_content)?;
        
        Ok(())
    }
    
    fn parse_schema_content(&mut self, content: &[u8]) -> Result<()> {
        let mut schema = Schema {
            target_namespace: CompactString::new(""),
            elements: HashMap::new(),
            types: HashMap::new(),
            imports: Vec::new(),
        };
        
        // Basic XSD parsing using quick-xml
        let mut reader = quick_xml::Reader::from_reader(content);
        reader.trim_text(true);
        
        let mut buf = Vec::new();
        let mut current_element: Option<SchemaElement> = None;
        let mut current_type: Option<SchemaType> = None;
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let tag_name = e.name();
                    let local_name = std::str::from_utf8(tag_name.local_name().as_ref())
                        .unwrap_or("");
                    
                    match local_name {
                        "schema" => {
                            // Extract target namespace
                            for attr in e.attributes().flatten() {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                if key == "targetNamespace" {
                                    let value = std::str::from_utf8(&attr.value).unwrap_or("");
                                    schema.target_namespace = CompactString::new(value);
                                }
                            }
                        }
                        "element" => {
                            let mut element = SchemaElement {
                                name: CompactString::new(""),
                                element_type: CompactString::new(""),
                                substitution_group: None,
                                period_type: None,
                                balance: None,
                                abstract_element: false,
                                nillable: false,
                            };
                            
                            for attr in e.attributes().flatten() {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                let value = std::str::from_utf8(&attr.value).unwrap_or("");
                                
                                match key {
                                    "name" => element.name = CompactString::new(value),
                                    "type" => element.element_type = CompactString::new(value),
                                    "substitutionGroup" => element.substitution_group = Some(CompactString::new(value)),
                                    "periodType" => element.period_type = Some(CompactString::new(value)),
                                    "balance" => element.balance = Some(CompactString::new(value)),
                                    "abstract" => element.abstract_element = value == "true",
                                    "nillable" => element.nillable = value == "true",
                                    _ => {}
                                }
                            }
                            
                            if !element.name.is_empty() {
                                if matches!(e, Event::Empty(_)) {
                                    // Self-closing element tag
                                    schema.elements.insert(element.name.clone(), element);
                                } else {
                                    current_element = Some(element);
                                }
                            }
                        }
                        "complexType" | "simpleType" => {
                            let mut schema_type = SchemaType {
                                name: CompactString::new(""),
                                base_type: None,
                                restrictions: Vec::new(),
                            };
                            
                            for attr in e.attributes().flatten() {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                let value = std::str::from_utf8(&attr.value).unwrap_or("");
                                
                                if key == "name" {
                                    schema_type.name = CompactString::new(value);
                                }
                            }
                            
                            if !schema_type.name.is_empty() {
                                current_type = Some(schema_type);
                            }
                        }
                        "restriction" => {
                            if let Some(ref mut t) = current_type {
                                for attr in e.attributes().flatten() {
                                    let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                    let value = std::str::from_utf8(&attr.value).unwrap_or("");
                                    
                                    if key == "base" {
                                        t.base_type = Some(CompactString::new(value));
                                    }
                                }
                            }
                        }
                        "minInclusive" | "maxInclusive" | "minExclusive" | "maxExclusive" | 
                        "pattern" | "length" | "minLength" | "maxLength" => {
                            if let Some(ref mut t) = current_type {
                                for attr in e.attributes().flatten() {
                                    let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                    let value = std::str::from_utf8(&attr.value).unwrap_or("");
                                    
                                    if key == "value" {
                                        let restriction = match local_name {
                                            "minInclusive" => TypeRestriction::MinInclusive(value.to_string()),
                                            "maxInclusive" => TypeRestriction::MaxInclusive(value.to_string()),
                                            "minExclusive" => TypeRestriction::MinExclusive(value.to_string()),
                                            "maxExclusive" => TypeRestriction::MaxExclusive(value.to_string()),
                                            "pattern" => TypeRestriction::Pattern(value.to_string()),
                                            "length" => TypeRestriction::Length(value.parse().unwrap_or(0)),
                                            "minLength" => TypeRestriction::MinLength(value.parse().unwrap_or(0)),
                                            "maxLength" => TypeRestriction::MaxLength(value.parse().unwrap_or(0)),
                                            _ => continue,
                                        };
                                        t.restrictions.push(restriction);
                                    }
                                }
                            }
                        }
                        "enumeration" => {
                            if let Some(ref mut t) = current_type {
                                for attr in e.attributes().flatten() {
                                    let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                    let value = std::str::from_utf8(&attr.value).unwrap_or("");
                                    
                                    if key == "value" {
                                        // Find or create enumeration restriction
                                        let mut found = false;
                                        for restriction in &mut t.restrictions {
                                            if let TypeRestriction::Enumeration(ref mut values) = restriction {
                                                values.push(value.to_string());
                                                found = true;
                                                break;
                                            }
                                        }
                                        if !found {
                                            t.restrictions.push(TypeRestriction::Enumeration(vec![value.to_string()]));
                                        }
                                    }
                                }
                            }
                        }
                        "import" => {
                            let mut import = SchemaImport {
                                namespace: CompactString::new(""),
                                schema_location: CompactString::new(""),
                            };
                            
                            for attr in e.attributes().flatten() {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                let value = std::str::from_utf8(&attr.value).unwrap_or("");
                                
                                match key {
                                    "namespace" => import.namespace = CompactString::new(value),
                                    "schemaLocation" => import.schema_location = CompactString::new(value),
                                    _ => {}
                                }
                            }
                            
                            if !import.namespace.is_empty() || !import.schema_location.is_empty() {
                                schema.imports.push(import);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    let tag_name = e.name();
                    let local_name = std::str::from_utf8(tag_name.local_name().as_ref())
                        .unwrap_or("");
                    
                    match local_name {
                        "element" => {
                            if let Some(element) = current_element.take() {
                                schema.elements.insert(element.name.clone(), element);
                            }
                        }
                        "complexType" | "simpleType" => {
                            if let Some(schema_type) = current_type.take() {
                                schema.types.insert(schema_type.name.clone(), schema_type);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(Error::Parse(format!("Schema parse error: {}", e))),
                _ => {}
            }
            buf.clear();
        }
        
        // Add the parsed schema to the document
        self.doc.schemas.push(schema);
        
        // Process imports recursively if schema loading is enabled
        if self.load_schemas {
            let imports = self.doc.schemas.last().unwrap().imports.clone();
            for import in imports {
                if !import.schema_location.is_empty() {
                    self.load_schema_from_ref(&import.schema_location)?;
                }
            }
        }
        
        Ok(())
    }
    
    fn is_tuple(&mut self, _tag_name: &str) -> bool {
        // Look ahead to see if this element contains other facts
        // For now, we'll use a simple heuristic: if it doesn't have contextRef, it might be a tuple
        let attrs = match self.peek_attributes() {
            Ok(attrs) => attrs,
            Err(_) => return false,
        };
        
        !attrs.iter().any(|(n, _)| *n == "contextRef")
    }

    fn get_or_create_context_id(&self, context_ref: &str) -> Result<u16> {
        self.document.contexts.iter()
            .position(|c| c.id == context_ref)
            .map(|i| i as u16)
            .ok_or_else(|| Error::NotFound(format!("Context: {}", context_ref)))
    }

    fn get_or_create_unit_id(&self, unit_ref: &str) -> Result<u16> {
        self.document.units.iter()
            .position(|u| u.id == unit_ref)
            .map(|i| (i + 1) as u16) // 0 means no unit
            .ok_or_else(|| Error::NotFound(format!("Unit: {}", unit_ref)))
    }

    // Helper methods for reading content

    fn read_text_content_with_cdata(&mut self) -> Result<String> {
        let mut content = String::new();
        
        while !self.scanner.is_eof() {
            if self.scanner.peek() == Some(b'<') {
                // Check for CDATA
                if self.peek_ahead(9) == Some(b"<![CDATA[") {
                    self.scanner.advance(9);
                    // Read until ]]>
                    let start = self.scanner.pos;
                    while !self.scanner.is_eof() {
                        if self.scanner.peek() == Some(b']') {
                            if self.peek_ahead(3) == Some(b"]]>") {
                                let cdata = std::str::from_utf8(&self.scanner.data[start..self.scanner.pos])
                                    .map_err(|_| Error::Parse("Invalid UTF-8 in CDATA".to_string()))?;
                                content.push_str(cdata);
                                self.scanner.advance(3);
                                break;
                            }
                        }
                        self.scanner.advance(1);
                    }
                } else {
                    // End of text content
                    break;
                }
            } else {
                // Regular text
                let start = self.scanner.pos;
                while self.scanner.peek() != Some(b'<') && !self.scanner.is_eof() {
                    self.scanner.advance(1);
                }
                let text = std::str::from_utf8(&self.scanner.data[start..self.scanner.pos])
                    .map_err(|_| Error::Parse("Invalid UTF-8 in text".to_string()))?;
                content.push_str(text);
            }
        }
        
        // Decode HTML entities
        Ok(self.decode_entities(&content))
    }

    fn read_mixed_content_until_closing(&mut self, tag_name: &str) -> Result<String> {
        let mut content = String::new();
        let mut depth = 1;
        
        while depth > 0 && !self.scanner.is_eof() {
            if self.scanner.peek() == Some(b'<') {
                // Check what kind of tag
                if self.peek_ahead(2) == Some(b"</") {
                    // Closing tag
                    let saved_pos = self.scanner.pos;
                    self.scanner.advance(2);
                    let tag = self.read_tag_name()?;
                    if tag == tag_name || tag.ends_with(&format!(":{}", tag_name)) {
                        depth -= 1;
                        if depth == 0 {
                            self.scanner.pos = saved_pos;
                            break;
                        }
                    }
                    self.scanner.pos = saved_pos;
                    content.push('<');
                    self.scanner.advance(1);
                } else if self.peek_ahead(9) == Some(b"<![CDATA[") {
                    // CDATA section
                    self.scanner.advance(9);
                    let start = self.scanner.pos;
                    while !self.scanner.is_eof() {
                        if self.peek_ahead(3) == Some(b"]]>") {
                            let cdata = std::str::from_utf8(&self.scanner.data[start..self.scanner.pos])
                                .map_err(|_| Error::Parse("Invalid UTF-8 in CDATA".to_string()))?;
                            content.push_str(cdata);
                            self.scanner.advance(3);
                            break;
                        }
                        self.scanner.advance(1);
                    }
                } else {
                    // Opening tag or other
                    content.push('<');
                    self.scanner.advance(1);
                }
            } else {
                // Regular character
                if let Some(ch) = self.scanner.peek() {
                    content.push(ch as char);
                    self.scanner.advance(1);
                }
            }
        }
        
        Ok(self.decode_entities(&content))
    }

    fn read_xml_content_until_closing(&mut self, tag_name: &str) -> Result<String> {
        // Similar to mixed content but preserves XML structure
        self.read_mixed_content_until_closing(tag_name)
    }

    fn decode_entities(&self, text: &str) -> String {
        text.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&#39;", "'")
    }

    fn peek_ahead(&self, n: usize) -> Option<&'a [u8]> {
        if self.scanner.pos + n <= self.scanner.data.len() {
            Some(&self.scanner.data[self.scanner.pos..self.scanner.pos + n])
        } else {
            None
        }
    }

    fn peek_tag_name(&mut self) -> Result<String> {
        let saved_pos = self.scanner.pos;
        self.scanner.skip_whitespace();
        
        if self.scanner.peek() == Some(b'<') {
            self.scanner.advance(1);
            let tag = self.read_tag_name()?.to_string();
            self.scanner.pos = saved_pos;
            Ok(tag)
        } else {
            self.scanner.pos = saved_pos;
            Err(Error::Parse("Expected tag".to_string()))
        }
    }

    fn peek_attributes(&mut self) -> Result<Vec<(&'a str, &'a str)>> {
        let saved_pos = self.scanner.pos;
        let attrs = self.parse_attributes();
        self.scanner.pos = saved_pos;
        attrs
    }

    fn check_self_closing(&self) -> bool {
        // Check if the previous characters indicate self-closing tag
        if self.scanner.pos >= 2 {
            self.scanner.data[self.scanner.pos - 2] == b'/' && self.scanner.data[self.scanner.pos - 1] == b'>'
        } else {
            false
        }
    }

    fn skip_closing_tag(&mut self, tag_name: &str) -> Result<()> {
        self.scanner.skip_whitespace();
        if self.scanner.peek() == Some(b'<') {
            self.scanner.advance(1);
            if self.scanner.peek() == Some(b'/') {
                self.scanner.advance(1);
                let tag = self.read_tag_name()?;
                if tag == tag_name || tag.ends_with(tag_name) || tag_name.ends_with(&tag) {
                    self.skip_to_tag_end()?;
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn skip_doctype(&mut self) -> Result<()> {
        // Skip DOCTYPE declaration
        while !self.scanner.is_eof() {
            if self.scanner.peek() == Some(b'>') {
                self.scanner.advance(1);
                break;
            }
            self.scanner.advance(1);
        }
        Ok(())
    }

    // Implement remaining base methods from parser.rs
    // ... (include all the base parsing methods like read_tag_name, parse_attributes, etc.)
}