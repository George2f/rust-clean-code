use crate::project::{FunctionInfo, ProjectIndex};
use crate::rule::{value_names, Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct NoDeadFunctions;

impl Rule for NoDeadFunctions {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-dead-functions",
            clean_code_id: "F4",
            clean_code_rule: "Dead Function",
            description: "Disallow functions that are never referenced (Clean Code F4)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: true,
        };
        &META
    }

    fn check(&self, _ctx: &RuleContext<'_>) -> Vec<Finding> {
        Vec::new()
    }

    fn check_project(&self, project: &ProjectIndex, options: &toml::Value) -> Vec<Finding> {
        let allow = value_names(options, "allow");
        let mut findings = Vec::new();
        for function in &project.functions {
            if is_exempt(function, &allow) {
                continue;
            }
            let count = reference_count(project, function);
            if count <= 1 {
                findings.push(dead_finding(function));
            }
        }
        findings
    }
}

fn reference_count(project: &ProjectIndex, function: &FunctionInfo) -> usize {
    project
        .ident_counts
        .get(&function.name)
        .copied()
        .unwrap_or(0)
}

fn dead_finding(function: &FunctionInfo) -> Finding {
    let message = format!(
        "function `{}` is never referenced; delete it (Clean Code F4)",
        function.name
    );
    Finding::new(function.file.clone(), function.name_span, message)
}

fn is_exempt(function: &FunctionInfo, allow: &[String]) -> bool {
    let reserved = is_reserved(function);
    let configured = allow.contains(&function.name);
    reserved || configured
}

fn is_reserved(function: &FunctionInfo) -> bool {
    let declared_public = function.is_pub || function.in_trait_impl;
    let test_or_exported = function.is_test || function.no_mangle;
    let named_main = function.name == "main";
    declared_public || test_or_exported || named_main
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, run_rule_project};

    #[test]
    fn flags_unreferenced_private_function() {
        let findings = run_rule_project(
            &NoDeadFunctions,
            &[(
                "test.rs",
                "pub fn run() { used(); }\nfn used() {}\nfn unused_helper() {}\n",
            )],
        );
        assert_eq!(
            (
                findings.len(),
                findings[0].message.contains("unused_helper")
            ),
            (1, true)
        );
    }

    #[test]
    fn allows_referenced_function() {
        assert_clean(&NoDeadFunctions, "pub fn run() { used(); }\nfn used() {}");
    }
}
