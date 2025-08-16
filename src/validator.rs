// Comprehensive XBRL validation
use crate::{model::*, Result, Error};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum ValidationError {
    InvalidContextRef { fact_index: usize, context_id: u16 },
    InvalidUnitRef { fact_index: usize, unit_id: u16 },
    CalculationInconsistency { concept: String, expected: f64, actual: f64 },
    InvalidDataType { concept: String, expected_type: String, actual_value: String },
    MissingRequiredElement { element: String },
    DuplicateId { id: String },
}

pub struct XbrlValidator {
    strict_mode: bool,
    #[allow(dead_code)]
    check_calculations: bool,
    check_duplicates: bool,
    check_contexts: bool,
    check_units: bool,
    #[allow(dead_code)]
    check_datatypes: bool,
    decimal_tolerance: f64,
}

impl XbrlValidator {
    pub fn new() -> Self {
        Self {
            strict_mode: false,
            check_calculations: true,
            check_duplicates: true,
            check_contexts: true,
            check_units: true,
            check_datatypes: true,
            decimal_tolerance: 0.01,
        }
    }

    pub fn strict(mut self) -> Self {
        self.strict_mode = true;
        self
    }

    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.decimal_tolerance = tolerance;
        self
    }

    pub fn validate(&self, doc: &mut Document) -> Result<()> {
        let mut validation_errors = Vec::new();
        
        // Context validation
        if self.check_contexts {
            validation_errors.extend(self.validate_contexts(doc));
        }

        // Unit validation
        if self.check_units {
            validation_errors.extend(self.validate_units(doc));
        }

        // Fact validation
        validation_errors.extend(self.validate_facts(doc));

        // Duplicate detection
        if self.check_duplicates {
            validation_errors.extend(self.check_duplicate_facts(doc));
        }

        // Return error in strict mode if any validation errors
        if self.strict_mode && !validation_errors.is_empty() {
            return Err(Error::Validation(format!(
                "Validation failed with {} errors", 
                validation_errors.len()
            )));
        }

        Ok(())
    }

    fn validate_contexts(&self, doc: &Document) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let mut context_ids = HashSet::new();
        
        for ctx in &doc.contexts {
            // Check for duplicate context IDs
            if !context_ids.insert(ctx.id.clone()) {
                errors.push(ValidationError::DuplicateId {
                    id: ctx.id.to_string(),
                });
            }

            // Validate entity identifier
            if ctx.entity.identifier.is_empty() {
                errors.push(ValidationError::MissingRequiredElement {
                    element: format!("Entity identifier for context {}", ctx.id),
                });
            }

            // Validate period
            match &ctx.period {
                Period::Duration { start, end } => {
                    if start > end {
                        errors.push(ValidationError::InvalidDataType {
                            concept: format!("context_{}", ctx.id),
                            expected_type: "valid period".to_string(),
                            actual_value: format!("start {} > end {}", start, end),
                        });
                    }
                }
                _ => {}
            }
        }
        
        errors
    }

    fn validate_units(&self, doc: &Document) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let mut unit_ids = HashSet::new();
        
        for unit in &doc.units {
            // Check for duplicate unit IDs
            if !unit_ids.insert(unit.id.clone()) {
                errors.push(ValidationError::DuplicateId {
                    id: unit.id.to_string(),
                });
            }

            // Validate measures
            match &unit.unit_type {
                UnitType::Simple(measures) => {
                    if measures.is_empty() {
                        errors.push(ValidationError::MissingRequiredElement {
                            element: format!("Measures for unit {}", unit.id),
                        });
                    }
                }
                UnitType::Divide { numerator, denominator } => {
                    if numerator.is_empty() || denominator.is_empty() {
                        errors.push(ValidationError::MissingRequiredElement {
                            element: format!("Numerator/denominator for unit {}", unit.id),
                        });
                    }
                }
                UnitType::Multiply(measures) => {
                    if measures.is_empty() {
                        errors.push(ValidationError::MissingRequiredElement {
                            element: format!("Measures for unit {}", unit.id),
                        });
                    }
                }
            }
        }
        
        errors
    }

    fn validate_facts(&self, doc: &Document) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        
        // Validate fact references
        for i in 0..doc.facts.len() {
            if i < doc.facts.context_ids.len() {
                let context_id = doc.facts.context_ids[i];
                if context_id as usize >= doc.contexts.len() {
                    errors.push(ValidationError::InvalidContextRef {
                        fact_index: i,
                        context_id,
                    });
                }
            }
            
            if i < doc.facts.unit_ids.len() {
                let unit_id = doc.facts.unit_ids[i];
                if unit_id > 0 && unit_id as usize > doc.units.len() {
                    errors.push(ValidationError::InvalidUnitRef {
                        fact_index: i,
                        unit_id,
                    });
                }
            }
        }
        
        errors
    }

    fn check_duplicate_facts(&self, doc: &Document) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let mut fact_keys = HashSet::new();
        
        for i in 0..doc.facts.len() {
            if i < doc.facts.concept_ids.len() && i < doc.facts.context_ids.len() {
                let key = (doc.facts.concept_ids[i], doc.facts.context_ids[i]);
                if !fact_keys.insert(key) && self.strict_mode {
                    errors.push(ValidationError::DuplicateId {
                        id: format!("Duplicate fact at index {}", i),
                    });
                }
            }
        }
        
        errors
    }
}

