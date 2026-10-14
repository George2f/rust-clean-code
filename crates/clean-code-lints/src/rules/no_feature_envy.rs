use std::collections::HashMap;

use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

const DEFAULT_MINIMUM_ACCESSES: i64 = 4;

pub struct NoFeatureEnvy;

impl Rule for NoFeatureEnvy {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-feature-envy",
            clean_code_id: "G14",
            clean_code_rule: "Feature Envy",
            description: "Disallow methods that access another object more than their own state (Clean Code G14)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let minimum = ctx.options_int("minimumAccesses", DEFAULT_MINIMUM_ACCESSES) as usize;
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

struct AccessCounts {
    names: Vec<String>,
    counts: HashMap<String, usize>,
}

impl AccessCounts {
    fn record_base(&mut self, base: &syn::Expr) {
        let syn::Expr::Path(path) = base else {
            return;
        };
        let Some(segment) = path.path.segments.first() else {
            return;
        };
        let name = segment.ident.to_string();
        let tracked = self.names.iter().any(|candidate| candidate == &name);
        if tracked {
            *self.counts.entry(name).or_insert(0) += 1;
        }
    }
}

impl<'ast> Visit<'ast> for AccessCounts {
    fn visit_expr_field(&mut self, node: &'ast syn::ExprField) {
        self.record_base(&node.base);
        syn::visit::visit_expr_field(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        self.record_base(&node.receiver);
        syn::visit::visit_expr_method_call(self, node);
    }
}

fn method_names(method: &syn::ImplItemFn) -> Vec<String> {
    let mut names = vec!["self".to_string()];
    for input in &method.sig.inputs {
        let syn::FnArg::Typed(parameter) = input else {
            continue;
        };
        let syn::Pat::Ident(pattern) = parameter.pat.as_ref() else {
            continue;
        };
        names.push(pattern.ident.to_string());
    }
    names
}

fn access_counts(block: &syn::Block, names: &[String]) -> HashMap<String, usize> {
    let mut visitor = AccessCounts {
        names: names.to_vec(),
        counts: HashMap::new(),
    };
    visitor.visit_block(block);
    visitor.counts
}

impl Visitor<'_, '_> {
    fn check_impl_method(&mut self, method: &syn::ImplItemFn) {
        let names = method_names(method);
        let counts = access_counts(&method.block, &names);
        let self_count = counts.get("self").copied().unwrap_or(0);
        for name in names.iter().filter(|name| name.as_str() != "self") {
            let envy = self.envy_for(name, &counts, self_count);
            if let Some(envy) = envy {
                self.findings.push(self.envy_finding(method, envy));
            }
        }
    }

    fn envy_for(
        &self,
        name: &str,
        counts: &HashMap<String, usize>,
        self_count: usize,
    ) -> Option<Envy> {
        let count = counts.get(name).copied().unwrap_or(0);
        let envious = count >= self.minimum && count > self_count;
        envious.then(|| Envy {
            name: name.to_string(),
            count,
            self_count,
        })
    }

    fn envy_finding(&self, method: &syn::ImplItemFn, envy: Envy) -> Finding {
        let message = format!(
            "method `{}` accesses `{}` {} times but its own state {} times; move the behavior (Clean Code G14)",
            method.sig.ident, envy.name, envy.count, envy.self_count
        );
        let span = self.ctx.file.span_of(&method.sig.ident);
        self.ctx.finding(span, message)
    }
}

struct Envy {
    name: String,
    count: usize,
    self_count: usize,
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        for item in &node.items {
            if let syn::ImplItem::Fn(method) = item {
                self.check_impl_method(method);
            }
        }
        syn::visit::visit_item_impl(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_method_obsessed_with_parameter() {
        assert_findings(
            &NoFeatureEnvy,
            "struct Customer; impl Customer { fn summarize(&self, order: &Order) -> u32 { order.total + order.total + order.total + order.total } }",
            1,
        );
    }

    #[test]
    fn allows_method_using_own_state() {
        assert_clean(
            &NoFeatureEnvy,
            "struct Customer; impl Customer { fn total(&self, order: &Order) -> u32 { self.own + order.total } }",
        );
    }
}
