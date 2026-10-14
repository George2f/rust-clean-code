use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

const COLLECTIONS: &[&str] = &[
    "Vec",
    "HashMap",
    "HashSet",
    "BTreeMap",
    "BTreeSet",
    "VecDeque",
    "BinaryHeap",
    "LinkedList",
];

pub struct NoReturnNull;

impl Rule for NoReturnNull {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-return-null",
            clean_code_id: "Ch7",
            clean_code_rule: "Don't Return Null",
            description: "Disallow returning Option-wrapped collections; return an empty value instead (Clean Code Ch7)",
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

fn collection_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Reference(reference) => collection_name(&reference.elem),
        syn::Type::Slice(_) => Some("slice".to_string()),
        syn::Type::Path(path) => path_collection(path),
        _ => None,
    }
}

fn path_collection(path: &syn::TypePath) -> Option<String> {
    let name = path.path.segments.last()?.ident.to_string();
    COLLECTIONS.contains(&name.as_str()).then_some(name)
}

fn option_collection(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    for argument in &arguments.args {
        if let syn::GenericArgument::Type(inner) = argument {
            return collection_name(inner);
        }
    }
    None
}

impl Visitor<'_, '_> {
    fn check_output(&mut self, signature: &syn::Signature) {
        let syn::ReturnType::Type(_, ty) = &signature.output else {
            return;
        };
        if let Some(collection) = option_collection(ty) {
            let message = format!(
                "returns `Option<{collection}>`; return an empty {collection} instead of None (Clean Code Ch7)"
            );
            let span = self.ctx.file.span_of(ty);
            self.findings.push(self.ctx.finding(span, message));
        }
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.check_output(&node.sig);
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.check_output(&node.sig);
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        self.check_output(&node.sig);
        syn::visit::visit_trait_item_fn(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_option_vec_return() {
        assert_findings(&NoReturnNull, "fn run() -> Option<Vec<u8>> { None }", 1);
    }

    #[test]
    fn flags_option_slice_return() {
        assert_findings(&NoReturnNull, "fn run() -> Option<&[u8]> { None }", 1);
    }

    #[test]
    fn allows_empty_collection_return() {
        assert_clean(&NoReturnNull, "fn run() -> Vec<u8> { Vec::new() }");
    }

    #[test]
    fn allows_scalar_option() {
        assert_clean(&NoReturnNull, "fn run() -> Option<u8> { None }");
    }

    #[test]
    fn allows_string_option() {
        assert_clean(&NoReturnNull, "fn run() -> Option<String> { None }");
    }
}
