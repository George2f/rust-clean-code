use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct NoObscuredIntent;

impl Rule for NoObscuredIntent {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-obscured-intent",
            clean_code_id: "G16",
            clean_code_rule: "Obscured Intent",
            description:
                "Disallow obscured intent such as nested conditional expressions (Clean Code G16)",
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

fn block_tail_is_if(block: &syn::Block) -> bool {
    matches!(block.stmts.last(), Some(syn::Stmt::Expr(syn::Expr::If(inner), None)) if inner.else_branch.is_some())
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        let nested_then = block_tail_is_if(&node.then_branch);
        let nested_else = match &node.else_branch {
            Some((_, else_expr)) => match else_expr.as_ref() {
                syn::Expr::Block(else_block) => block_tail_is_if(&else_block.block),
                _ => false,
            },
            None => false,
        };
        if nested_then || nested_else {
            let span = self.ctx.file.span_of(node);
            self.findings.push(self.ctx.finding(
                span,
                "conditional expression nested inside another; extract it or use a match (Clean Code G16)",
            ));
        }
        syn::visit::visit_expr_if(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_nested_conditional_expression() {
        assert_findings(
            &NoObscuredIntent,
            "fn run(a: bool, b: bool) -> i32 { if a { if b { 1 } else { 2 } } else { 3 } }",
            1,
        );
    }

    #[test]
    fn allows_else_if_chain() {
        assert_clean(
            &NoObscuredIntent,
            "fn run(a: bool, b: bool) -> i32 { if a { 1 } else if b { 2 } else { 3 } }",
        );
    }
}
