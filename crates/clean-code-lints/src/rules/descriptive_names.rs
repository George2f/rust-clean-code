use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

const DEFAULT_ALLOW: &[&str] = &["i", "j", "k", "x", "y", "z", "e", "_"];
const DEFAULT_FORBIDDEN: &[&str] = &[
    "data", "info", "stuff", "thing", "temp", "tmp", "foo", "bar", "baz", "obj", "arr", "val",
    "utils", "misc",
];

pub struct DescriptiveNames;

impl Rule for DescriptiveNames {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "descriptive-names",
            clean_code_id: "N1",
            clean_code_rule: "Choose Descriptive Names",
            description: "Disallow single-letter and vague variable names (Clean Code N1)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let allow = ctx.options_names_or("allow", DEFAULT_ALLOW);
        let forbidden = ctx.options_names_or("forbidden", DEFAULT_FORBIDDEN);
        let mut visitor = Visitor {
            ctx,
            allow,
            forbidden,
            findings: Vec::new(),
        };
        visitor.visit_file(ctx.ast);
        visitor.findings
    }
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    allow: Vec<String>,
    forbidden: Vec<String>,
    findings: Vec<Finding>,
}

impl Visitor<'_, '_> {
    fn check_name(&mut self, name: &syn::Ident) {
        let text = name.to_string();
        let message = self.issue_message(&text);
        let Some(message) = message else {
            return;
        };
        let span = self.ctx.file.span_of(name);
        self.findings.push(self.ctx.finding(span, message));
    }

    fn issue_message(&self, text: &str) -> Option<String> {
        let skipped = text.starts_with('_');
        let single_letter = text.len() == 1;
        let allowed = self.allow.iter().any(|allowed| allowed == text);
        if skipped || (single_letter && allowed) {
            return None;
        }
        if single_letter {
            return Some(format!(
                "single-letter name `{text}`; use a descriptive name (Clean Code N1)"
            ));
        }
        let forbidden = self.forbidden.iter().any(|word| word == text);
        forbidden.then(|| format!("vague name `{text}`; say what it holds or does (Clean Code N1)"))
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        self.check_name(&node.ident);
        syn::visit::visit_pat_ident(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.check_name(&node.sig.ident);
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.check_name(&node.sig.ident);
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_field(&mut self, node: &'ast syn::Field) {
        if let Some(ident) = &node.ident {
            self.check_name(ident);
        }
        syn::visit::visit_field(self, node);
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        self.check_name(&node.ident);
        syn::visit::visit_item_const(self, node);
    }

    fn visit_item_static(&mut self, node: &'ast syn::ItemStatic) {
        self.check_name(&node.ident);
        syn::visit::visit_item_static(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_vague_name() {
        let findings = assert_findings(&DescriptiveNames, "fn run() { let data = 1; }", 1);
        assert!(findings[0].message.contains("data"));
    }

    #[test]
    fn flags_single_letter_binding() {
        assert_findings(&DescriptiveNames, "fn run() { let q = 1; }", 1);
    }

    #[test]
    fn allows_allowed_single_letters() {
        assert_clean(&DescriptiveNames, "fn run() { let x = 1; }");
    }

    #[test]
    fn allows_descriptive_names() {
        assert_clean(&DescriptiveNames, "fn run() { let user_count = 1; }");
    }

    #[test]
    fn flags_vague_function_name() {
        assert_findings(&DescriptiveNames, "fn tmp() {}", 1);
    }
}
