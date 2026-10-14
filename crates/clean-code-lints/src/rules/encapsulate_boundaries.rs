use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct EncapsulateBoundaries;

impl Rule for EncapsulateBoundaries {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "encapsulate-boundaries",
            clean_code_id: "G33",
            clean_code_rule: "Encapsulate Boundary Conditions",
            description: "Disallow inline offset arithmetic on boundary members such as length (Clean Code G33)",
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

fn is_boundary_call(expr: &syn::Expr) -> bool {
    matches!(expr, syn::Expr::Paren(paren) if is_boundary_call(&paren.expr))
        || matches!(expr, syn::Expr::MethodCall(call) if is_boundary_method(call))
}

fn is_boundary_method(call: &syn::ExprMethodCall) -> bool {
    matches!(
        call.method.to_string().as_str(),
        "len" | "count" | "capacity"
    )
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        let is_offset = matches!(node.op, syn::BinOp::Add(_) | syn::BinOp::Sub(_));
        if is_offset && (is_boundary_call(&node.left) || is_boundary_call(&node.right)) {
            let span = self.ctx.file.span_of(node);
            self.findings.push(self.ctx.finding(
                span,
                "inline boundary arithmetic on a length; encapsulate the boundary condition (Clean Code G33)",
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
    fn flags_length_minus_one() {
        assert_findings(
            &EncapsulateBoundaries,
            "fn run(values: &[u8]) -> usize { values.len() - 1 }",
            1,
        );
    }

    #[test]
    fn flags_length_offset_from_left() {
        assert_findings(
            &EncapsulateBoundaries,
            "fn run(values: &[u8], offset: usize) -> usize { offset + values.len() }",
            1,
        );
    }

    #[test]
    fn allows_last() {
        assert_clean(
            &EncapsulateBoundaries,
            "fn run(values: &[u8]) -> Option<&u8> { values.last() }",
        );
    }
}
