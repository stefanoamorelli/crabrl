use compact_str::CompactString;
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
    pub ids: Vec<Option<CompactString>>,
    pub footnote_refs: Vec<Vec<CompactString>>,
}

#[derive(Debug, Clone)]
pub enum FactValue {
    Text(CompactString),
    Decimal(f64),
    Integer(i64),
    Boolean(bool),
    Date(CompactString),
    DateTime(CompactString),
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
    pub id: Option<CompactString>,
    pub concept: CompactString,
    pub context_ref: CompactString,
    pub unit_ref: Option<CompactString>,
    pub value: String,
    pub decimals: Option<i8>,
    pub precision: Option<u8>,
    pub nil: bool,
    pub nil_reason: Option<CompactString>,
    pub footnote_refs: Vec<CompactString>,
}

// Context with full dimension support
#[derive(Debug, Clone)]
pub struct Context {
    pub id: CompactString,
    pub entity: Entity,
    pub period: Period,
    pub scenario: Option<Scenario>,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub identifier: CompactString,
    pub scheme: CompactString,
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
    pub dimension: CompactString,
    pub member: CompactString,
}

#[derive(Debug, Clone)]
pub struct TypedMember {
    pub dimension: CompactString,
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
    Instant { date: CompactString },
    Duration { start: CompactString, end: CompactString },
    Forever,
}

// Complex unit support with divide/multiply
#[derive(Debug, Clone)]
pub struct Unit {
    pub id: CompactString,
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
    pub namespace: CompactString,
    pub name: CompactString,
}

// Tuple support for structured data
#[derive(Debug, Clone)]
pub struct Tuple {
    pub id: Option<CompactString>,
    pub name: CompactString,
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
    pub id: CompactString,
    pub role: Option<CompactString>,
    pub lang: Option<CompactString>,
    pub content: String,
    pub fact_refs: Vec<CompactString>,
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
    pub target_namespace: CompactString,
    pub elements: HashMap<CompactString, SchemaElement>,
    pub types: HashMap<CompactString, SchemaType>,
    pub imports: Vec<SchemaImport>,
}

#[derive(Debug, Clone)]
pub struct SchemaElement {
    pub name: CompactString,
    pub element_type: CompactString,
    pub substitution_group: Option<CompactString>,
    pub period_type: Option<CompactString>,
    pub balance: Option<CompactString>,
    pub abstract_element: bool,
    pub nillable: bool,
}

#[derive(Debug, Clone)]
pub struct SchemaType {
    pub name: CompactString,
    pub base_type: Option<CompactString>,
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
    pub namespace: CompactString,
    pub schema_location: CompactString,
}

// Linkbase support
#[derive(Debug, Clone)]
pub struct Linkbase {
    pub role: CompactString,
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
    pub from: CompactString,
    pub to: CompactString,
    pub order: f32,
    pub priority: Option<i32>,
    pub use_attribute: Option<CompactString>,
}

#[derive(Debug, Clone)]
pub struct CalculationLink {
    pub from: CompactString,
    pub to: CompactString,
    pub weight: f64,
    pub order: f32,
}

#[derive(Debug, Clone)]
pub struct DefinitionLink {
    pub from: CompactString,
    pub to: CompactString,
    pub arcrole: CompactString,
    pub order: f32,
}

#[derive(Debug, Clone)]
pub struct LabelLink {
    pub concept: CompactString,
    pub label: CompactString,
    pub role: CompactString,
    pub lang: CompactString,
}

#[derive(Debug, Clone)]
pub struct ReferenceLink {
    pub concept: CompactString,
    pub reference: Reference,
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub role: CompactString,
    pub parts: HashMap<CompactString, String>,
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
    pub role_types: Vec<CompactString>,
    pub arcrole_types: Vec<CompactString>,
    pub schemas: Vec<Schema>,
    pub dimensions: Vec<DimensionMember>,
    pub concept_names: Vec<CompactString>,
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


