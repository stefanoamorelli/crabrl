// Linkbase processing for XBRL
use crate::{Error, Result, model::*};
use compact_str::CompactString;
use std::collections::HashMap;
use std::path::Path;

pub struct LinkbaseProcessor {
    presentation_links: HashMap<CompactString, Vec<PresentationLink>>,
    calculation_links: HashMap<CompactString, Vec<CalculationLink>>,
    definition_links: HashMap<CompactString, Vec<DefinitionLink>>,
    label_links: HashMap<CompactString, Vec<LabelLink>>,
    reference_links: HashMap<CompactString, Vec<ReferenceLink>>,
}

impl LinkbaseProcessor {
    pub fn new() -> Self {
        Self {
            presentation_links: HashMap::new(),
            calculation_links: HashMap::new(),
            definition_links: HashMap::new(),
            label_links: HashMap::new(),
            reference_links: HashMap::new(),
        }
    }

    pub fn load_linkbase<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let content = std::fs::read(path)?;
        self.parse_linkbase(&content)
    }

    pub fn parse_linkbase(&mut self, data: &[u8]) -> Result<()> {
        // Skip BOM if present
        let data = if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
            &data[3..]
        } else {
            data
        };

        let text = std::str::from_utf8(data)
            .map_err(|_| Error::Parse("Invalid UTF-8 in linkbase".to_string()))?;

        // Detect linkbase type and parse accordingly
        if text.contains("presentationLink") {
            self.parse_presentation_linkbase(text)?;
        }
        if text.contains("calculationLink") {
            self.parse_calculation_linkbase(text)?;
        }
        if text.contains("definitionLink") {
            self.parse_definition_linkbase(text)?;
        }
        if text.contains("labelLink") {
            self.parse_label_linkbase(text)?;
        }
        if text.contains("referenceLink") {
            self.parse_reference_linkbase(text)?;
        }

        Ok(())
    }

    fn parse_presentation_linkbase(&mut self, text: &str) -> Result<()> {
        // Parse presentation arcs
        let mut pos = 0;
        while let Some(arc_start) = text[pos..].find("<link:presentationArc") {
            let arc_start = pos + arc_start;
            pos = arc_start + 1;

            if let Some(arc_end) = text[arc_start..].find("/>") {
                let arc_text = &text[arc_start..arc_start + arc_end];
                
                let mut link = PresentationLink {
                    from: CompactString::new(""),
                    to: CompactString::new(""),
                    order: 1.0,
                    priority: None,
                    use_attribute: None,
                };

                // Extract from
                if let Some(from_start) = arc_text.find("xlink:from=\"") {
                    let from_start = from_start + 12;
                    if let Some(from_end) = arc_text[from_start..].find('"') {
                        link.from = CompactString::from(&arc_text[from_start..from_start + from_end]);
                    }
                }

                // Extract to
                if let Some(to_start) = arc_text.find("xlink:to=\"") {
                    let to_start = to_start + 10;
                    if let Some(to_end) = arc_text[to_start..].find('"') {
                        link.to = CompactString::from(&arc_text[to_start..to_start + to_end]);
                    }
                }

                // Extract order
                if let Some(order_start) = arc_text.find("order=\"") {
                    let order_start = order_start + 7;
                    if let Some(order_end) = arc_text[order_start..].find('"') {
                        if let Ok(order) = arc_text[order_start..order_start + order_end].parse() {
                            link.order = order;
                        }
                    }
                }

                // Extract priority
                if let Some(priority_start) = arc_text.find("priority=\"") {
                    let priority_start = priority_start + 10;
                    if let Some(priority_end) = arc_text[priority_start..].find('"') {
                        if let Ok(priority) = arc_text[priority_start..priority_start + priority_end].parse() {
                            link.priority = Some(priority);
                        }
                    }
                }

                // Extract use
                if let Some(use_start) = arc_text.find("use=\"") {
                    let use_start = use_start + 5;
                    if let Some(use_end) = arc_text[use_start..].find('"') {
                        link.use_attribute = Some(CompactString::from(&arc_text[use_start..use_start + use_end]));
                    }
                }

                self.presentation_links
                    .entry(link.from.clone())
                    .or_insert_with(Vec::new)
                    .push(link);
            }
        }

        Ok(())
    }

    fn parse_calculation_linkbase(&mut self, text: &str) -> Result<()> {
        // Parse calculation arcs
        let mut pos = 0;
        while let Some(arc_start) = text[pos..].find("<link:calculationArc") {
            let arc_start = pos + arc_start;
            pos = arc_start + 1;

            if let Some(arc_end) = text[arc_start..].find("/>") {
                let arc_text = &text[arc_start..arc_start + arc_end];
                
                let mut link = CalculationLink {
                    from: CompactString::new(""),
                    to: CompactString::new(""),
                    weight: 1.0,
                    order: 1.0,
                };

                // Extract from
                if let Some(from_start) = arc_text.find("xlink:from=\"") {
                    let from_start = from_start + 12;
                    if let Some(from_end) = arc_text[from_start..].find('"') {
                        link.from = CompactString::from(&arc_text[from_start..from_start + from_end]);
                    }
                }

                // Extract to
                if let Some(to_start) = arc_text.find("xlink:to=\"") {
                    let to_start = to_start + 10;
                    if let Some(to_end) = arc_text[to_start..].find('"') {
                        link.to = CompactString::from(&arc_text[to_start..to_start + to_end]);
                    }
                }

                // Extract weight
                if let Some(weight_start) = arc_text.find("weight=\"") {
                    let weight_start = weight_start + 8;
                    if let Some(weight_end) = arc_text[weight_start..].find('"') {
                        if let Ok(weight) = arc_text[weight_start..weight_start + weight_end].parse() {
                            link.weight = weight;
                        }
                    }
                }

                // Extract order
                if let Some(order_start) = arc_text.find("order=\"") {
                    let order_start = order_start + 7;
                    if let Some(order_end) = arc_text[order_start..].find('"') {
                        if let Ok(order) = arc_text[order_start..order_start + order_end].parse() {
                            link.order = order;
                        }
                    }
                }

                self.calculation_links
                    .entry(link.from.clone())
                    .or_insert_with(Vec::new)
                    .push(link);
            }
        }

        Ok(())
    }

    fn parse_definition_linkbase(&mut self, text: &str) -> Result<()> {
        // Parse definition arcs
        let mut pos = 0;
        while let Some(arc_start) = text[pos..].find("<link:definitionArc") {
            let arc_start = pos + arc_start;
            pos = arc_start + 1;

            if let Some(arc_end) = text[arc_start..].find("/>") {
                let arc_text = &text[arc_start..arc_start + arc_end];
                
                let mut link = DefinitionLink {
                    from: CompactString::new(""),
                    to: CompactString::new(""),
                    arcrole: CompactString::new(""),
                    order: 1.0,
                };

                // Extract from
                if let Some(from_start) = arc_text.find("xlink:from=\"") {
                    let from_start = from_start + 12;
                    if let Some(from_end) = arc_text[from_start..].find('"') {
                        link.from = CompactString::from(&arc_text[from_start..from_start + from_end]);
                    }
                }

                // Extract to
                if let Some(to_start) = arc_text.find("xlink:to=\"") {
                    let to_start = to_start + 10;
                    if let Some(to_end) = arc_text[to_start..].find('"') {
                        link.to = CompactString::from(&arc_text[to_start..to_start + to_end]);
                    }
                }

                // Extract arcrole
                if let Some(arcrole_start) = arc_text.find("xlink:arcrole=\"") {
                    let arcrole_start = arcrole_start + 15;
                    if let Some(arcrole_end) = arc_text[arcrole_start..].find('"') {
                        link.arcrole = CompactString::from(&arc_text[arcrole_start..arcrole_start + arcrole_end]);
                    }
                }

                // Extract order
                if let Some(order_start) = arc_text.find("order=\"") {
                    let order_start = order_start + 7;
                    if let Some(order_end) = arc_text[order_start..].find('"') {
                        if let Ok(order) = arc_text[order_start..order_start + order_end].parse() {
                            link.order = order;
                        }
                    }
                }

                self.definition_links
                    .entry(link.from.clone())
                    .or_insert_with(Vec::new)
                    .push(link);
            }
        }

        Ok(())
    }

    fn parse_label_linkbase(&mut self, text: &str) -> Result<()> {
        // Parse labels
        let mut pos = 0;
        while let Some(label_start) = text[pos..].find("<link:label") {
            let label_start = pos + label_start;
            pos = label_start + 1;

            if let Some(label_end) = text[label_start..].find("</link:label>") {
                let label_text = &text[label_start..label_start + label_end];
                
                let mut link = LabelLink {
                    concept: CompactString::new(""),
                    label: CompactString::new(""),
                    role: CompactString::new(""),
                    lang: CompactString::new("en"),
                };

                // Extract label ID for concept mapping
                if let Some(id_start) = label_text.find("xlink:label=\"") {
                    let id_start = id_start + 13;
                    if let Some(id_end) = label_text[id_start..].find('"') {
                        link.concept = CompactString::from(&label_text[id_start..id_start + id_end]);
                    }
                }

                // Extract role
                if let Some(role_start) = label_text.find("xlink:role=\"") {
                    let role_start = role_start + 12;
                    if let Some(role_end) = label_text[role_start..].find('"') {
                        link.role = CompactString::from(&label_text[role_start..role_start + role_end]);
                    }
                }

                // Extract lang
                if let Some(lang_start) = label_text.find("xml:lang=\"") {
                    let lang_start = lang_start + 10;
                    if let Some(lang_end) = label_text[lang_start..].find('"') {
                        link.lang = CompactString::from(&label_text[lang_start..lang_start + lang_end]);
                    }
                }

                // Extract label text content
                if let Some(content_start) = label_text.find('>') {
                    let content = &label_text[content_start + 1..];
                    link.label = CompactString::from(content.trim());
                }

                self.label_links
                    .entry(link.concept.clone())
                    .or_insert_with(Vec::new)
                    .push(link);
            }
        }

        Ok(())
    }

    fn parse_reference_linkbase(&mut self, text: &str) -> Result<()> {
        // Parse references - simplified version
        let mut pos = 0;
        while let Some(ref_start) = text[pos..].find("<link:reference") {
            let ref_start = pos + ref_start;
            pos = ref_start + 1;

            if let Some(ref_end) = text[ref_start..].find("</link:reference>") {
                let ref_text = &text[ref_start..ref_start + ref_end];
                
                let mut reference = Reference {
                    role: CompactString::new(""),
                    parts: HashMap::new(),
                };

                // Extract role
                if let Some(role_start) = ref_text.find("xlink:role=\"") {
                    let role_start = role_start + 12;
                    if let Some(role_end) = ref_text[role_start..].find('"') {
                        reference.role = CompactString::from(&ref_text[role_start..role_start + role_end]);
                    }
                }

                // Parse reference parts (simplified)
                let parts = ["Name", "Number", "Section", "Subsection", "Paragraph", "Subparagraph", "Clause"];
                for part in &parts {
                    let tag = format!("<link:{}", part);
                    if let Some(part_start) = ref_text.find(&tag) {
                        let part_start = part_start + tag.len();
                        if let Some(content_start) = ref_text[part_start..].find('>') {
                            let content_start = part_start + content_start + 1;
                            if let Some(content_end) = ref_text[content_start..].find('<') {
                                let content = &ref_text[content_start..content_start + content_end];
                                reference.parts.insert(
                                    CompactString::from(*part),
                                    content.trim().to_string()
                                );
                            }
                        }
                    }
                }

                // Find concept this reference belongs to
                if let Some(label_start) = ref_text.find("xlink:label=\"") {
                    let label_start = label_start + 13;
                    if let Some(label_end) = ref_text[label_start..].find('"') {
                        let concept = CompactString::from(&ref_text[label_start..label_start + label_end]);
                        
                        let link = ReferenceLink {
                            concept: concept.clone(),
                            reference,
                        };
                        
                        self.reference_links
                            .entry(concept)
                            .or_insert_with(Vec::new)
                            .push(link);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn get_presentation_tree(&self, root: &str) -> Vec<&PresentationLink> {
        self.presentation_links
            .get(root)
            .map(|links| {
                let mut sorted = links.iter().collect::<Vec<_>>();
                sorted.sort_by(|a, b| a.order.partial_cmp(&b.order).unwrap());
                sorted
            })
            .unwrap_or_default()
    }

    pub fn calculate_total(&self, parent: &str, facts: &HashMap<String, f64>) -> f64 {
        if let Some(links) = self.calculation_links.get(parent) {
            links.iter()
                .map(|link| {
                    facts.get(link.to.as_str())
                        .map(|value| value * link.weight)
                        .unwrap_or(0.0)
                })
                .sum()
        } else {
            facts.get(parent).copied().unwrap_or(0.0)
        }
    }

    pub fn get_label(&self, concept: &str, role: &str, lang: &str) -> Option<&str> {
        self.label_links
            .get(concept)
            .and_then(|labels| {
                labels.iter()
                    .find(|l| l.role == role && l.lang == lang)
                    .or_else(|| labels.iter().find(|l| l.lang == lang))
                    .or_else(|| labels.first())
            })
            .map(|l| l.label.as_str())
    }

    pub fn validate_calculations(&self, facts: &HashMap<String, f64>) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for (parent, links) in &self.calculation_links {
            let calculated = self.calculate_total(parent, facts);
            if let Some(&actual) = facts.get(parent.as_str()) {
                let diff = (calculated - actual).abs();
                let tolerance = 0.01; // Allow small rounding differences
                
                if diff > tolerance {
                    errors.push(ValidationError::CalculationInconsistency {
                        concept: parent.to_string(),
                        expected: calculated,
                        actual,
                    });
                }
            }
        }

        errors
    }
}
