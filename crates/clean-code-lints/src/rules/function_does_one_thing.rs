use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

const DEFAULT_MAX_STATEMENTS: i64 = 20;
const DEFAULT_MAX_LINES: i64 = 30;

pub struct FunctionDoesOneThing;

impl Rule for FunctionDoesOneThing {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "function-does-one-thing",
            clean_code_id: "G30",
            clean_code_rule: "Functions Should Do One Thing",
            description: "Enforce that functions do one thing and stay short (Clean Code G30)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let max_statements = ctx.options_int("maxStatements", DEFAULT_MAX_STATEMENTS) as usize;
        let max_lines = ctx.options_int("maxLines", DEFAULT_MAX_LINES) as usize;
        let mut visitor = Visitor {
            ctx,
            max_statements,
            max_lines,
            findings: Vec::new(),
        };
        visitor.visit_file(ctx.ast);
        visitor.findings
    }
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    max_statements: usize,
    max_lines: usize,
    findings: Vec<Finding>,
}

struct StmtCount {
    count: usize,
}

impl<'ast> Visit<'ast> for StmtCount {
    fn visit_stmt(&mut self, node: &'ast syn::Stmt) {
        self.count += 1;
        syn::visit::visit_stmt(self, node);
    }
}

impl Visitor<'_, '_> {
    fn check_fn(&mut self, signature: &syn::Signature, block: &syn::Block) {
        let mut counter = StmtCount { count: 0 };
        counter.visit_block(block);
        let span = self.ctx.file.span_of(block);
        let lines = span.end.line.saturating_sub(span.start.line) + 1;
        if counter.count > self.max_statements || lines > self.max_lines {
            let message = format!(
                "function `{}` has {} statements and {lines} lines (max {} statements, {} lines); split it (Clean Code G30)",
                signature.ident, counter.count, self.max_statements, self.max_lines
            );
            let name_span = self.ctx.file.span_of(&signature.ident);
            self.findings.push(self.ctx.finding(name_span, message));
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
    fn flags_long_function() {
        let mut body = String::from("fn run() {\n");
        for index in 0..25 {
            body.push_str(&format!("    let value_{index} = {index};\n"));
        }
        body.push_str("}\n");
        assert_findings(&FunctionDoesOneThing, &body, 1);
    }

    #[test]
    fn allows_short_function() {
        assert_clean(&FunctionDoesOneThing, "fn run() {\n    let value = 1;\n}");
    }
}
