use std::path::PathBuf;

use crate::engine::parse_entries;
use crate::project::ProjectIndex;
use crate::rule::{Finding, Rule, RuleContext};

pub(crate) fn run_rule(rule: &dyn Rule, source: &str) -> Vec<Finding> {
    run_rule_project(rule, &[("test.rs", source)])
}

pub(crate) fn run_rule_project(rule: &dyn Rule, files: &[(&str, &str)]) -> Vec<Finding> {
    let sources: Vec<(PathBuf, String)> = files
        .iter()
        .map(|(path, text)| (PathBuf::from(path), (*text).to_string()))
        .collect();
    let (entries, errors) = parse_entries(&sources);
    assert!(errors.is_empty(), "test sources must parse: {errors:?}");
    let project = ProjectIndex::build(entries);
    let options = toml::Value::Table(toml::Table::new());
    let mut findings = Vec::new();
    for entry in &project.entries {
        let ctx = RuleContext {
            file: &entry.source,
            ast: &entry.ast,
            options: &options,
            project: &project,
        };
        findings.extend(rule.check(&ctx));
    }
    findings.extend(rule.check_project(&project, &options));
    findings
}

pub(crate) fn assert_clean(rule: &dyn Rule, source: &str) {
    let findings = run_rule(rule, source);
    assert!(
        findings.is_empty(),
        "expected no findings, got: {:?}",
        messages(&findings)
    );
}

pub(crate) fn assert_findings(rule: &dyn Rule, source: &str, expected: usize) -> Vec<Finding> {
    let findings = run_rule(rule, source);
    assert_eq!(
        findings.len(),
        expected,
        "expected {expected} findings, got: {:?}",
        messages(&findings)
    );
    findings
}

pub(crate) fn messages(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| finding.message.clone())
        .collect()
}
