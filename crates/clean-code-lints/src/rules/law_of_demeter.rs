use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

const DEFAULT_ALLOW: &[&str] = &[
    "iter",
    "into_iter",
    "iter_mut",
    "map",
    "flat_map",
    "filter",
    "filter_map",
    "take",
    "skip",
    "step_by",
    "chain",
    "zip",
    "enumerate",
    "rev",
    "peekable",
    "then",
    "and_then",
    "or_else",
    "map_err",
    "unwrap_or",
    "unwrap_or_else",
    "unwrap_or_default",
    "ok_or",
    "ok_or_else",
    "collect",
    "to_vec",
    "to_string",
    "to_owned",
    "copied",
    "cloned",
    "as_ref",
    "as_mut",
    "as_str",
    "as_slice",
    "join",
    "sum",
    "product",
    "count",
    "last",
    "next",
    "get_or_insert",
];

const DEFAULT_MAX_CHAIN_LENGTH: i64 = 3;

pub struct LawOfDemeter;

impl Rule for LawOfDemeter {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "law-of-demeter",
            clean_code_id: "G36",
            clean_code_rule: "Avoid Transitive Navigation",
            description:
                "Disallow long member chains that navigate through other objects (Clean Code G36)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let max = ctx.options_int("maxChainLength", DEFAULT_MAX_CHAIN_LENGTH) as usize;
        let allow = ctx.options_names_or("allow", DEFAULT_ALLOW);
        let ast = ctx.ast;
        let mut visitor = Visitor {
            ctx,
            max,
            allow,
            findings: Vec::new(),
        };
        visitor.visit_file(ast);
        visitor.findings
    }
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    max: usize,
    allow: Vec<String>,
    findings: Vec<Finding>,
}

fn chain_stats(expr: &syn::Expr, allow: &[String]) -> (usize, bool) {
    match expr {
        syn::Expr::MethodCall(call) => method_stats(call, allow),
        syn::Expr::Field(field) => {
            let (depth, allowed) = chain_stats(&field.base, allow);
            (depth + 1, allowed)
        }
        syn::Expr::Try(try_expr) => chain_stats(&try_expr.expr, allow),
        syn::Expr::Paren(paren) => chain_stats(&paren.expr, allow),
        _ => (0, false),
    }
}

fn method_stats(call: &syn::ExprMethodCall, allow: &[String]) -> (usize, bool) {
    let (depth, allowed) = chain_stats(&call.receiver, allow);
    let listed = allow.contains(&call.method.to_string());
    (depth + 1, allowed || listed)
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let expression = syn::Expr::MethodCall(node.clone());
        let (depth, allowed) = chain_stats(&expression, &self.allow);
        let exceeding = depth == self.max + 1;
        if exceeding && !allowed {
            let message = format!(
                "member chain of length {depth}; talk to a direct collaborator (Clean Code G36)"
            );
            let span = self.ctx.file.span_of(node);
            self.findings.push(self.ctx.finding(span, message));
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_long_domain_chain() {
        assert_findings(
            &LawOfDemeter,
            "fn run(order: Order) -> u8 { order.customer().address().city().code() }",
            1,
        );
    }

    #[test]
    fn allows_three_link_chain() {
        assert_clean(
            &LawOfDemeter,
            "fn run(order: Order) -> u8 { order.customer().address().code() }",
        );
    }

    #[test]
    fn allows_iterator_chains() {
        assert_clean(
            &LawOfDemeter,
            "fn run(values: &[u8]) -> Vec<u8> { values.iter().map(|v| v + 1).filter(|v| v > 0).collect() }",
        );
    }
}
