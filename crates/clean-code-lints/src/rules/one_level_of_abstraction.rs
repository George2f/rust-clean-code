use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

const LOW_LEVEL_METHODS: &[&str] = &[
    "chars",
    "bytes",
    "split",
    "split_whitespace",
    "split_once",
    "lines",
    "trim",
    "trim_start",
    "trim_end",
    "to_lowercase",
    "to_uppercase",
    "to_string",
    "to_owned",
    "parse",
    "push",
    "pop",
    "insert",
    "remove",
    "retain",
    "replace",
    "get",
    "find",
    "starts_with",
    "ends_with",
    "contains",
    "index",
    "join",
    "as_bytes",
    "len",
];

const DEFAULT_MIN_LOW_LEVEL_OPS: i64 = 3;
const DEFAULT_MIN_HIGH_LEVEL_CALLS: i64 = 2;

pub struct OneLevelOfAbstraction;

impl Rule for OneLevelOfAbstraction {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "one-level-of-abstraction",
            clean_code_id: "G34",
            clean_code_rule: "Functions Should Descend Only One Level of Abstraction",
            description: "Keep every function at a single level of abstraction (Clean Code G34)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let min_low = ctx.options_int("minLowLevelOps", DEFAULT_MIN_LOW_LEVEL_OPS) as usize;
        let min_high = ctx.options_int("minHighLevelCalls", DEFAULT_MIN_HIGH_LEVEL_CALLS) as usize;
        let mut visitor = Visitor {
            ctx,
            min_low,
            min_high,
            findings: Vec::new(),
        };
        visitor.visit_file(ctx.ast);
        visitor.findings
    }
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    min_low: usize,
    min_high: usize,
    findings: Vec<Finding>,
}

struct LevelScan {
    low: usize,
    high: usize,
}

impl<'ast> Visit<'ast> for LevelScan {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        self.high += 1;
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let name = node.method.to_string();
        if LOW_LEVEL_METHODS.contains(&name.as_str()) {
            self.low += 1;
        } else {
            self.high += 1;
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        let arithmetic = matches!(
            node.op,
            syn::BinOp::Add(_)
                | syn::BinOp::Sub(_)
                | syn::BinOp::Mul(_)
                | syn::BinOp::Div(_)
                | syn::BinOp::Rem(_)
                | syn::BinOp::BitAnd(_)
                | syn::BinOp::BitOr(_)
                | syn::BinOp::BitXor(_)
                | syn::BinOp::Shl(_)
                | syn::BinOp::Shr(_)
        );
        let has_literal = matches!(node.left.as_ref(), syn::Expr::Lit(_))
            || matches!(node.right.as_ref(), syn::Expr::Lit(_));
        if arithmetic && has_literal {
            self.low += 1;
        }
        syn::visit::visit_expr_binary(self, node);
    }

    fn visit_expr_closure(&mut self, _node: &'ast syn::ExprClosure) {}

    fn visit_item(&mut self, _node: &'ast syn::Item) {}
}

impl Visitor<'_, '_> {
    fn check_fn(&mut self, signature: &syn::Signature, block: &syn::Block) {
        let mut scan = LevelScan { low: 0, high: 0 };
        scan.visit_block(block);
        if scan.low >= self.min_low && scan.high >= self.min_high {
            let message = format!(
                "function `{}` mixes {} low-level operations with {} high-level calls; split it by abstraction level (Clean Code G34)",
                signature.ident, scan.low, scan.high
            );
            let span = self.ctx.file.span_of(&signature.ident);
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
    fn flags_mixed_abstraction_levels() {
        let source = "fn transform(raw: &str) -> String {\n    let cleaned = normalize(raw);\n    let trimmed = cleaned.trim().to_string();\n    let upper = trimmed.to_uppercase().replace(\"A\", \"B\");\n    let mut parts: Vec<&str> = upper.split(',').collect();\n    parts.push(\"tail\");\n    parts.join(\"|\")\n}\n";
        assert_findings(&OneLevelOfAbstraction, source, 1);
    }

    #[test]
    fn allows_single_level() {
        assert_clean(
            &OneLevelOfAbstraction,
            "fn transform(raw: &str) -> String { normalize(raw) }",
        );
    }
}
