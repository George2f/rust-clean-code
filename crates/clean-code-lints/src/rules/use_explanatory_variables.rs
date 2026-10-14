use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct UseExplanatoryVariables;

impl Rule for UseExplanatoryVariables {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "use-explanatory-variables",
            clean_code_id: "G19",
            clean_code_rule: "Use Explanatory Variables",
            description: "Require complex lookups in conditions to be assigned to explanatory variables (Clean Code G19)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let minimum = ctx.options_int("minComplexLookups", 1) as usize;
        let mut visitor = Visitor {
            ctx,
            minimum,
            findings: Vec::new(),
        };
        visitor.visit_file(ctx.ast);
        visitor.findings
    }
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    minimum: usize,
    findings: Vec<Finding>,
}

struct ComplexLookups {
    count: usize,
}

fn chain_depth(expr: &syn::Expr) -> usize {
    match expr {
        syn::Expr::MethodCall(call) => 1 + chain_depth(&call.receiver),
        syn::Expr::Field(field) => 1 + chain_depth(&field.base),
        syn::Expr::Paren(paren) => chain_depth(&paren.expr),
        _ => 0,
    }
}

impl<'ast> Visit<'ast> for ComplexLookups {
    fn visit_expr_index(&mut self, node: &'ast syn::ExprIndex) {
        self.count += 1;
        syn::visit::visit_expr_index(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if chain_depth(&syn::Expr::MethodCall(node.clone())) >= 2 {
            self.count += 1;
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

impl Visitor<'_, '_> {
    fn check_condition(&mut self, condition: &syn::Expr) {
        let mut lookups = ComplexLookups { count: 0 };
        lookups.visit_expr(condition);
        let threshold = self.minimum.max(1);
        if lookups.count >= threshold {
            let message = format!(
                "condition contains {} complex lookup(s); assign them to explanatory variables (Clean Code G19)",
                lookups.count
            );
            let span = self.ctx.file.span_of(condition);
            self.findings.push(self.ctx.finding(span, message));
        }
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        self.check_condition(&node.cond);
        syn::visit::visit_expr_if(self, node);
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.check_condition(&node.cond);
        syn::visit::visit_expr_while(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_index_lookup_in_condition() {
        assert_findings(
            &UseExplanatoryVariables,
            "fn run(values: &[u8]) { if values[0] == 1 { work(); } }",
            1,
        );
    }

    #[test]
    fn flags_chained_lookup_in_condition() {
        assert_findings(
            &UseExplanatoryVariables,
            "fn run(order: Order) { if order.customer().address().len() > 0 { work(); } }",
            1,
        );
    }

    #[test]
    fn allows_simple_predicate_call() {
        assert_clean(
            &UseExplanatoryVariables,
            "fn run(values: &[u8]) { if values.is_empty() { work(); } }",
        );
    }
}
