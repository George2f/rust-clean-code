use syn::visit::Visit;

use crate::ast_util::has_cfg_test;
use crate::rule::{Finding, Flag, Rule, RuleContext, RuleMeta, Tier};

const SQL_PAIRS: &[(&str, &str)] = &[
    ("select", "from"),
    ("insert", "into"),
    ("update", "set"),
    ("delete", "from"),
];

const HTML_TAGS: &[&str] = &[
    "html", "div", "span", "body", "table", "head", "script", "form", "link", "meta", "style",
];

pub struct NoMultipleLanguages;

impl Rule for NoMultipleLanguages {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-multiple-languages",
            clean_code_id: "G1",
            clean_code_rule: "Multiple Languages in One Source File",
            description: "Disallow SQL or HTML embedded in string literals; keep one language per file (Clean Code G1)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let check_sql = ctx.options_flag("checkSql", Flag::On);
        let check_html = ctx.options_flag("checkHtml", Flag::On);
        let mut visitor = Visitor {
            ctx,
            check_sql,
            check_html,
            findings: Vec::new(),
        };
        visitor.visit_file(ctx.ast);
        visitor.findings
    }
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    check_sql: Flag,
    check_html: Flag,
    findings: Vec<Finding>,
}

fn words(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| word.to_ascii_lowercase())
        .collect()
}

fn looks_like_sql(text: &str) -> bool {
    let words = words(text);
    SQL_PAIRS.iter().any(|(left, right)| {
        let has_left = words.iter().any(|word| word == left);
        let has_right = words.iter().any(|word| word == right);
        has_left && has_right
    })
}

fn looks_like_html(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered.char_indices().any(|(index, character)| {
        let is_open = character == '<';
        let rest = lowered.get(index + 1..).unwrap_or("");
        let rest = rest.strip_prefix('/').unwrap_or(rest);
        let tag: String = rest.chars().take_while(char::is_ascii_alphabetic).collect();
        is_open && HTML_TAGS.contains(&tag.as_str())
    })
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        syn::visit::visit_item_mod(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_expr_lit(&mut self, node: &'ast syn::ExprLit) {
        if let syn::Lit::Str(literal) = &node.lit {
            let text = literal.value();
            let language = detected_language(&text, self.check_sql, self.check_html);
            if let Some(language) = language {
                let message = format!(
                    "string literal contains {language}; move it to its own file or data source (Clean Code G1)"
                );
                let span = self.ctx.file.span_of(node);
                self.findings.push(self.ctx.finding(span, message));
            }
        }
        syn::visit::visit_expr_lit(self, node);
    }
}

fn detected_language(text: &str, check_sql: Flag, check_html: Flag) -> Option<&'static str> {
    let sql = check_sql.is_on() && looks_like_sql(text);
    if sql {
        return Some("SQL");
    }
    let html = check_html.is_on() && looks_like_html(text);
    if html {
        return Some("HTML");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_sql_literal() {
        assert_findings(
            &NoMultipleLanguages,
            "fn query() -> &'static str { \"SELECT id FROM users\" }",
            1,
        );
    }

    #[test]
    fn flags_html_literal() {
        assert_findings(
            &NoMultipleLanguages,
            "fn markup() -> &'static str { \"<div>hello</div>\" }",
            1,
        );
    }

    #[test]
    fn allows_plain_text() {
        assert_clean(
            &NoMultipleLanguages,
            "fn label() -> &'static str { \"active users\" }",
        );
    }

    #[test]
    fn allows_prose_with_sql_words() {
        assert_clean(
            &NoMultipleLanguages,
            "fn label() -> &'static str { \"selected from the list\" }",
        );
    }

    #[test]
    fn allows_comparison_operator() {
        assert_clean(
            &NoMultipleLanguages,
            "fn label() -> &'static str { \"value < 10\" }",
        );
    }
}
