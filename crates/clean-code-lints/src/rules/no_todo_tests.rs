use syn::visit::Visit;

use crate::ast_util::{attr_last_segment, path_last_segment};
use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct NoTodoTests;

impl Rule for NoTodoTests {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-todo-tests",
            clean_code_id: "T4",
            clean_code_rule: "An Ignored Test Is a Question about an Ambiguity",
            description: "Disallow todo tests and tests with empty bodies (Clean Code T4)",
            tier: Tier::Static,
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

impl Visitor<'_, '_> {
    fn check_test(
        &mut self,
        signature: &syn::Signature,
        attrs: &[syn::Attribute],
        block: &syn::Block,
    ) {
        let is_test = attrs
            .iter()
            .any(|attr| attr_last_segment(attr).as_deref() == Some("test"));
        if !is_test {
            return;
        }
        let span = self.ctx.file.span_of(&signature.ident);
        let empty_body = block.stmts.is_empty();
        if empty_body {
            self.findings.push(self.ctx.finding(
                span,
                "test has an empty body; write the test or delete it (Clean Code T4)",
            ));
            return;
        }
        let mut scan = TodoScan { found: false };
        scan.visit_block(block);
        if scan.found {
            self.findings.push(self.ctx.finding(
                span,
                "test is left as todo; finish it or turn the question into an issue (Clean Code T4)",
            ));
        }
    }
}

struct TodoScan {
    found: bool,
}

impl<'ast> Visit<'ast> for TodoScan {
    fn visit_expr_macro(&mut self, node: &'ast syn::ExprMacro) {
        if matches!(
            path_last_segment(&node.mac.path).as_deref(),
            Some("todo" | "unimplemented")
        ) {
            self.found = true;
        }
        syn::visit::visit_expr_macro(self, node);
    }

    fn visit_stmt_macro(&mut self, node: &'ast syn::StmtMacro) {
        if matches!(
            path_last_segment(&node.mac.path).as_deref(),
            Some("todo" | "unimplemented")
        ) {
            self.found = true;
        }
        syn::visit::visit_stmt_macro(self, node);
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.check_test(&node.sig, &node.attrs, &node.block);
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.check_test(&node.sig, &node.attrs, &node.block);
        syn::visit::visit_impl_item_fn(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_empty_test() {
        assert_findings(&NoTodoTests, "#[test]\nfn t() {}", 1);
    }

    #[test]
    fn flags_todo_test() {
        assert_findings(&NoTodoTests, "#[test]\nfn t() { todo!() }", 1);
    }

    #[test]
    fn allows_completed_test() {
        assert_clean(&NoTodoTests, "#[test]\nfn t() { assert_eq!(1, 1); }");
    }

    #[test]
    fn allows_non_test_fn() {
        assert_clean(&NoTodoTests, "fn helper() {}");
    }
}
