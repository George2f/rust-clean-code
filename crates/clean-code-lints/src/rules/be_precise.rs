use syn::visit::Visit;

use crate::ast_util::type_last_segment;
use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct BePrecise;

impl Rule for BePrecise {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "be-precise",
            clean_code_id: "G26",
            clean_code_rule: "Be Precise",
            description: "Disallow imprecise operations such as float equality (Clean Code G26)",
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

fn is_floatish(expr: &syn::Expr) -> bool {
    matches!(expr, syn::Expr::Lit(lit) if matches!(lit.lit, syn::Lit::Float(_)))
        || matches!(expr, syn::Expr::Cast(cast) if matches!(type_last_segment(&cast.ty).as_deref(), Some("f32" | "f64")))
        || matches!(expr, syn::Expr::Paren(paren) if is_floatish(&paren.expr))
        || matches!(expr, syn::Expr::Unary(unary) if is_floatish(&unary.expr))
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        let equality = matches!(node.op, syn::BinOp::Eq(_) | syn::BinOp::Ne(_));
        if equality && (is_floatish(&node.left) || is_floatish(&node.right)) {
            let span = self.ctx.file.span_of(node);
            self.findings.push(self.ctx.finding(
                span,
                "imprecise float equality; compare with an epsilon tolerance (Clean Code G26)",
            ));
        }
        syn::visit::visit_expr_binary(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_float_literal_equality() {
        assert_findings(&BePrecise, "fn run(a: f64) -> bool { a == 1.5 }", 1);
    }

    #[test]
    fn flags_cast_equality() {
        assert_findings(
            &BePrecise,
            "fn run(a: f64, b: f64) -> bool { (a as f32) == (b as f32) }",
            1,
        );
    }

    #[test]
    fn allows_ordering_comparison() {
        assert_clean(&BePrecise, "fn run(a: f64) -> bool { a > 1.5 }");
    }

    #[test]
    fn allows_integer_equality() {
        assert_clean(&BePrecise, "fn run(a: u8) -> bool { a == 3 }");
    }
}
