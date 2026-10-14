use syn::visit::Visit;

use crate::ast_util::type_last_segment;
use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct PreferExceptionsOverErrorCodes;

impl Rule for PreferExceptionsOverErrorCodes {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "prefer-exceptions-over-error-codes",
            clean_code_id: "Ch3",
            clean_code_rule: "Prefer Exceptions to Returning Error Codes",
            description: "Disallow boolean status returns; prefer Result (Clean Code Ch3)",
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

struct FalseScan {
    found: bool,
}

impl<'ast> Visit<'ast> for FalseScan {
    fn visit_expr_lit(&mut self, node: &'ast syn::ExprLit) {
        if matches!(node.lit, syn::Lit::Bool(ref literal) if !literal.value) {
            self.found = true;
        }
        syn::visit::visit_expr_lit(self, node);
    }
}

impl Visitor<'_, '_> {
    fn check_fn(&mut self, signature: &syn::Signature, block: &syn::Block) {
        let syn::ReturnType::Type(_, ty) = &signature.output else {
            return;
        };
        if type_last_segment(ty).as_deref() != Some("bool") {
            return;
        }
        let mut scan = FalseScan { found: false };
        scan.visit_block(block);
        if scan.found {
            let message = format!(
                "function `{}` returns bool as a status; return Result and let the error carry information (Clean Code Ch3)",
                signature.ident
            );
            let span = self.ctx.file.span_of(ty);
            self.findings.push(self.ctx.finding(span, message));
        }
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.check_fn(&node.sig, &node.block);
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.check_fn(&node.sig, &node.block);
        syn::visit::visit_impl_item_fn(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_bool_status_return() {
        assert_findings(
            &PreferExceptionsOverErrorCodes,
            "fn save(valid: bool) -> bool { if valid { true } else { false } }",
            1,
        );
    }

    #[test]
    fn allows_result_return() {
        assert_clean(
            &PreferExceptionsOverErrorCodes,
            "fn save() -> Result<(), Error> { Ok(()) }",
        );
    }

    #[test]
    fn allows_pure_predicate() {
        assert_clean(
            &PreferExceptionsOverErrorCodes,
            "fn is_positive(value: i64) -> bool { value > 0 }",
        );
    }
}
