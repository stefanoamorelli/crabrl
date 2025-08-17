use std::collections::HashMap;

// ============================================================================
// Core XBRL Data Structures - Full Specification Support
// ============================================================================

#[repr(C, align(64))]
#[derive(Clone)]
pub struct FactStorage {
    pub concept_ids: Vec<u32>,
    pub context_ids: Vec<u16>,
    pub unit_ids: Vec<u16>,
    pub values: Vec<FactValue>,
    pub decimals: Vec<Option<i8>>,
    pub ids: Vec<Option<String>>,
    pub footnote_refs: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub enum FactValue {
    Text(String),
    Decimal(f64),
    Integer(i64),
    Boolean(bool),
    Date(String),
    DateTime(String),
    Nil,
}

impl FactStorage {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            concept_ids: Vec::with_capacity(capacity),
            context_ids: Vec::with_capacity(capacity),
            unit_ids: Vec::with_capacity(capacity),
            values: Vec::with_capacity(capacity),
            decimals: Vec::with_capacity(capacity),
            ids: Vec::with_capacity(capacity),
            footnote_refs: Vec::with_capacity(capacity),
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.concept_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.concept_ids.is_empty()
    }
}

// Full fact representation with all XBRL features
#[derive(Debug, Clone)]
pub struct Fact {
    pub id: Option<String>,
    pub concept: String,
    pub context_ref: String,
    pub unit_ref: Option<String>,
    pub value: String,
    pub decimals: Option<i8>,
    pub precision: Option<u8>,
    pub nil: bool,
    pub nil_reason: Option<String>,
    pub footnote_refs: Vec<String>,
}

// Context with full dimension support
#[derive(Debug, Clone)]
pub struct Context {
    pub id: String,
    pub entity: Entity,
    pub period: Period,
    pub scenario: Option<Scenario>,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub identifier: String,
    pub scheme: String,
    pub segment: Option<Segment>,
}

// Dimensional data support
#[derive(Debug, Clone)]
pub struct Segment {
    pub explicit_members: Vec<DimensionMember>,
    pub typed_members: Vec<TypedMember>,
}

#[derive(Debug, Clone)]
pub struct DimensionMember {
    pub dimension: String,
    pub member: String,
}

#[derive(Debug, Clone)]
pub struct TypedMember {
    pub dimension: String,
    pub value: String, // XML content
}

#[derive(Debug, Clone)]
pub struct Scenario {
    pub explicit_members: Vec<DimensionMember>,
    pub typed_members: Vec<TypedMember>,
}

// Period with forever support
#[derive(Debug, Clone)]
pub enum Period {
    Instant {
        date: String,
    },
    Duration {
        start: String,
        end: String,
    },
    Forever,
}

// Complex unit support with divide/multiply
#[derive(Debug, Clone)]
pub struct Unit {
    pub id: String,
    pub unit_type: UnitType,
}

#[derive(Debug, Clone)]
pub enum UnitType {
    Simple(Vec<Measure>),
    Divide {
        numerator: Vec<Measure>,
        denominator: Vec<Measure>,
    },
    Multiply(Vec<Measure>),
}

#[derive(Debug, Clone)]
pub struct Measure {
    pub namespace: String,
    pub name: String,
}

// Tuple support for structured data
#[derive(Debug, Clone)]
pub struct Tuple {
    pub id: Option<String>,
    pub name: String,
    pub facts: Vec<FactOrTuple>,
}

#[derive(Debug, Clone)]
pub enum FactOrTuple {
    Fact(Fact),
    Tuple(Box<Tuple>),
}

// Footnote support
#[derive(Debug, Clone)]
pub struct Footnote {
    pub id: String,
    pub role: Option<String>,
    pub lang: Option<String>,
    pub content: String,
    pub fact_refs: Vec<String>,
}

// Fraction support
#[derive(Debug, Clone)]
pub struct FractionValue {
    pub numerator: f64,
    pub denominator: f64,
}

// Schema and taxonomy support
#[derive(Debug, Clone)]
pub struct Schema {
    pub target_namespace: String,
    pub elements: HashMap<String, SchemaElement>,
    pub types: HashMap<String, SchemaType>,
    pub imports: Vec<SchemaImport>,
}

