use syn::visit::Visit;

use crate::ast_util::attr_last_segment;
use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct OneAssertionPerTest;

impl Rule for OneAssertionPerTest {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "one-assertion-per-test",
            clean_code_id: "Ch9",
            clean_code_rule: "One Assert per Test",
            description: "Enforce a single assertion per test (Clean Code Ch9)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let max = ctx.options_int("maxAssertions", 1) as usize;
        let mut visitor = Visitor {
            ctx,
            max,
            findings: Vec::new(),
        };
        visitor.visit_file(ctx.ast);
        visitor.findings
    }
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    max: usize,
    findings: Vec<Finding>,
}

impl Visitor<'_, '_> {
    fn check_test(
        &mut self,
        signature: &syn::Signature,
        attrs: &[syn::Attribute],
        block: &syn::Block,
    ) {
        let is_test = attrs
            .iter()
            .any(|attr| attr_last_segment(attr).as_deref() == Some("test"));
        if !is_test {
            return;
        }
        let mut scan = AssertScan { count: 0 };
        scan.visit_block(block);
        if scan.count > self.max {
            let message = format!(
                "test makes {} assertions (max {}); keep one assertion per test (Clean Code Ch9)",
                scan.count, self.max
            );
            let span = self.ctx.file.span_of(&signature.ident);
            self.findings.push(self.ctx.finding(span, message));
        }
    }
}

struct AssertScan {
    count: usize,
}

impl AssertScan {
    fn record(&mut self, mac: &syn::Macro) {
        let name = mac
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default();
        if name.starts_with("assert") {
            self.count += 1;
        }
    }
}

impl<'ast> Visit<'ast> for AssertScan {
    fn visit_expr_macro(&mut self, node: &'ast syn::ExprMacro) {
        self.record(&node.mac);
        syn::visit::visit_expr_macro(self, node);
    }

    fn visit_stmt_macro(&mut self, node: &'ast syn::StmtMacro) {
        self.record(&node.mac);
        syn::visit::visit_stmt_macro(self, node);
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.check_test(&node.sig, &node.attrs, &node.block);
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.check_test(&node.sig, &node.attrs, &node.block);
        syn::visit::visit_impl_item_fn(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_two_assertions() {
        assert_findings(
            &OneAssertionPerTest,
            "#[test]\nfn t() { assert_eq!(1, 1); assert_eq!(2, 2); }",
            1,
        );
    }

    #[test]
    fn allows_one_assertion() {
        assert_clean(
            &OneAssertionPerTest,
            "#[test]\nfn t() { assert_eq!(1, 1); }",
        );
    }

    #[test]
    fn ignores_non_test_helpers() {
        assert_clean(
            &OneAssertionPerTest,
            "fn helper() { assert_eq!(1, 1); assert_eq!(2, 2); }",
        );
    }
}
