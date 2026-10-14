use syn::visit::Visit;

use crate::ast_util::{expr_is_diverging, macro_path_is_diverging, token_idents};
use crate::diagnostic::{Fix, TextEdit};
use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};
use crate::source::{NodeSpan, SourceFile};

pub struct NoDeadCode;

impl Rule for NoDeadCode {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-dead-code",
            clean_code_id: "G9",
            clean_code_rule: "Dead Code",
            description: "Disallow unreachable code (Clean Code G9)",
            tier: Tier::Static,
            fixable: true,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let ignore_names = ctx.options_names_or("ignoreNames", &[]);
        let mut visitor = Visitor {
            ctx,
            ignore_names,
            findings: Vec::new(),
        };
        visitor.visit_file(ctx.ast);
        visitor.findings
    }
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    ignore_names: Vec<String>,
    findings: Vec<Finding>,
}

fn unreachable_run(block: &syn::Block) -> &[syn::Stmt] {
    let Some(index) = block.stmts.iter().position(is_diverging_stmt) else {
        return &[];
    };
    block.stmts.get(index + 1..).unwrap_or(&[])
}

fn is_diverging_stmt(stmt: &syn::Stmt) -> bool {
    matches!(stmt, syn::Stmt::Expr(expr, _) if expr_is_diverging(expr))
        || matches!(stmt, syn::Stmt::Macro(stmt_macro) if macro_path_is_diverging(&stmt_macro.mac))
}

fn only_ignored_statements(run: &[syn::Stmt], ignore_names: &[String]) -> bool {
    run.iter()
        .all(|stmt| statement_only_references(stmt, ignore_names))
}

fn statement_only_references(stmt: &syn::Stmt, ignore_names: &[String]) -> bool {
    let idents = token_idents(stmt);
    idents.iter().all(|ident| ignore_names.contains(ident))
}

fn delete_range(file: &SourceFile, first: NodeSpan, last: NodeSpan) -> TextEdit {
    let start = file.byte_offset(first.start.line, 0);
    let end = newline_end(&file.text, last.byte_end);
    TextEdit {
        start,
        end,
        replacement: String::new(),
    }
}

fn newline_end(text: &str, end: usize) -> usize {
    let bytes = text.as_bytes();
    let tail = bytes.get(end..).unwrap_or_default();
    end + tail_drop(tail)
}

fn tail_drop(tail: &[u8]) -> usize {
    if tail.starts_with(b"\r\n") {
        return 2;
    }
    if tail.starts_with(b"\n") {
        return 1;
    }
    0
}

impl Visitor<'_, '_> {
    fn check_block(&mut self, block: &syn::Block) {
        let run = unreachable_run(block);
        if run.is_empty() {
            return;
        }
        let names_present = !self.ignore_names.is_empty();
        let all_ignored = only_ignored_statements(run, &self.ignore_names);
        if names_present && all_ignored {
            return;
        }
        let finding = self.unreachable_finding(run);
        self.findings.push(finding);
    }

    fn unreachable_finding(&self, run: &[syn::Stmt]) -> Finding {
        let file = self.ctx.file;
        let first_span = file.span_of(&run[0]);
        let last = run.last().unwrap_or(&run[0]);
        let last_span = file.span_of(last);
        let delete = delete_range(file, first_span, last_span);
        let message = "unreachable statement after a diverging expression (Clean Code G9)";
        let finding = Finding::new(file.path.clone(), first_span, message);
        finding.with_fix(Fix {
            edits: vec![delete],
        })
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_block(&mut self, node: &'ast syn::Block) {
        self.check_block(node);
        syn::visit::visit_block(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_statement_after_return() {
        let findings = assert_findings(
            &NoDeadCode,
            "fn run() -> i32 {\n    return 1;\n    let x = 2;\n    1\n}",
            1,
        );
        assert!(findings[0].fix.is_some());
    }

    #[test]
    fn allows_code_ending_with_return() {
        assert_clean(
            &NoDeadCode,
            "fn run() -> i32 {\n    let x = 2;\n    return x;\n}",
        );
    }

    #[test]
    fn flags_statement_after_panic() {
        assert_findings(
            &NoDeadCode,
            "fn run() {\n    panic!(\"stop\");\n    let x = 2;\n}",
            1,
        );
    }
}