#[derive(Debug, Clone)]
pub struct SchemaElement {
    pub name: String,
    pub element_type: String,
    pub substitution_group: Option<String>,
    pub period_type: Option<String>,
    pub balance: Option<String>,
    pub abstract_element: bool,
    pub nillable: bool,
}

#[derive(Debug, Clone)]
pub struct SchemaType {
    pub name: String,
    pub base_type: Option<String>,
    pub restrictions: Vec<TypeRestriction>,
}

#[derive(Debug, Clone)]
pub enum TypeRestriction {
    MinInclusive(String),
    MaxInclusive(String),
    MinExclusive(String),
    MaxExclusive(String),
    Pattern(String),
    Enumeration(Vec<String>),
    Length(usize),
    MinLength(usize),
    MaxLength(usize),
}

#[derive(Debug, Clone)]
pub struct SchemaImport {
    pub namespace: String,
    pub schema_location: String,
}

// Linkbase support
#[derive(Debug, Clone)]
pub struct Linkbase {
    pub role: String,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone)]
pub enum Link {
    Presentation(PresentationLink),
    Calculation(CalculationLink),
    Definition(DefinitionLink),
    Label(LabelLink),
    Reference(ReferenceLink),
}

#[derive(Debug, Clone)]
pub struct PresentationLink {
    pub from: String,
    pub to: String,
    pub order: f32,
    pub priority: Option<i32>,
    pub use_attribute: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CalculationLink {
    pub from: String,
    pub to: String,
    pub weight: f64,
    pub order: f32,
}

#[derive(Debug, Clone)]
pub struct DefinitionLink {
    pub from: String,
    pub to: String,
    pub arcrole: String,
    pub order: f32,
}

#[derive(Debug, Clone)]
pub struct LabelLink {
    pub concept: String,
    pub label: String,
    pub role: String,
    pub lang: String,
}

#[derive(Debug, Clone)]
pub struct ReferenceLink {
    pub concept: String,
    pub reference: Reference,
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub role: String,
    pub parts: HashMap<String, String>,
}

// Main document structure with full XBRL support
#[derive(Clone)]
pub struct Document {
    pub facts: FactStorage,
    pub contexts: Vec<Context>,
    pub units: Vec<Unit>,
    pub tuples: Vec<Tuple>,
    pub footnotes: Vec<Footnote>,
    pub presentation_links: Vec<PresentationLink>,
    pub calculation_links: Vec<CalculationLink>,
    pub definition_links: Vec<DefinitionLink>,
    pub label_links: Vec<LabelLink>,
    pub reference_links: Vec<ReferenceLink>,
    pub custom_links: Vec<Link>,
    pub role_types: Vec<String>,
    pub arcrole_types: Vec<String>,
    pub schemas: Vec<Schema>,
    pub dimensions: Vec<DimensionMember>,
    pub concept_names: Vec<String>,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    pub fn new() -> Self {
        Self {
            facts: FactStorage::with_capacity(10000),
            contexts: Vec::with_capacity(100),
            units: Vec::with_capacity(50),
            tuples: Vec::new(),
            footnotes: Vec::new(),
            presentation_links: Vec::new(),
            calculation_links: Vec::new(),
            definition_links: Vec::new(),
            label_links: Vec::new(),
            reference_links: Vec::new(),
            custom_links: Vec::new(),
            role_types: Vec::new(),
            arcrole_types: Vec::new(),
            schemas: Vec::new(),
            dimensions: Vec::new(),
            concept_names: Vec::new(),
        }
    }

    pub fn with_capacity(facts: usize, contexts: usize, units: usize) -> Self {
        Self {
            facts: FactStorage::with_capacity(facts),
            contexts: Vec::with_capacity(contexts),
            units: Vec::with_capacity(units),
            tuples: Vec::new(),
            footnotes: Vec::new(),
            presentation_links: Vec::new(),
            calculation_links: Vec::new(),
            definition_links: Vec::new(),
            label_links: Vec::new(),
            reference_links: Vec::new(),
            custom_links: Vec::new(),
            role_types: Vec::new(),
            arcrole_types: Vec::new(),
            schemas: Vec::new(),
            dimensions: Vec::new(),
            concept_names: Vec::new(),
        }
    }
}
