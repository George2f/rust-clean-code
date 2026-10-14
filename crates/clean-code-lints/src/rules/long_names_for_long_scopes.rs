use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

const DEFAULT_ALLOW: &[&str] = &["i", "j", "k", "x", "y", "z", "e", "_", "it"];
const DEFAULT_MIN_LENGTH: i64 = 3;

pub struct LongNamesForLongScopes;

impl Rule for LongNamesForLongScopes {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "long-names-for-long-scopes",
            clean_code_id: "N5",
            clean_code_rule: "Use Long Names for Long Scopes",
            description: "Require long names for long scopes (Clean Code N5)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let settings = settings_of(ctx);
        let file = ctx.file;
        let ast = ctx.ast;
        let mut findings = Vec::new();
        for item in &ast.items {
            let Some(name) = item_name(item) else {
                continue;
            };
            let text = name.to_string();
            if !is_short_module_name(&text, &settings) {
                continue;
            }
            let message = format!(
                "module-level name `{text}` is too short; use a longer, descriptive name (Clean Code N5)"
            );
            findings.push(Finding::new(file.path.clone(), file.span_of(name), message));
        }
        findings
    }
}

struct Settings {
    min_length: usize,
    allow: Vec<String>,
}

fn settings_of(ctx: &RuleContext<'_>) -> Settings {
    Settings {
        min_length: ctx.options_int("minLength", DEFAULT_MIN_LENGTH) as usize,
        allow: ctx.options_names_or("allow", DEFAULT_ALLOW),
    }
}

fn is_short_module_name(text: &str, settings: &Settings) -> bool {
    let long_enough = text.len() >= settings.min_length;
    let skipped = text.starts_with('_');
    let allowed = settings.allow.iter().any(|allowed| allowed == text);
    !(long_enough || skipped || allowed)
}

fn item_name(item: &syn::Item) -> Option<&syn::Ident> {
    match item {
        syn::Item::Fn(item) => Some(&item.sig.ident),
        syn::Item::Const(item) => Some(&item.ident),
        syn::Item::Static(item) => Some(&item.ident),
        syn::Item::Struct(item) => Some(&item.ident),
        syn::Item::Enum(item) => Some(&item.ident),
        syn::Item::Trait(item) => Some(&item.ident),
        syn::Item::Type(item) => Some(&item.ident),
        syn::Item::Union(item) => Some(&item.ident),
        syn::Item::Mod(item) => Some(&item.ident),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_short_module_name() {
        assert_findings(&LongNamesForLongScopes, "fn go() {}", 1);
    }

    #[test]
    fn allows_allowed_short_name() {
        assert_clean(&LongNamesForLongScopes, "fn it() {}");
    }

    #[test]
    fn allows_descriptive_name() {
        assert_clean(&LongNamesForLongScopes, "fn calculate_totals() {}");
    }

    #[test]
    fn checks_constants() {
        assert_findings(&LongNamesForLongScopes, "const N: u8 = 1;", 1);
    }
}
