use std::collections::HashMap;

use syn::visit::Visit;

use crate::project::{FileEntry, ProjectIndex};
use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};
use crate::source::SourceFile;

pub struct NoInheritedConstants;

impl Rule for NoInheritedConstants {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-inherited-constants",
            clean_code_id: "J2",
            clean_code_rule: "Don't Inherit Constants",
            description: "Disallow referencing inherited constants through the implementing type (Clean Code J2)",
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
        let trait_constants = trait_constants(project);
        let mut findings = Vec::new();
        for entry in &project.entries {
            findings.extend(file_findings(entry, &trait_constants));
        }
        findings
    }
}

type TraitConstants = HashMap<String, String>;

fn trait_constants(project: &ProjectIndex) -> TraitConstants {
    let mut constants: TraitConstants = HashMap::new();
    for entry in &project.entries {
        constants.extend(file_trait_constants(&entry.ast));
    }
    constants
}

fn file_trait_constants(ast: &syn::File) -> TraitConstants {
    let mut constants = TraitConstants::new();
    for item in &ast.items {
        let syn::Item::Trait(item_trait) = item else {
            continue;
        };
        for member in &item_trait.items {
            if let Some((constant, trait_name)) = trait_member_constants(item_trait, member) {
                constants.insert(constant, trait_name);
            }
        }
    }
    constants
}

fn trait_member_constants(
    item_trait: &syn::ItemTrait,
    member: &syn::TraitItem,
) -> Option<(String, String)> {
    let syn::TraitItem::Const(constant) = member else {
        return None;
    };
    Some((constant.ident.to_string(), item_trait.ident.to_string()))
}

fn file_findings(entry: &FileEntry, trait_constants: &TraitConstants) -> Vec<Finding> {
    let mut visitor = PathScan {
        source: &entry.source,
        trait_constants,
        findings: Vec::new(),
    };
    visitor.visit_file(&entry.ast);
    visitor.findings
}

struct PathScan<'a> {
    source: &'a SourceFile,
    trait_constants: &'a TraitConstants,
    findings: Vec<Finding>,
}

fn starts_uppercase(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|first| first.is_ascii_uppercase())
}

struct ConstantRef {
    owner: String,
    constant: String,
    trait_name: String,
}

fn pair_segments(
    segments: &syn::punctuated::Punctuated<syn::PathSegment, syn::token::PathSep>,
) -> Option<(&syn::Ident, &syn::Ident)> {
    let mut iter = segments.iter();
    let first = iter.next()?;
    let second = iter.next()?;
    let extra = iter.next();
    if extra.is_some() {
        return None;
    }
    Some((&first.ident, &second.ident))
}

fn owning_trait(
    owner_name: &str,
    constant_name: &str,
    constants: &TraitConstants,
) -> Option<String> {
    if !starts_uppercase(owner_name) {
        return None;
    }
    if owner_name == "Self" {
        return None;
    }
    let trait_name = constants.get(constant_name)?.clone();
    if owner_name == trait_name {
        return None;
    }
    Some(trait_name)
}

fn constant_ref(
    segments: &syn::punctuated::Punctuated<syn::PathSegment, syn::token::PathSep>,
    constants: &TraitConstants,
) -> Option<ConstantRef> {
    let (owner, constant) = pair_segments(segments)?;
    let owner_name = owner.to_string();
    let constant_name = constant.to_string();
    let trait_name = owning_trait(&owner_name, &constant_name, constants)?;
    Some(ConstantRef {
        owner: owner_name,
        constant: constant_name,
        trait_name,
    })
}

impl PathScan<'_> {
    fn path_finding(&self, node: &syn::ExprPath) -> Option<Finding> {
        let reference = constant_ref(&node.path.segments, self.trait_constants)?;
        Some(self.constant_finding(node, &reference))
    }

    fn constant_finding(&self, node: &syn::ExprPath, reference: &ConstantRef) -> Finding {
        let message = format!(
            "`{}::{}` reaches a constant through an implementor; use `{}::{}` (Clean Code J2)",
            reference.owner, reference.constant, reference.trait_name, reference.constant
        );
        let span = self.source.span_of(node);
        Finding::new(self.source.path.clone(), span, message)
    }
}

impl<'ast> Visit<'ast> for PathScan<'_> {
    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if let Some(finding) = self.path_finding(node) {
            self.findings.push(finding);
        }
        syn::visit::visit_expr_path(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, run_rule_project};

    #[test]
    fn flags_constant_reached_through_implementor() {
        let source = "trait Limits { const MAX: u8; }\nstruct Config;\nimpl Limits for Config { const MAX: u8 = 1; }\nfn run() -> u8 { Config::MAX }\n";
        let findings = run_rule_project(&NoInheritedConstants, &[("test.rs", source)]);
        assert_eq!(
            (findings.len(), findings[0].message.contains("Limits::MAX")),
            (1, true)
        );
    }

    #[test]
    fn allows_reference_through_trait() {
        assert_clean(
            &NoInheritedConstants,
            "trait Limits { const MAX: u8; }\nfn run() -> u8 { Limits::MAX }",
        );
    }
}