// Validation context and rules
pub struct ValidationContext {
    pub profile: ValidationProfile,
    pub custom_rules: Vec<Box<dyn Fn(&Document) -> Vec<ValidationError>>>,
}

#[derive(Debug, Clone, Copy)]
pub enum ValidationProfile {
    Generic,
    SecEdgar,
    Ifrs,
    UsGaap,
}

impl ValidationContext {
    pub fn new(profile: ValidationProfile) -> Self {
        Self {
            profile,
            custom_rules: Vec::new(),
        }
    }

    pub fn add_rule<F>(&mut self, rule: F) 
    where
        F: Fn(&Document) -> Vec<ValidationError> + 'static
    {
        self.custom_rules.push(Box::new(rule));
    }

    pub fn validate(&self, doc: &Document) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        
        // Apply profile-specific rules
        match self.profile {
            ValidationProfile::SecEdgar => {
                errors.extend(sec_validation_rules(doc));
            }
            ValidationProfile::Ifrs => {
                errors.extend(ifrs_validation_rules(doc));
            }
            _ => {}
        }
        
        // Apply custom rules
        for rule in &self.custom_rules {
            errors.extend(rule(doc));
        }
        
        errors
    }
}

// SEC EDGAR specific validation rules
pub fn sec_validation_rules(doc: &Document) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    
    // Check for required DEI contexts
    let mut has_current_period = false;
    let mut has_entity_info = false;
    let mut has_dei_elements = false;
    
    for ctx in &doc.contexts {
        // Check for current period context
        if ctx.id.contains("CurrentYear") || ctx.id.contains("CurrentPeriod") || 
           ctx.id.contains("DocumentPeriodEndDate") {
            has_current_period = true;
        }
        
        // Validate CIK format (10 digits)
        if ctx.entity.scheme.contains("sec.gov/CIK") {
            has_entity_info = true;
            let cik = &ctx.entity.identifier;
            if cik.len() != 10 || !cik.chars().all(|c| c.is_ascii_digit()) {
                errors.push(ValidationError::InvalidDataType {
                    concept: "CIK".to_string(),
                    expected_type: "10-digit number".to_string(),
                    actual_value: cik.to_string(),
                });
            }
        }
    }
    
    // Check for DEI elements in facts
    for i in 0..doc.facts.concept_ids.len() {
        if i < doc.concept_names.len() {
            let concept = &doc.concept_names[i];
            if concept.contains("dei:") || concept.contains("DocumentType") || 
               concept.contains("EntityRegistrantName") {
                has_dei_elements = true;
            }
        }
    }
    
    // Required elements validation
    if !has_current_period {
        errors.push(ValidationError::MissingRequiredElement {
            element: "Current period context required for SEC filing".to_string(),
        });
    }
    
    if !has_entity_info {
        errors.push(ValidationError::MissingRequiredElement {
            element: "Entity CIK information required for SEC filing".to_string(),
        });
    }
    
    if !has_dei_elements {
        errors.push(ValidationError::MissingRequiredElement {
            element: "DEI (Document and Entity Information) elements required".to_string(),
        });
    }
    
    // Validate segment reporting if present
    for ctx in &doc.contexts {
        if let Some(segment) = &ctx.entity.segment {
            // Check explicit members have valid dimension references
            for member in &segment.explicit_members {
                if member.dimension.is_empty() || member.member.is_empty() {
                    errors.push(ValidationError::InvalidDataType {
                        concept: format!("segment_{}", ctx.id),
                        expected_type: "valid dimension member".to_string(),
                        actual_value: format!("{}:{}", member.dimension, member.member),
                    });
                }
            }
        }
    }
    
    // Validate calculation consistency for monetary items
    let mut monetary_facts: Vec<(usize, f64)> = Vec::new();
    for i in 0..doc.facts.len() {
        if i < doc.facts.values.len() {
            if let FactValue::Decimal(val) = &doc.facts.values[i] {
                // Check if this is a monetary fact (has USD unit)
                if i < doc.facts.unit_ids.len() {
                    let unit_id = doc.facts.unit_ids[i] as usize;
                    if unit_id < doc.units.len() {
                        if let UnitType::Simple(measures) = &doc.units[unit_id].unit_type {
                            if measures.iter().any(|m| m.name == "USD" || m.name == "usd") {
                                monetary_facts.push((i, *val));
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Basic calculation validation - check for reasonable values
    for (idx, value) in monetary_facts {
        if value.is_nan() || value.is_infinite() {
            errors.push(ValidationError::InvalidDataType {
                concept: format!("fact_{}", idx),
                expected_type: "valid monetary amount".to_string(),
                actual_value: format!("{}", value),
            });
        }
        // Check for suspiciously large values (> $10 trillion)
        if value.abs() > 10_000_000_000_000.0 {
            errors.push(ValidationError::InvalidDataType {
                concept: format!("fact_{}", idx),
                expected_type: "reasonable monetary amount".to_string(),
                actual_value: format!("${:.2}", value),
            });
        }
    }
    
    errors
}

// IFRS specific validation rules  
pub fn ifrs_validation_rules(doc: &Document) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    
    // Check for IFRS-required contexts
    let mut has_reporting_period = false;
    let mut has_comparative_period = false;
    let mut has_entity_info = false;
    
    for ctx in &doc.contexts {
        // Check for reporting period
        match &ctx.period {
            Period::Duration { start, end: _ } => {
                has_reporting_period = true;
                // IFRS requires comparative information
                if start.contains("PY") || ctx.id.contains("PriorYear") || 
                   ctx.id.contains("Comparative") {
                    has_comparative_period = true;
                }
            }
            Period::Instant { date } => {
                if !date.is_empty() {
                    has_reporting_period = true;
                }
            }
            _ => {}
        }
        
        // Validate entity information
        if !ctx.entity.identifier.is_empty() {
            has_entity_info = true;
        }
    }
    
    // Required contexts validation
    if !has_reporting_period {
        errors.push(ValidationError::MissingRequiredElement {
            element: "Reporting period required for IFRS filing".to_string(),
        });
    }
    
    if !has_comparative_period {
        errors.push(ValidationError::MissingRequiredElement {
            element: "Comparative period information required by IFRS".to_string(),
        });
    }
    
    if !has_entity_info {
        errors.push(ValidationError::MissingRequiredElement {
            element: "Entity identification required for IFRS filing".to_string(),
        });
    }
    
    // Validate dimensional structure
    let mut dimension_validations = Vec::new();
    for ctx in &doc.contexts {
        // Check segment dimensions
        if let Some(segment) = &ctx.entity.segment {
            for member in &segment.explicit_members {
                // IFRS dimensions should follow specific patterns
                if !member.dimension.contains(":") {
                    dimension_validations.push(format!("Invalid dimension format: {}", member.dimension));
                }
                if member.dimension.contains("ifrs") || member.dimension.contains("ifrs-full") {
                    // Valid IFRS dimension
                    if member.member.is_empty() {
                        errors.push(ValidationError::InvalidDataType {
                            concept: format!("dimension_{}", ctx.id),
                            expected_type: "valid IFRS dimension member".to_string(),
                            actual_value: member.dimension.to_string(),
                        });
                    }
                }
            }
            
            // Check typed members for IFRS compliance
            for typed in &segment.typed_members {
                if typed.dimension.contains("ifrs") && typed.value.is_empty() {
                    errors.push(ValidationError::InvalidDataType {
                        concept: format!("typed_dimension_{}", ctx.id),
                        expected_type: "non-empty typed dimension value".to_string(),
                        actual_value: typed.dimension.to_string(),
                    });
                }
            }
        }
        
        // Check scenario dimensions (alternative to segment)
        if let Some(scenario) = &ctx.scenario {
            for member in &scenario.explicit_members {
                if member.dimension.contains("ifrs") && member.member.is_empty() {
                    errors.push(ValidationError::InvalidDataType {
                        concept: format!("scenario_dimension_{}", ctx.id),
                        expected_type: "valid IFRS scenario member".to_string(),
                        actual_value: member.dimension.to_string(),
                    });
                }
            }
        }
    }
    
    // Check for mandatory IFRS disclosures in facts
    let mut has_financial_position = false;
    let mut has_comprehensive_income = false;
    let mut has_cash_flows = false;
    let mut has_changes_in_equity = false;
    
    for i in 0..doc.concept_names.len() {
        let concept = &doc.concept_names[i];
        let lower = concept.to_lowercase();
        
        if lower.contains("financialposition") || lower.contains("balancesheet") || 
           lower.contains("assets") || lower.contains("liabilities") {
            has_financial_position = true;
        }
        
        if lower.contains("comprehensiveincome") || lower.contains("profitorloss") || 
           lower.contains("income") || lower.contains("revenue") {
            has_comprehensive_income = true;
        }
        
        if lower.contains("cashflow") || lower.contains("cashflows") {
            has_cash_flows = true;
        }
        
        if lower.contains("changesinequity") || lower.contains("equity") {
            has_changes_in_equity = true;
        }
    }
    
    // Validate mandatory statements
    if !has_financial_position {
        errors.push(ValidationError::MissingRequiredElement {
            element: "Statement of Financial Position required by IFRS".to_string(),
        });
    }
    
    if !has_comprehensive_income {
        errors.push(ValidationError::MissingRequiredElement {
            element: "Statement of Comprehensive Income required by IFRS".to_string(),
        });
    }
    
    if !has_cash_flows {
        errors.push(ValidationError::MissingRequiredElement {
            element: "Statement of Cash Flows required by IFRS".to_string(),
        });
    }
    
    if !has_changes_in_equity {
        errors.push(ValidationError::MissingRequiredElement {
            element: "Statement of Changes in Equity required by IFRS".to_string(),
        });
    }
    
    // Validate presentation linkbase relationships
    for link in &doc.presentation_links {
        // Check order is valid (typically 1.0 to 999.0)
        if link.order < 0.0 || link.order > 1000.0 {
            errors.push(ValidationError::InvalidDataType {
                concept: format!("presentation_link_{}_{}", link.from, link.to),
                expected_type: "valid presentation order (0-1000)".to_string(),
                actual_value: format!("{}", link.order),
            });
        }
    }
    
    // Validate calculation relationships
    for link in &doc.calculation_links {
        // Check weight is reasonable (-1.0 or 1.0 typically)
        if link.weight != 1.0 && link.weight != -1.0 && link.weight != 0.0 {
            // Unusual weight, might be an error
            if link.weight.abs() > 10.0 {
                errors.push(ValidationError::InvalidDataType {
                    concept: format!("calculation_link_{}_{}", link.from, link.to),
                    expected_type: "reasonable calculation weight".to_string(),
                    actual_value: format!("{}", link.weight),
                });
            }
        }
    }
    
    errors
}