use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct EncapsulateConditionals;

impl Rule for EncapsulateConditionals {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "encapsulate-conditionals",
            clean_code_id: "G28",
            clean_code_rule: "Encapsulate Conditionals",
            description: "Disallow complex boolean conditions; extract them into well-named functions (Clean Code G28)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let max = ctx.options_int("maxLogicalOperators", 2) as usize;
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

struct LogicalOps {
    count: usize,
}

impl<'ast> Visit<'ast> for LogicalOps {
    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if matches!(node.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) {
            self.count += 1;
        }
        syn::visit::visit_expr_binary(self, node);
    }
}

impl Visitor<'_, '_> {
    fn check_condition(&mut self, condition: &syn::Expr) {
        let mut counter = LogicalOps { count: 0 };
        counter.visit_expr(condition);
        if counter.count > self.max {
            let message = format!(
                "condition uses {} logical operators (max {}); extract it into a named function (Clean Code G28)",
                counter.count, self.max
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
    fn flags_three_operators() {
        assert_findings(
            &EncapsulateConditionals,
            "fn run(a: bool, b: bool, c: bool, d: bool) { if a && b && c && d { work(); } }",
            1,
        );
    }

    #[test]
    fn allows_two_operators() {
        assert_clean(
            &EncapsulateConditionals,
            "fn run(a: bool, b: bool, c: bool) { if a && b && c { work(); } }",
        );
    }
}
