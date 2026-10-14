use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct NoOutputArguments;

impl Rule for NoOutputArguments {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-output-arguments",
            clean_code_id: "F2",
            clean_code_rule: "Output Arguments",
            description: "Disallow function parameters that the function mutates (Clean Code F2)",
            tier: Tier::Heuristic,
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

fn takes_mut_reference(parameter: &syn::PatType) -> bool {
    matches!(
        parameter.ty.as_ref(),
        syn::Type::Reference(reference) if reference.mutability.is_some()
    )
}

impl Visitor<'_, '_> {
    fn check_signature(&mut self, signature: &syn::Signature) {
        for input in &signature.inputs {
            let syn::FnArg::Typed(parameter) = input else {
                continue;
            };
            if !takes_mut_reference(parameter) {
                continue;
            }
            let span = self.ctx.file.span_of(input);
            self.findings.push(self.ctx.finding(
                span,
                "output argument (`&mut` parameter); return the value instead (Clean Code F2)",
            ));
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
    fn flags_mut_parameter() {
        assert_findings(&NoOutputArguments, "fn fill(output: &mut Vec<u8>) {}", 1);
    }

    #[test]
    fn allows_shared_reference() {
        assert_clean(&NoOutputArguments, "fn total(values: &[u8]) -> u32 { 0 }");
    }

    #[test]
    fn ignores_mut_self() {
        assert_clean(
            &NoOutputArguments,
            "struct S; impl S { fn push(&mut self, value: u8) {} }",
        );
    }
}
