use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

const TYPE_WORDS: &[&str] = &[
    "str", "string", "num", "number", "int", "integer", "bool", "boolean", "arr", "array", "vec",
    "vector", "obj", "object", "ptr", "pointer", "func", "fn", "map", "list", "set",
];

pub struct NoEncodings;

impl Rule for NoEncodings {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-encodings",
            clean_code_id: "N6",
            clean_code_rule: "Avoid Encodings",
            description:
                "Disallow encoded names such as Hungarian notation prefixes (Clean Code N6)",
            tier: Tier::Static,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let mut words: Vec<String> = TYPE_WORDS.iter().map(|word| (*word).to_string()).collect();
        words.extend(ctx.options_names("patterns"));
        let mut visitor = Visitor {
            ctx,
            words,
            findings: Vec::new(),
        };
        visitor.visit_file(ctx.ast);
        visitor.findings
    }
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    words: Vec<String>,
    findings: Vec<Finding>,
}

fn is_encoded_name(text: &str, words: &[String]) -> bool {
    let skipped_prefix = text.starts_with('_');
    let uppercase_name = text
        .chars()
        .all(|character| !character.is_ascii_lowercase());
    let skipped = skipped_prefix || uppercase_name;
    !skipped && has_type_prefix(text, words)
}

fn has_type_prefix(text: &str, words: &[String]) -> bool {
    let first = text.split('_').next();
    let snake_prefix = text.contains('_')
        && matches!(first, Some(first) if words.iter().any(|word| word == first));
    let camel_prefix = words.iter().any(|word| camel_prefixed(text, word));
    snake_prefix || camel_prefix
}

fn camel_prefixed(text: &str, word: &str) -> bool {
    let rest = text.strip_prefix(word).unwrap_or_default();
    let next = rest.chars().next();
    matches!(next, Some(next) if next.is_ascii_uppercase())
}

impl Visitor<'_, '_> {
    fn check_name(&mut self, name: &syn::Ident) {
        let text = name.to_string();
        if !is_encoded_name(&text, &self.words) {
            return;
        }
        let message = format!(
            "name `{text}` encodes its type; drop the prefix, the type system knows it (Clean Code N6)"
        );
        let span = self.ctx.file.span_of(name);
        self.findings.push(self.ctx.finding(span, message));
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        self.check_name(&node.ident);
        syn::visit::visit_pat_ident(self, node);
    }

    fn visit_field(&mut self, node: &'ast syn::Field) {
        if let Some(ident) = &node.ident {
            self.check_name(ident);
        }
        syn::visit::visit_field(self, node);
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        self.check_name(&node.ident);
        syn::visit::visit_item_const(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_snake_case_type_prefix() {
        assert_findings(&NoEncodings, "fn run() { let str_name = \"x\"; }", 1);
    }

    #[test]
    fn flags_camel_case_type_prefix() {
        assert_findings(&NoEncodings, "fn run() { let strName = \"x\"; }", 1);
    }

    #[test]
    fn allows_plain_name() {
        assert_clean(&NoEncodings, "fn run() { let user_name = \"x\"; }");
    }
}
