use syn::visit::Visit;

use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};

const DEFAULT_MAX_NAMED_IMPORTS: i64 = 10;

pub struct PreferNamespaceImports;

impl Rule for PreferNamespaceImports {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "prefer-namespace-imports",
            clean_code_id: "J1",
            clean_code_rule: "Avoid Long Import Lists by Using Wildcards",
            description: "Avoid long import lists by importing the namespace (Clean Code J1)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: true,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let max = ctx.options_int("maxNamedImports", DEFAULT_MAX_NAMED_IMPORTS) as usize;
        let mut visitor = Visitor {
            ctx,
            max,
            findings: Vec::new(),
        };
        visitor.visit_file(ctx.ast);
        visitor.findings
    }
}

struct Visitor<'a, 'c> {
    ctx: &'c RuleContext<'a>,
    max: usize,
    findings: Vec<Finding>,
}

fn count_leaves(tree: &syn::UseTree) -> usize {
    match tree {
        syn::UseTree::Path(path) => count_leaves(&path.tree),
        syn::UseTree::Name(_) | syn::UseTree::Rename(_) | syn::UseTree::Glob(_) => 1,
        syn::UseTree::Group(group) => group.items.iter().map(count_leaves).sum(),
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        let count = count_leaves(&node.tree);
        if count > self.max {
            let message = format!(
                "use declaration lists {count} imports (max {}); import the namespace instead (Clean Code J1)",
                self.max
            );
            let span = self.ctx.file.span_of(node);
            self.findings.push(
                self.ctx
                    .finding(span, message)
                    .with_help("bring in the module and qualify the names, or split the list"),
            );
        }
        syn::visit::visit_item_use(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_long_import_list() {
        let names: Vec<String> = (0..11).map(|index| format!("item_{index}")).collect();
        let source = format!("use module::{{{}}};\n", names.join(", "));
        assert_findings(&PreferNamespaceImports, &source, 1);
    }

    #[test]
    fn allows_short_import_list() {
        assert_clean(
            &PreferNamespaceImports,
            "use module::{first, second, third};",
        );
    }

    #[test]
    fn allows_single_import() {
        assert_clean(&PreferNamespaceImports, "use std::collections::HashMap;");
    }
}
