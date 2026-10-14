use syn::visit::Visit;

use crate::ast_util::token_idents;
use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

const DEFAULT_MAX_DISTANCE: i64 = 15;

pub struct DeclarationProximity;

impl Rule for DeclarationProximity {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "declaration-proximity",
            clean_code_id: "G10",
            clean_code_rule: "Vertical Separation",
            description: "Declare variables close to where they are first used (Clean Code G10)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let max_distance = ctx.options_int("maxDistance", DEFAULT_MAX_DISTANCE) as usize;
        let mut visitor = Visitor {
            ctx,
            max_distance,
            findings: Vec::new(),
        };
        visitor.visit_file(ctx.ast);
        visitor.findings
    }
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    max_distance: usize,
    findings: Vec<Finding>,
}

struct PatNames {
    names: Vec<String>,
}

impl<'ast> Visit<'ast> for PatNames {
    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        self.names.push(node.ident.to_string());
        syn::visit::visit_pat_ident(self, node);
    }
}

fn first_use_distance(remaining: &[syn::Stmt], names: &[String]) -> Option<usize> {
    let position = remaining
        .iter()
        .position(|candidate| statement_references(candidate, names));
    let distance = position?;
    Some(distance + 1)
}

fn statement_references(stmt: &syn::Stmt, names: &[String]) -> bool {
    let idents = token_idents(stmt);
    names
        .iter()
        .any(|name| idents.iter().any(|ident| ident == name))
}

impl Visitor<'_, '_> {
    fn check_block(&mut self, block: &syn::Block) {
        for index in 0..block.stmts.len() {
            self.check_local(block, index);
        }
    }

    fn check_local(&mut self, block: &syn::Block, index: usize) {
        let statement = &block.stmts[index];
        let syn::Stmt::Local(local) = statement else {
            return;
        };
        let mut patterns = PatNames { names: Vec::new() };
        patterns.visit_pat(&local.pat);
        let names = patterns.names;
        if names.is_empty() {
            return;
        }
        let remaining = &block.stmts[index + 1..];
        let Some(distance) = first_use_distance(remaining, &names) else {
            return;
        };
        if distance <= self.max_distance {
            return;
        }
        let finding = self.declaration_finding(local, &names, distance);
        self.findings.push(finding);
    }

    fn declaration_finding(
        &self,
        local: &syn::Local,
        names: &[String],
        distance: usize,
    ) -> Finding {
        let name = names.join(", ");
        let message = format!(
            "`{name}` is declared {distance} statements before its first use (max {}); move it closer (Clean Code G10)",
            self.max_distance
        );
        self.ctx.finding(self.ctx.file.span_of(local), message)
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
    fn flags_declaration_far_from_use() {
        let mut body = String::from("fn run() {\n    let start = 1;\n");
        for index in 0..20 {
            body.push_str(&format!("    step_{index}({index});\n"));
        }
        body.push_str("    finish(start);\n}\n");
        assert_findings(&DeclarationProximity, &body, 1);
    }

    #[test]
    fn allows_declaration_close_to_use() {
        assert_clean(
            &DeclarationProximity,
            "fn run() {\n    let start = 1;\n    finish(start);\n}",
        );
    }
}
