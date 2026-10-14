use quote::ToTokens;
use syn::visit::Visit;

use crate::ast_util::has_cfg_test;
use crate::rule::{Finding, Flag, Rule, RuleContext, RuleMeta, Tier};
use crate::source::NodeSpan;

const DEFAULT_IGNORE: &[i64] = &[-1, 0, 1, 2];

pub struct NoMagicNumbers;

impl Rule for NoMagicNumbers {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-magic-numbers",
            clean_code_id: "G25",
            clean_code_rule: "Replace Magic Numbers with Named Constants",
            description:
                "Disallow magic numbers; replace them with named constants (Clean Code G25)",
            tier: Tier::Static,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let (ignore, ignore_enums) = read_options(ctx);
        let ast = ctx.ast;
        let mut visitor = Visitor {
            ctx,
            ignore,
            ignore_enums,
            const_depth: 0,
            enum_depth: 0,
            findings: Vec::new(),
        };
        visitor.visit_file(ast);
        visitor.findings
    }
}

fn read_options(ctx: &RuleContext<'_>) -> (Vec<i64>, Flag) {
    let configured = ctx.options.get("ignore");
    let ignore = match configured {
        Some(_) => ctx.options_ints("ignore"),
        None => DEFAULT_IGNORE.to_vec(),
    };
    let ignore_enums = ctx.options_flag("ignoreEnums", Flag::On);
    (ignore, ignore_enums)
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    ignore: Vec<i64>,
    ignore_enums: Flag,
    const_depth: usize,
    enum_depth: usize,
    findings: Vec<Finding>,
}

fn literal_value(lit: &syn::Lit) -> Option<f64> {
    match lit {
        syn::Lit::Int(int) => int.base10_parse::<f64>().ok(),
        syn::Lit::Float(float) => float.base10_parse::<f64>().ok(),
        _ => None,
    }
}

impl Visitor<'_, '_> {
    fn check_value(&mut self, value: f64, span: NodeSpan, text: &str) {
        let in_const = self.const_depth > 0;
        let in_enum = self.ignore_enums.is_on() && self.enum_depth > 0;
        if in_const || in_enum || self.is_ignored(value) {
            return;
        }
        let message = format!("magic number {text}; extract a named constant (Clean Code G25)");
        self.findings.push(self.ctx.finding(span, message));
    }

    fn is_ignored(&self, value: f64) -> bool {
        let ignored = self
            .ignore
            .iter()
            .any(|ignored| (value - *ignored as f64).abs() < f64::EPSILON);
        ignored
    }

    fn literal_finding(&mut self, literal: &syn::Lit, text: &str, span: NodeSpan) {
        if let Some(value) = literal_value(literal) {
            self.check_value(value, span, text);
        }
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        syn::visit::visit_item_mod(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        self.const_depth += 1;
        syn::visit::visit_item_const(self, node);
        self.const_depth -= 1;
    }

    fn visit_item_static(&mut self, node: &'ast syn::ItemStatic) {
        self.const_depth += 1;
        syn::visit::visit_item_static(self, node);
        self.const_depth -= 1;
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        self.enum_depth += 1;
        syn::visit::visit_item_enum(self, node);
        self.enum_depth -= 1;
    }

    fn visit_expr_unary(&mut self, node: &'ast syn::ExprUnary) {
        let negated = matches!(node.op, syn::UnOp::Neg(_));
        if negated {
            let operand = node.expr.as_ref();
            if let syn::Expr::Lit(expr_lit) = operand {
                if let Some(value) = literal_value(&expr_lit.lit) {
                    let text = node.to_token_stream().to_string();
                    let span = self.ctx.file.span_of(node);
                    self.check_value(-value, span, &text);
                    return;
                }
            }
        }
        syn::visit::visit_expr_unary(self, node);
    }

    fn visit_expr_lit(&mut self, node: &'ast syn::ExprLit) {
        let text = node.to_token_stream().to_string();
        let span = self.ctx.file.span_of(node);
        self.literal_finding(&node.lit, &text, span);
        syn::visit::visit_expr_lit(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_magic_number() {
        let findings = assert_findings(&NoMagicNumbers, "fn run() -> i32 { 42 }", 1);
        assert!(findings[0].message.contains("42"));
    }

    #[test]
    fn allows_named_constant() {
        assert_clean(
            &NoMagicNumbers,
            "const LIMIT: i32 = 42;\nfn run() -> i32 { LIMIT }",
        );
    }

    #[test]
    fn allows_default_ignored_values() {
        assert_clean(&NoMagicNumbers, "fn run() -> i32 { 0 + 1 + 2 - 1 }");
    }

    #[test]
    fn ignores_enum_discriminants() {
        assert_clean(&NoMagicNumbers, "enum Code { NotFound = 404 }");
    }

    #[test]
    fn flags_negative_magic_number() {
        assert_findings(&NoMagicNumbers, "fn run() -> i32 { -42 }", 1);
    }
}
