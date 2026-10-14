use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

const DEFAULT_PREFIXES: &[&str] = &["get", "is", "has", "find"];

pub struct NamesDescribeSideEffects;

impl Rule for NamesDescribeSideEffects {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "names-describe-side-effects",
            clean_code_id: "N7",
            clean_code_rule: "Names Should Describe Side Effects",
            description: "Require names that describe side effects (Clean Code N7)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let prefixes = ctx.options_names_or("prefixes", DEFAULT_PREFIXES);
        let mut visitor = Visitor {
            ctx,
            prefixes,
            findings: Vec::new(),
        };
        visitor.visit_file(ctx.ast);
        visitor.findings
    }
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    prefixes: Vec<String>,
    findings: Vec<Finding>,
}

fn has_query_prefix(name: &str, prefixes: &[String]) -> bool {
    matches!(name.split('_').next(), Some(first) if prefixes.iter().any(|prefix| prefix == first))
}

fn has_side_effect(signature: &syn::Signature) -> bool {
    signature.inputs.iter().any(input_has_side_effect)
}

fn input_has_side_effect(input: &syn::FnArg) -> bool {
    match input {
        syn::FnArg::Receiver(receiver) => {
            receiver.reference.is_some() && receiver.mutability.is_some()
        }
        syn::FnArg::Typed(parameter) => matches!(
            parameter.ty.as_ref(),
            syn::Type::Reference(reference) if reference.mutability.is_some()
        ),
    }
}

impl Visitor<'_, '_> {
    fn check_fn(&mut self, signature: &syn::Signature) {
        let name = signature.ident.to_string();
        if !has_query_prefix(&name, &self.prefixes) {
            return;
        }
        if !has_side_effect(signature) {
            return;
        }
        let message = format!(
            "`{name}` sounds like a query but mutates state; rename it to describe the side effect (Clean Code N7)"
        );
        let span = self.ctx.file.span_of(&signature.ident);
        self.findings.push(self.ctx.finding(span, message));
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.check_fn(&node.sig);
        syn::visit::visit_impl_item_fn(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_query_name_with_mutation() {
        assert_findings(
            &NamesDescribeSideEffects,
            "struct Store; impl Store { fn get_value(&mut self) -> u8 { 0 } }",
            1,
        );
    }

    #[test]
    fn allows_query_name_without_mutation() {
        assert_clean(
            &NamesDescribeSideEffects,
            "struct Store; impl Store { fn get_value(&self) -> u8 { 0 } }",
        );
    }

    #[test]
    fn flags_mut_output_parameter() {
        assert_findings(
            &NamesDescribeSideEffects,
            "struct Store; impl Store { fn find_item(&self, output: &mut Vec<u8>) {} }",
            1,
        );
    }
}
