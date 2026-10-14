use crate::project::{DuplicateSite, ProjectIndex};
use crate::rule::{value_int, Finding, Rule, RuleContext, RuleMeta, Tier};

const DEFAULT_MIN_STATEMENTS: i64 = 4;
const DEFAULT_MIN_NODES: i64 = 25;

pub struct NoDuplication;

impl Rule for NoDuplication {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-duplication",
            clean_code_id: "G5",
            clean_code_rule: "Duplication",
            description: "Disallow duplicated blocks of statements (Clean Code G5)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, _ctx: &RuleContext<'_>) -> Vec<Finding> {
        Vec::new()
    }

    fn check_project(&self, project: &ProjectIndex, options: &toml::Value) -> Vec<Finding> {
        let min_statements = value_int(options, "minStatements", DEFAULT_MIN_STATEMENTS) as usize;
        let min_nodes = value_int(options, "minNodes", DEFAULT_MIN_NODES) as usize;
        let mut findings = Vec::new();
        for sites in project.body_hashes.values() {
            findings.extend(group_findings(sites, min_statements, min_nodes));
        }
        findings
    }
}

fn group_findings(
    sites: &[DuplicateSite],
    min_statements: usize,
    min_nodes: usize,
) -> Vec<Finding> {
    let mut candidates: Vec<&DuplicateSite> = sites
        .iter()
        .filter(|site| site_qualifies(site, min_statements, min_nodes))
        .collect();
    if candidates.len() < 2 {
        return Vec::new();
    }
    candidates.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then(left.span.byte_start.cmp(&right.span.byte_start))
    });
    let first = candidates[0];
    candidates
        .iter()
        .skip(1)
        .map(|site| duplicate_finding(first, site))
        .collect()
}

fn site_qualifies(site: &DuplicateSite, min_statements: usize, min_nodes: usize) -> bool {
    let statements_ok = site.statement_count >= min_statements;
    let nodes_ok = site.token_count >= min_nodes;
    statements_ok && nodes_ok
}

fn duplicate_finding(first: &DuplicateSite, site: &DuplicateSite) -> Finding {
    let message = format!(
        "duplicated block; an identical block starts at {}:{} (Clean Code G5)",
        first.file.display(),
        first.span.start.line
    );
    Finding::new(site.file.clone(), site.span, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, run_rule_project};

    const BODY: &str =
        "let x = a + 1;\n    let y = x * 2;\n    let z = y - 3;\n    let w = z + 4;\n    w * 5";

    #[test]
    fn flags_duplicated_bodies() {
        let source = format!(
            "fn first(a: u8) -> u8 {{\n    {BODY}\n}}\nfn second(a: u8) -> u8 {{\n    {BODY}\n}}\n"
        );
        let findings = run_rule_project(&NoDuplication, &[("test.rs", &source)]);
        assert_eq!(
            (findings.len(), findings[0].message.contains("duplicated")),
            (1, true)
        );
    }

    #[test]
    fn allows_distinct_bodies() {
        assert_clean(
            &NoDuplication,
            "fn first(a: u8) -> u8 { a + 1 }\nfn second(a: u8) -> u8 { a + 2 }",
        );
    }
}
