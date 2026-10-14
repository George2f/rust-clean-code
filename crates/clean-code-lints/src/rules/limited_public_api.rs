use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

const DEFAULT_MAX_EXPORTS: i64 = 50;
const DEFAULT_MAX_MEMBERS: i64 = 25;

pub struct LimitedPublicApi;

impl Rule for LimitedPublicApi {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "limited-public-api",
            clean_code_id: "G8",
            clean_code_rule: "Too Much Information",
            description: "Disallow modules that expose too many public members (Clean Code G8)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let max_exports = ctx.options_int("maxExports", DEFAULT_MAX_EXPORTS) as usize;
        let max_members = ctx.options_int("maxClassMembers", DEFAULT_MAX_MEMBERS) as usize;
        let ast = ctx.ast;
        let mut findings = Vec::new();
        findings.extend(export_finding(ctx, ast, max_exports));
        findings.extend(member_findings(ctx, ast, max_members));
        findings
    }
}

fn export_finding(ctx: &RuleContext<'_>, ast: &syn::File, max_exports: usize) -> Option<Finding> {
    let exports = ast.items.iter().filter(|item| is_public(item)).count();
    let too_many = exports > max_exports;
    too_many.then(|| {
        let message = format!(
            "module exposes {exports} public items (max {max_exports}); narrow the API (Clean Code G8)"
        );
        ctx.finding(ctx.file.span_at(0, 0), message)
    })
}

fn member_findings(ctx: &RuleContext<'_>, ast: &syn::File, max_members: usize) -> Vec<Finding> {
    let mut findings = Vec::new();
    for item in &ast.items {
        let syn::Item::Impl(item_impl) = item else {
            continue;
        };
        let members = public_members(item_impl);
        let too_many = members > max_members;
        if too_many {
            let message = format!(
                "type exposes {members} public members (max {max_members}); narrow the API (Clean Code G8)"
            );
            findings.push(ctx.finding(ctx.file.span_of(item_impl), message));
        }
    }
    findings
}

fn public_members(item_impl: &syn::ItemImpl) -> usize {
    item_impl
        .items
        .iter()
        .filter(|member| matches!(member, syn::ImplItem::Fn(function) if !matches!(function.vis, syn::Visibility::Inherited)))
        .count()
}

fn is_public(item: &syn::Item) -> bool {
    let visibility = match item {
        syn::Item::Fn(item) => Some(&item.vis),
        syn::Item::Struct(item) => Some(&item.vis),
        syn::Item::Enum(item) => Some(&item.vis),
        syn::Item::Trait(item) => Some(&item.vis),
        syn::Item::Const(item) => Some(&item.vis),
        syn::Item::Static(item) => Some(&item.vis),
        syn::Item::Type(item) => Some(&item.vis),
        syn::Item::Union(item) => Some(&item.vis),
        syn::Item::Mod(item) => Some(&item.vis),
        _ => None,
    };
    matches!(visibility, Some(visibility) if !matches!(visibility, syn::Visibility::Inherited))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_module_with_too_many_exports() {
        let mut source = String::new();
        for index in 0..51 {
            source.push_str(&format!("pub fn function_{index}() {{}}\n"));
        }
        assert_findings(&LimitedPublicApi, &source, 1);
    }

    #[test]
    fn allows_small_module() {
        assert_clean(&LimitedPublicApi, "pub fn one() {}\npub fn two() {}");
    }
}
