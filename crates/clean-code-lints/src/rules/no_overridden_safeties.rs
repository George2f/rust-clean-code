use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct NoOverriddenSafeties;

impl Rule for NoOverriddenSafeties {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-overridden-safeties",
            clean_code_id: "G4",
            clean_code_rule: "Overridden Safeties",
            description: "Disallow disabled safety mechanisms such as allow attributes, unsafe blocks and panicking unwraps (Clean Code G4)",
            tier: Tier::Static,
            fixable: false,
            has_suggestions: true,
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

fn is_allow_attribute(attr: &syn::Attribute) -> bool {
    attr.path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "allow")
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_attribute(&mut self, node: &'ast syn::Attribute) {
        if is_allow_attribute(node) {
            let message = "lint suppression; fix the underlying issue or justify the override with a comment (Clean Code G4)";
            let span = self.ctx.file.span_of(node);
            let finding = self.ctx.finding(span, message);
            self.findings
                .push(finding.with_help("remove the allow, or document why it is unavoidable"));
        }
        syn::visit::visit_attribute(self, node);
    }

    fn visit_expr_unsafe(&mut self, node: &'ast syn::ExprUnsafe) {
        let message = "unsafe block; isolate unsafe code and justify it (Clean Code G4)";
        let span = self.ctx.file.span_of(node);
        self.findings.push(self.ctx.finding(span, message));
        syn::visit::visit_expr_unsafe(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let name = node.method.to_string();
        let panics = name == "unwrap" || name == "expect";
        if panics {
            let message =
                format!("`.{name}()` panics on failure; handle the error instead (Clean Code G4)");
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
    fn flags_allow_attribute() {
        assert_findings(&NoOverriddenSafeties, "#[allow(dead_code)]\nfn run() {}", 1);
    }

    #[test]
    fn flags_unwrap() {
        assert_findings(
            &NoOverriddenSafeties,
            "fn run(input: Option<u8>) -> u8 { input.unwrap() }",
            1,
        );
    }

    #[test]
    fn flags_unsafe_block() {
        assert_findings(
            &NoOverriddenSafeties,
            "fn run() { unsafe { danger(); } }",
            1,
        );
    }

    #[test]
    fn allows_clean_code() {
        assert_clean(
            &NoOverriddenSafeties,
            "fn run(input: Option<u8>) -> u8 { input.unwrap_or(0) }",
        );
    }
}
