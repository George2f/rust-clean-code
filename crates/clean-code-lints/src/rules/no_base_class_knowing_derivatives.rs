use std::collections::HashMap;

use syn::visit::Visit;

use crate::project::{FileEntry, ProjectIndex};
use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};
use crate::source::SourceFile;

pub struct NoBaseClassKnowingDerivatives;

impl Rule for NoBaseClassKnowingDerivatives {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-base-class-knowing-derivatives",
            clean_code_id: "G7",
            clean_code_rule: "Base Classes Depending on Their Derivatives",
            description:
                "Disallow trait default methods referencing their own implementors (Clean Code G7)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, _ctx: &RuleContext<'_>) -> Vec<Finding> {
        Vec::new()
    }

    fn check_project(&self, project: &ProjectIndex, _options: &toml::Value) -> Vec<Finding> {
        let implementors = implementor_map(project);
        let mut findings = Vec::new();
        for entry in &project.entries {
            findings.extend(file_findings(entry, &implementors));
        }
        findings
    }
}

type Implementors = HashMap<String, Vec<String>>;

fn implementor_map(project: &ProjectIndex) -> Implementors {
    let mut implementors: Implementors = HashMap::new();
    for entry in &project.entries {
        for item in &entry.ast.items {
            let syn::Item::Impl(item_impl) = item else {
                continue;
            };
            let Some((trait_segment, type_segment)) = impl_names(item_impl) else {
                continue;
            };
            implementors
                .entry(trait_segment)
                .or_default()
                .push(type_segment);
        }
    }
    implementors
}

fn impl_names(item_impl: &syn::ItemImpl) -> Option<(String, String)> {
    let (_, trait_path, _) = item_impl.trait_.as_ref()?;
    let trait_segment = trait_path.segments.last()?;
    let syn::Type::Path(type_path) = item_impl.self_ty.as_ref() else {
        return None;
    };
    let type_segment = type_path.path.segments.last()?;
    Some((
        trait_segment.ident.to_string(),
        type_segment.ident.to_string(),
    ))
}

fn file_findings(entry: &FileEntry, implementors: &Implementors) -> Vec<Finding> {
    let mut findings = Vec::new();
    for item in &entry.ast.items {
        let syn::Item::Trait(item_trait) = item else {
            continue;
        };
        let trait_name = item_trait.ident.to_string();
        let Some(types) = implementors.get(&trait_name) else {
            continue;
        };
        findings.extend(trait_body_findings(entry, item_trait, types));
    }
    findings
}

fn trait_body_findings(
    entry: &FileEntry,
    item_trait: &syn::ItemTrait,
    types: &[String],
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for member in &item_trait.items {
        let syn::TraitItem::Fn(function) = member else {
            continue;
        };
        let Some(block) = &function.default else {
            continue;
        };
        let mut visitor = ImplementorScan {
            source: &entry.source,
            types,
            findings: Vec::new(),
        };
        visitor.visit_block(block);
        findings.extend(visitor.findings);
    }
    findings
}

struct ImplementorScan<'a> {
    source: &'a SourceFile,
    types: &'a [String],
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for ImplementorScan<'_> {
    fn visit_ident(&mut self, node: &'ast proc_macro2::Ident) {
        let name = node.to_string();
        let is_implementor = self.types.iter().any(|candidate| candidate == &name);
        if is_implementor {
            let message = format!(
                "trait default method references its implementor `{name}`; invert the dependency (Clean Code G7)"
            );
            let span = self.source.span_of(node);
            self.findings
                .push(Finding::new(self.source.path.clone(), span, message));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, run_rule_project};

    #[test]
    fn flags_trait_knowing_implementor() {
        let source = "trait Shape { fn area(&self) -> u32 { Circle::default_area() } }\nstruct Circle;\nimpl Shape for Circle { fn area(&self) -> u32 { 0 } }\n";
        let findings = run_rule_project(&NoBaseClassKnowingDerivatives, &[("test.rs", source)]);
        assert_eq!(
            (findings.len(), findings[0].message.contains("Circle")),
            (1, true)
        );
    }

    #[test]
    fn allows_generic_default_method() {
        assert_clean(
            &NoBaseClassKnowingDerivatives,
            "trait Shape { fn area(&self) -> u32 { 0 } }\nstruct Circle;\nimpl Shape for Circle { fn area(&self) -> u32 { 0 } }",
        );
    }
}
