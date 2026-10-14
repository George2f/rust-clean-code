use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct CommandQuerySeparation;

impl Rule for CommandQuerySeparation {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "command-query-separation",
            clean_code_id: "Ch3",
            clean_code_rule: "Command-Query Separation",
            description: "Separate commands that mutate state from queries that return values (Clean Code Ch3)",
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

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let mut_self = matches!(
            node.sig.inputs.first(),
            Some(syn::FnArg::Receiver(receiver))
                if receiver.reference.is_some() && receiver.mutability.is_some()
        );
        let returns_value = !matches!(node.sig.output, syn::ReturnType::Default);
        if mut_self && returns_value {
            let message = format!(
                "method `{}` mutates and returns a value; separate the command from the query (Clean Code Ch3)",
                node.sig.ident
            );
            let span = self.ctx.file.span_of(&node.sig.ident);
            self.findings.push(self.ctx.finding(span, message));
        }
        syn::visit::visit_impl_item_fn(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_mutating_method_with_return() {
        assert_findings(
            &CommandQuerySeparation,
            "struct Stack; impl Stack { fn pop(&mut self) -> u8 { 0 } }",
            1,
        );
    }

    #[test]
    fn allows_command_without_return() {
        assert_clean(
            &CommandQuerySeparation,
            "struct Stack; impl Stack { fn push(&mut self, v: u8) {} }",
        );
    }

    #[test]
    fn allows_query_without_mutation() {
        assert_clean(
            &CommandQuerySeparation,
            "struct Stack; impl Stack { fn len(&self) -> usize { 0 } }",
        );
    }
}
