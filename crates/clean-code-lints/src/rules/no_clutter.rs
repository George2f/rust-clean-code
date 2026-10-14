use quote::ToTokens;
use syn::visit::Visit;

use crate::ast_util::path_last_segment;
use crate::diagnostic::{Fix, TextEdit};
use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct NoClutter;

impl Rule for NoClutter {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-clutter",
            clean_code_id: "G12",
            clean_code_rule: "Clutter",
            description: "Disallow clutter such as leftover debug macros (Clean Code G12)",
            tier: Tier::Heuristic,
            fixable: true,
            has_suggestions: true,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let mut visitor = Visitor {
            ctx,
            findings: Vec::new(),
        };
        visitor.visit_file(ctx.ast);
        visitor.findings
    }
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    findings: Vec<Finding>,
}

impl Visitor<'_, '_> {
    fn check_macro<T: ToTokens + ?Sized>(&mut self, mac: &syn::Macro, span_node: &T, suffix: &str) {
        let name = path_last_segment(&mac.path);
        if name.as_deref() != Some("dbg") {
            return;
        }
        let span = self.ctx.file.span_of(span_node);
        let mut finding = self
            .ctx
            .finding(
                span,
                "debug clutter `dbg!`; remove it before committing (Clean Code G12)",
            )
            .with_help("unwrap the expression instead of printing it");
        let tokens = mac.tokens.clone();
        if let Ok(inner) = syn::parse2::<syn::Expr>(tokens) {
            finding = finding.with_fix(Fix {
                edits: vec![TextEdit {
                    start: span.byte_start,
                    end: span.byte_end,
                    replacement: format!("{}{suffix}", inner.to_token_stream()),
                }],
            });
        }
        self.findings.push(finding);
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_expr_macro(&mut self, node: &'ast syn::ExprMacro) {
        self.check_macro(&node.mac, node, "");
        syn::visit::visit_expr_macro(self, node);
    }

    fn visit_stmt_macro(&mut self, node: &'ast syn::StmtMacro) {
        let semicolon = node.semi_token.is_some();
        let suffix = if semicolon { ";" } else { "" };
        self.check_macro(&node.mac, node, suffix);
        syn::visit::visit_stmt_macro(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_dbg_macro() {
        let findings = assert_findings(&NoClutter, "fn run() { dbg!(1); }", 1);
        assert!(findings[0].fix.is_some());
    }

    #[test]
    fn statement_fix_keeps_the_semicolon() {
        let findings = assert_findings(&NoClutter, "fn run() { dbg!(1); }", 1);
        let fix = findings[0]
            .fix
            .as_ref()
            .unwrap_or_else(|| panic!("expected a fix"));
        assert_eq!(fix.edits[0].replacement, "1;");
    }

    #[test]
    fn expression_fix_unwraps_without_semicolon() {
        let findings = assert_findings(&NoClutter, "fn run() -> i32 { dbg!(1) }", 1);
        let fix = findings[0]
            .fix
            .as_ref()
            .unwrap_or_else(|| panic!("expected a fix"));
        assert_eq!(fix.edits[0].replacement, "1");
    }

    #[test]
    fn allows_println() {
        assert_clean(&NoClutter, "fn run() { println!(\"done\"); }");
    }
}
