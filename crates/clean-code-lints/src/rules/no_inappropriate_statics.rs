use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

pub struct NoInappropriateStatics;

impl Rule for NoInappropriateStatics {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-inappropriate-statics",
            clean_code_id: "G18",
            clean_code_rule: "Inappropriate Static",
            description: "Disallow types with only static members; prefer plain module functions (Clean Code G18)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: false,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let file = ctx.file;
        let ast = ctx.ast;
        let mut findings = Vec::new();
        for item in &ast.items {
            let syn::Item::Struct(item_struct) = item else {
                continue;
            };
            let Some(message) = static_only_message(item_struct, ast) else {
                continue;
            };
            findings.push(Finding::new(
                file.path.clone(),
                file.span_of(&item_struct.ident),
                message,
            ));
        }
        findings
    }
}

struct ImplStats {
    functions: usize,
    has_receiver: bool,
}

impl ImplStats {
    fn exceeds(&self) -> bool {
        let enough_functions = self.functions >= 2;
        let no_receiver = !self.has_receiver;
        enough_functions && no_receiver
    }
}

fn static_only_message(item_struct: &syn::ItemStruct, ast: &syn::File) -> Option<String> {
    if !matches!(item_struct.fields, syn::Fields::Unit) {
        return None;
    }
    let name = item_struct.ident.to_string();
    let stats = impl_statistics(ast, &name);
    stats.exceeds().then(|| {
        format!(
            "`{name}` is a namespace with only associated functions; use free functions or a module (Clean Code G18)"
        )
    })
}

fn impl_statistics(ast: &syn::File, type_name: &str) -> ImplStats {
    let mut stats = ImplStats {
        functions: 0,
        has_receiver: false,
    };
    for item in &ast.items {
        let syn::Item::Impl(item_impl) = item else {
            continue;
        };
        if !impl_matches_type(item_impl, type_name) {
            continue;
        }
        for member in &item_impl.items {
            if let syn::ImplItem::Fn(function) = member {
                stats.functions += 1;
                let receiver = function.sig.inputs.first();
                let taken = matches!(receiver, Some(syn::FnArg::Receiver(_)));
                stats.has_receiver = stats.has_receiver || taken;
            }
        }
    }
    stats
}

fn impl_matches_type(item_impl: &syn::ItemImpl, name: &str) -> bool {
    matches!(
        item_impl.self_ty.as_ref(),
        syn::Type::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == name)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_static_only_namespace() {
        assert_findings(
            &NoInappropriateStatics,
            "struct Math;\nimpl Math {\n    fn add(a: u8, b: u8) -> u8 { a + b }\n    fn sub(a: u8, b: u8) -> u8 { a - b }\n}",
            1,
        );
    }

    #[test]
    fn allows_struct_with_members() {
        assert_clean(
            &NoInappropriateStatics,
            "struct Counter { value: u8 }\nimpl Counter {\n    fn add(&mut self) {}\n    fn get(&self) -> u8 { self.value }\n}",
        );
    }

    #[test]
    fn allows_single_helper_function() {
        assert_clean(
            &NoInappropriateStatics,
            "struct Math;\nimpl Math { fn add(a: u8, b: u8) -> u8 { a + b } }",
        );
    }
}
