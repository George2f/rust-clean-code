use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

const DEFAULT_FORBIDDEN: &[&str] = &[
    "handle", "process", "manage", "util", "utils", "misc", "stuff", "thing", "data", "info",
    "temp", "tmp", "foo", "bar", "baz", "callback", "cb", "do", "perform", "execute",
];

pub struct FunctionNamesSayWhatTheyDo;

impl Rule for FunctionNamesSayWhatTheyDo {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "function-names-say-what-they-do",
            clean_code_id: "G20",
            clean_code_rule: "Function Names Should Say What They Do",
            description: "Require function names that say what they do (Clean Code G20)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let forbidden = ctx.options_names_or("forbidden", DEFAULT_FORBIDDEN);
        let mut visitor = Visitor {
            ctx,
            forbidden,
            findings: Vec::new(),
        };
        visitor.visit_file(ctx.ast);
        visitor.findings
    }
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    forbidden: Vec<String>,
    findings: Vec<Finding>,
}

impl Visitor<'_, '_> {
    fn check_name(&mut self, name: &syn::Ident) {
        let text = name.to_string();
        let vague = self.vague_word(&text);
        let Some(vague) = vague else {
            return;
        };
        let message = format!(
            "function name `{text}` contains vague word `{vague}`; say what it does (Clean Code G20)"
        );
        let span = self.ctx.file.span_of(name);
        self.findings.push(self.ctx.finding(span, message));
    }

    fn vague_word<'a>(&self, text: &'a str) -> Option<&'a str> {
        text.split('_').find(|segment| {
            self.forbidden
                .iter()
                .any(|word| word.eq_ignore_ascii_case(segment))
        })
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.check_name(&node.sig.ident);
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.check_name(&node.sig.ident);
        syn::visit::visit_impl_item_fn(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_vague_verb() {
        assert_findings(&FunctionNamesSayWhatTheyDo, "fn handle_event() {}", 1);
    }

    #[test]
    fn flags_vague_noun() {
        assert_findings(&FunctionNamesSayWhatTheyDo, "fn format_util() {}", 1);
    }

    #[test]
    fn allows_function_suffix_segment() {
        assert_clean(&FunctionNamesSayWhatTheyDo, "fn visit_item_fn() {}");
    }

    #[test]
    fn allows_descriptive_verb() {
        assert_clean(&FunctionNamesSayWhatTheyDo, "fn calculate_total() {}");
    }
}
