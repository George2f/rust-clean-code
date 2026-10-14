use syn::visit::Visit;

use crate::diagnostic::{Fix, TextEdit};
use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};
use crate::source::{NodeSpan, SourceFile};

pub struct NoNegativeConditionals;

impl Rule for NoNegativeConditionals {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-negative-conditionals",
            clean_code_id: "G29",
            clean_code_rule: "Avoid Negative Conditionals",
            description: "Disallow negated conditions when both branches exist (Clean Code G29)",
            tier: Tier::Static,
            fixable: true,
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

#[derive(Clone, Copy)]
struct SwapSpans {
    condition: NodeSpan,
    inner: NodeSpan,
    then_block: NodeSpan,
    else_block: NodeSpan,
}

fn slice_of(file: &SourceFile, span: NodeSpan) -> String {
    file.text
        .get(span.byte_start..span.byte_end)
        .unwrap_or("")
        .to_string()
}

fn swap_fix(file: &SourceFile, spans: SwapSpans) -> Fix {
    let condition_text = slice_of(file, spans.inner);
    let then_text = slice_of(file, spans.then_block);
    let else_text = slice_of(file, spans.else_block);
    Fix {
        edits: vec![
            TextEdit {
                start: spans.condition.byte_start,
                end: spans.condition.byte_end,
                replacement: condition_text,
            },
            TextEdit {
                start: spans.then_block.byte_start,
                end: spans.then_block.byte_end,
                replacement: else_text,
            },
            TextEdit {
                start: spans.else_block.byte_start,
                end: spans.else_block.byte_end,
                replacement: then_text,
            },
        ],
    }
}

fn negated_condition(node: &syn::ExprIf) -> Option<&syn::ExprUnary> {
    let syn::Expr::Unary(unary) = node.cond.as_ref() else {
        return None;
    };
    matches!(unary.op, syn::UnOp::Not(_)).then_some(unary)
}

impl Visitor<'_, '_> {
    fn negated_branches_finding(&self, node: &syn::ExprIf) -> Option<Finding> {
        let unary = negated_condition(node)?;
        let (_, else_expr) = node.else_branch.as_ref()?;
        let syn::Expr::Block(else_block) = else_expr.as_ref() else {
            return None;
        };
        Some(self.branch_swap_finding(node, unary, &else_block.block))
    }

    fn branch_swap_finding(
        &self,
        node: &syn::ExprIf,
        unary: &syn::ExprUnary,
        else_block: &syn::Block,
    ) -> Finding {
        let file = self.ctx.file;
        let spans = SwapSpans {
            condition: file.span_of(node.cond.as_ref()),
            inner: file.span_of(unary.expr.as_ref()),
            then_block: file.span_of(&node.then_branch),
            else_block: file.span_of(else_block),
        };
        let message = "negated condition with both branches; invert the condition and swap the branches (Clean Code G29)".to_string();
        let finding = Finding::new(file.path.clone(), spans.condition, message);
        finding.with_fix(swap_fix(file, spans))
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        if let Some(finding) = self.negated_branches_finding(node) {
            self.findings.push(finding);
        }
        syn::visit::visit_expr_if(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_negated_condition_with_else() {
        let findings = assert_findings(
            &NoNegativeConditionals,
            "fn run(ready: bool) { if !ready { start(); } else { stop(); } }",
            1,
        );
        assert!(findings[0].fix.is_some());
    }

    #[test]
    fn allows_positive_condition() {
        assert_clean(
            &NoNegativeConditionals,
            "fn run(ready: bool) { if ready { start(); } else { stop(); } }",
        );
    }

    #[test]
    fn allows_negated_condition_without_else() {
        assert_clean(
            &NoNegativeConditionals,
            "fn run(ready: bool) { if !ready { return; } }",
        );
    }
}
