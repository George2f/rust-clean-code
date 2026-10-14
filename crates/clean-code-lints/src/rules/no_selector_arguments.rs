use syn::visit::Visit;

use crate::ast_util::type_last_segment;
use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct NoSelectorArguments;

impl Rule for NoSelectorArguments {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-selector-arguments",
            clean_code_id: "G15",
            clean_code_rule: "Selector Arguments",
            description: "Disallow arguments used only to select behavior; split into separate functions (Clean Code G15)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let minimum = ctx.options_int("minComparisons", 2) as usize;
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

struct UsageScan<'a> {
    name: &'a str,
    selector: usize,
    other: usize,
}

fn is_plain_path(expr: &syn::Expr, name: &str) -> bool {
    matches!(expr, syn::Expr::Paren(paren) if is_plain_path(&paren.expr, name))
        || matches!(expr, syn::Expr::Path(path) if simple_path(path, name))
}

fn simple_path(path: &syn::ExprPath, name: &str) -> bool {
    path.qself.is_none() && path.path.segments.len() == 1 && path.path.segments[0].ident == name
}

impl<'ast> Visit<'ast> for UsageScan<'_> {
    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if matches!(node.op, syn::BinOp::Eq(_) | syn::BinOp::Ne(_)) {
            let left = is_plain_path(&node.left, self.name);
            let right = is_plain_path(&node.right, self.name);
            if left || right {
                self.selector += 1;
                if !left {
                    self.visit_expr(&node.left);
                }
                if !right {
                    self.visit_expr(&node.right);
                }
                return;
            }
        }
        syn::visit::visit_expr_binary(self, node);
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        if is_plain_path(&node.cond, self.name) {
            self.selector += 1;
        } else {
            self.visit_expr(&node.cond);
        }
        self.visit_block(&node.then_branch);
        if let Some((_, else_expr)) = &node.else_branch {
            self.visit_expr(else_expr);
        }
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        if is_plain_path(&node.expr, self.name) {
            self.selector += 1;
        } else {
            self.visit_expr(&node.expr);
        }
        for arm in &node.arms {
            self.visit_pat(&arm.pat);
            if let Some((_, guard)) = &arm.guard {
                self.visit_expr(guard);
            }
            self.visit_expr(&arm.body);
        }
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if is_plain_path(&syn::Expr::Path(node.clone()), self.name) {
            self.other += 1;
        }
        syn::visit::visit_expr_path(self, node);
    }
}

impl Visitor<'_, '_> {
    fn check_fn(&mut self, signature: &syn::Signature, block: &syn::Block) {
        for input in &signature.inputs {
            let syn::FnArg::Typed(parameter) = input else {
                continue;
            };
            if type_last_segment(&parameter.ty).as_deref() == Some("bool") {
                continue;
            }
            let syn::Pat::Ident(pattern) = parameter.pat.as_ref() else {
                continue;
            };
            let name = pattern.ident.to_string();
            let mut scan = UsageScan {
                name: &name,
                selector: 0,
                other: 0,
            };
            scan.visit_block(block);
            if scan.selector >= self.minimum && scan.other == 0 {
                let message = format!(
                    "parameter `{name}` is only compared to select behavior; split the function (Clean Code G15)"
                );
                let span = self.ctx.file.span_of(input);
                self.findings.push(self.ctx.finding(span, message));
            }
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
    fn flags_parameter_used_only_for_comparisons() {
        assert_findings(
            &NoSelectorArguments,
            "fn apply(kind: Kind) { if kind == Kind::Fast { fast(); } if kind == Kind::Slow { slow(); } }",
            1,
        );
    }

    #[test]
    fn allows_parameter_with_other_uses() {
        assert_clean(
            &NoSelectorArguments,
            "fn apply(kind: Kind) { log(kind); if kind == Kind::Fast { fast(); } }",
        );
    }

    #[test]
    fn skips_boolean_flags() {
        assert_clean(
            &NoSelectorArguments,
            "fn apply(fast: bool) { if fast == true { go(); } }",
        );
    }
}
