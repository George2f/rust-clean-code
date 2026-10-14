use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

const DEFAULT_MAX: i64 = 3;

pub struct MaxArguments;

impl Rule for MaxArguments {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "max-arguments",
            clean_code_id: "F1",
            clean_code_rule: "Too Many Arguments",
            description: "Enforce a maximum number of function parameters (Clean Code F1)",
            tier: Tier::Static,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let max = ctx.options_int("max", DEFAULT_MAX) as usize;
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
    fn check_signature(&mut self, signature: &syn::Signature) {
        let count = signature
            .inputs
            .iter()
            .filter(|input| matches!(input, syn::FnArg::Typed(_)))
            .count();
        if count > self.max {
            let message = format!(
                "function has {count} arguments (max {}; Clean Code F1)",
                self.max
            );
            let span = self.ctx.file.span_of(&signature.ident);
            self.findings.push(self.ctx.finding(span, message));
        }
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.check_signature(&node.sig);
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.check_signature(&node.sig);
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        self.check_signature(&node.sig);
        syn::visit::visit_trait_item_fn(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_more_than_three_arguments() {
        let findings = assert_findings(
            &MaxArguments,
            "fn combine(a: u8, b: u8, c: u8, d: u8) {}",
            1,
        );
        assert!(findings[0].message.contains("4 arguments"));
    }

    #[test]
    fn allows_three_arguments() {
        assert_clean(&MaxArguments, "fn combine(a: u8, b: u8, c: u8) {}");
    }

    #[test]
    fn ignores_receiver() {
        assert_clean(
            &MaxArguments,
            "struct S; impl S { fn go(&self, a: u8, b: u8, c: u8) {} }",
        );
    }
}
