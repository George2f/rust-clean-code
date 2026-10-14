use syn::visit::Visit;

use crate::ast_util::attr_last_segment;
use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct NoSkippedTests;

impl Rule for NoSkippedTests {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-skipped-tests",
            clean_code_id: "T3",
            clean_code_rule: "Don't Skip Trivial Tests",
            description: "Disallow skipped tests (Clean Code T3)",
            tier: Tier::Static,
            fixable: false,
            has_suggestions: false,
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
    fn check_attrs(&mut self, attrs: &[syn::Attribute]) {
        for attr in attrs {
            if attr_last_segment(attr).as_deref() == Some("ignore") {
                let span = self.ctx.file.span_of(attr);
                self.findings.push(self.ctx.finding(
                    span,
                    "skipped test; fix the test or delete it (Clean Code T3)",
                ));
            }
        }
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.check_attrs(&node.attrs);
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.check_attrs(&node.attrs);
        syn::visit::visit_impl_item_fn(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_ignored_test() {
        assert_findings(&NoSkippedTests, "#[test]\n#[ignore]\nfn slow() {}", 1);
    }

    #[test]
    fn allows_running_test() {
        assert_clean(&NoSkippedTests, "#[test]\nfn fast() {}");
    }
}
