use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct NoPassNull;

impl Rule for NoPassNull {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-pass-null",
            clean_code_id: "Ch7",
            clean_code_rule: "Don't Pass Null",
            description: "Disallow passing None as a function argument (Clean Code Ch7)",
            tier: Tier::Static,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let ignore = ctx.options_names("ignoreCallees");
        let mut visitor = Visitor {
            ctx,
            ignore,
            findings: Vec::new(),
        };
        visitor.visit_file(ctx.ast);
        visitor.findings
    }
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    ignore: Vec<String>,
    findings: Vec<Finding>,
}

fn is_none_literal(expr: &syn::Expr) -> bool {
    matches!(expr, syn::Expr::Paren(paren) if is_none_literal(&paren.expr))
        || matches!(expr, syn::Expr::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == "None"))
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        let callee = match node.func.as_ref() {
            syn::Expr::Path(path) => path
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string())
                .unwrap_or_default(),
            _ => String::new(),
        };
        let ignored = self
            .ignore
            .iter()
            .any(|entry| !callee.is_empty() && callee.starts_with(entry.trim_start_matches('^')));
        if !ignored {
            for argument in &node.args {
                if is_none_literal(argument) {
                    let span = self.ctx.file.span_of(argument);
                    self.findings.push(self.ctx.finding(
                        span,
                        "passing None as an argument; make the absence explicit or split the function (Clean Code Ch7)",
                    ));
                }
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_none_argument() {
        assert_findings(&NoPassNull, "fn run() { connect(None); }", 1);
    }

    #[test]
    fn allows_some_argument() {
        assert_clean(&NoPassNull, "fn run() { connect(Some(1)); }");
    }

    #[test]
    fn allows_binding_none() {
        assert_clean(
            &NoPassNull,
            "fn run() { let absent = None; use_it(absent); }",
        );
    }
}
