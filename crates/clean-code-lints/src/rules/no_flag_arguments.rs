use syn::visit::Visit;

use crate::ast_util::type_last_segment;
use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct NoFlagArguments;

impl Rule for NoFlagArguments {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-flag-arguments",
            clean_code_id: "F3",
            clean_code_rule: "Flag Arguments",
            description: "Disallow boolean flag parameters that make a function do more than one thing (Clean Code F3)",
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
    fn check_signature(&mut self, signature: &syn::Signature) {
        for input in &signature.inputs {
            let syn::FnArg::Typed(parameter) = input else {
                continue;
            };
            if type_last_segment(&parameter.ty).as_deref() == Some("bool") {
                let span = self.ctx.file.span_of(input);
                self.findings.push(self.ctx.finding(
                    span,
                    "boolean flag argument; split the function or pass a dedicated type (Clean Code F3)",
                ));
            }
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
    fn flags_boolean_parameter() {
        assert_findings(&NoFlagArguments, "fn render(with_border: bool) {}", 1);
    }

    #[test]
    fn allows_other_parameter_types() {
        assert_clean(&NoFlagArguments, "fn render(mode: Mode) {}");
    }

    #[test]
    fn flags_boolean_parameter_on_method() {
        assert_findings(
            &NoFlagArguments,
            "struct S; impl S { fn go(&self, fast: bool) {} }",
            1,
        );
    }
}
