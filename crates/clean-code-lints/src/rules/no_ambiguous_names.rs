use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};
use crate::source::{NodeSpan, SourceFile};

const DEFAULT_CONFUSING: &[&str] = &["l", "O", "I"];

pub struct NoAmbiguousNames;

impl Rule for NoAmbiguousNames {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-ambiguous-names",
            clean_code_id: "N4",
            clean_code_rule: "Unambiguous Names",
            description:
                "Disallow names that are visually ambiguous or differ only by case (Clean Code N4)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let confusing = ctx.options_names_or("confusingNames", DEFAULT_CONFUSING);
        let mut collector = Collector {
            source: ctx.file,
            names: Vec::new(),
        };
        collector.visit_file(ctx.ast);
        let names = collector.names;
        let mut findings = Vec::new();
        findings.extend(confusing_findings(ctx, &confusing, &names));
        findings.extend(case_findings(ctx, &names));
        findings
    }
}

fn confusing_findings(
    ctx: &RuleContext<'_>,
    confusing: &[String],
    names: &[(String, NodeSpan)],
) -> Vec<Finding> {
    names
        .iter()
        .filter(|(name, _)| confusing.iter().any(|confusing| confusing == name))
        .map(|(name, span)| {
            let message = format!("name `{name}` is visually ambiguous; rename it (Clean Code N4)");
            ctx.finding(*span, message)
        })
        .collect()
}

fn case_findings(ctx: &RuleContext<'_>, names: &[(String, NodeSpan)]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (index, (left, left_span)) in names.iter().enumerate() {
        for (right, right_span) in names.iter().skip(index + 1) {
            let left_entry = NameEntry {
                name: left,
                span: *left_span,
            };
            let right_entry = NameEntry {
                name: right,
                span: *right_span,
            };
            findings.extend(pair_findings(ctx, left_entry, right_entry));
        }
    }
    findings
}

struct NameEntry<'a> {
    name: &'a str,
    span: NodeSpan,
}

fn pair_findings(ctx: &RuleContext<'_>, left: NameEntry<'_>, right: NameEntry<'_>) -> Vec<Finding> {
    let collides = left.name != right.name && left.name.to_lowercase() == right.name.to_lowercase();
    if !collides {
        return Vec::new();
    }
    vec![
        ctx.finding(
            left.span,
            format!(
                "`{}` and `{}` differ only by case; rename one (Clean Code N4)",
                left.name, right.name
            ),
        ),
        ctx.finding(
            right.span,
            format!(
                "`{}` and `{}` differ only by case; rename one (Clean Code N4)",
                right.name, left.name
            ),
        ),
    ]
}

struct Collector<'a> {
    source: &'a SourceFile,
    names: Vec<(String, NodeSpan)>,
}

impl Collector<'_> {
    fn record(&mut self, name: &syn::Ident) {
        self.names
            .push((name.to_string(), self.source.span_of(name)));
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        self.record(&node.ident);
        syn::visit::visit_pat_ident(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.record(&node.sig.ident);
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.record(&node.sig.ident);
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_field(&mut self, node: &'ast syn::Field) {
        if let Some(ident) = &node.ident {
            self.record(ident);
        }
        syn::visit::visit_field(self, node);
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        self.record(&node.ident);
        syn::visit::visit_item_const(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_confusing_names() {
        assert_findings(&NoAmbiguousNames, "fn run() { let l = 1; }", 1);
    }

    #[test]
    fn flags_case_only_collision() {
        assert_findings(
            &NoAmbiguousNames,
            "fn run() { let total = 1; let TOTAL = 2; }",
            2,
        );
    }

    #[test]
    fn allows_descriptive_names() {
        assert_clean(&NoAmbiguousNames, "fn run() { let total = 1; }");
    }
}
