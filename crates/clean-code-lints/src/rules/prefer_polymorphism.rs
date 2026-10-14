use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

const DEFAULT_MAX_SWITCH_CASES: i64 = 4;
const DEFAULT_MAX_IF_ELSE_BRANCHES: i64 = 5;

pub struct PreferPolymorphism;

impl Rule for PreferPolymorphism {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "prefer-polymorphism",
            clean_code_id: "G23",
            clean_code_rule: "Prefer Polymorphism to If/Else or Switch/Case",
            description: "Prefer polymorphism over long switch statements and if/else chains (Clean Code G23)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let max_cases = ctx.options_int("maxSwitchCases", DEFAULT_MAX_SWITCH_CASES) as usize;
        let max_branches =
            ctx.options_int("maxIfElseBranches", DEFAULT_MAX_IF_ELSE_BRANCHES) as usize;
        let mut visitor = Visitor {
            ctx,
            max_cases,
            max_branches,
            findings: Vec::new(),
        };
        visitor.visit_file(ctx.ast);
        visitor.findings
    }
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    max_cases: usize,
    max_branches: usize,
    findings: Vec<Finding>,
}

fn if_chain_length(node: &syn::ExprIf) -> usize {
    match &node.else_branch {
        Some((_, else_expr)) => match else_expr.as_ref() {
            syn::Expr::If(inner) => 1 + if_chain_length(inner),
            _ => 1,
        },
        None => 1,
    }
}

fn is_value_pattern(pattern: &syn::Pat) -> bool {
    matches!(
        pattern,
        syn::Pat::Lit(_)
            | syn::Pat::Path(_)
            | syn::Pat::Wild(_)
            | syn::Pat::Range(_)
            | syn::Pat::Const(_)
    ) || matches!(pattern, syn::Pat::Ident(ident) if ident.subpat.is_none())
        || matches!(pattern, syn::Pat::Reference(reference) if is_value_pattern(&reference.pat))
        || matches!(pattern, syn::Pat::Or(or) if or.cases.iter().all(is_value_pattern))
}

fn is_switch_like(node: &syn::ExprMatch) -> bool {
    node.arms.iter().all(|arm| is_value_pattern(&arm.pat))
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        let too_many_arms = node.arms.len() > self.max_cases;
        let switch_like = is_switch_like(node);
        if too_many_arms && switch_like {
            let message = format!(
                "match has {} arms (max {}); prefer an enum method or trait object (Clean Code G23)",
                node.arms.len(),
                self.max_cases
            );
            let span = self.ctx.file.span_of(node);
            self.findings.push(self.ctx.finding(span, message));
        }
        syn::visit::visit_expr_match(self, node);
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        let length = if_chain_length(node);
        if length == self.max_branches + 1 {
            let message = format!(
                "if/else chain has {length} branches (max {}); prefer polymorphism (Clean Code G23)",
                self.max_branches
            );
            let span = self.ctx.file.span_of(node);
            self.findings.push(self.ctx.finding(span, message));
        }
        syn::visit::visit_expr_if(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_long_match() {
        assert_findings(
            &PreferPolymorphism,
            "fn run(code: u8) -> u8 { match code { 1 => 1, 2 => 2, 3 => 3, 4 => 4, 5 => 5, _ => 0 } }",
            1,
        );
    }

    #[test]
    fn allows_short_match() {
        assert_clean(
            &PreferPolymorphism,
            "fn run(code: u8) -> u8 { match code { 1 => 1, 2 => 2, _ => 0 } }",
        );
    }

    #[test]
    fn flags_long_if_chain() {
        assert_findings(
            &PreferPolymorphism,
            "fn run(code: u8) { if code == 1 { one(); } else if code == 2 { two(); } else if code == 3 { three(); } else if code == 4 { four(); } else if code == 5 { five(); } else if code == 6 { six(); } }",
            1,
        );
    }

    #[test]
    fn allows_large_enum_match() {
        assert_clean(
            &PreferPolymorphism,
            "fn run(item: Item) -> u8 { match item { Item::A(value) => value, Item::B => 1, Item::C => 2, Item::D => 3, Item::E => 4, Item::F => 5 } }",
        );
    }
}
