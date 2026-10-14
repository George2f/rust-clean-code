use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::ToTokens;
use syn::visit::Visit;
use syn::{Attribute, FnArg, ImplItemFn, ItemFn, ItemImpl, Signature, TraitItemFn, Visibility};

use crate::ast_util::{attr_last_segment, tokens_contain_ident};
use crate::source::{NodeSpan, SourceFile};

#[derive(Debug, Clone)]
pub struct ParseError {
    pub file: PathBuf,
    pub message: String,
}

pub struct FileEntry {
    pub path: PathBuf,
    pub source: SourceFile,
    pub ast: syn::File,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub file: PathBuf,
    pub name_span: NodeSpan,
    pub is_pub: bool,
    pub in_trait_impl: bool,
    pub is_test: bool,
    pub is_method: bool,
    pub mut_self: bool,
    pub returns_value: bool,
    pub no_mangle: bool,
}

#[derive(Debug, Clone)]
pub struct DuplicateSite {
    pub file: PathBuf,
    pub span: NodeSpan,
    pub statement_count: usize,
    pub token_count: usize,
}

pub struct ProjectIndex {
    pub entries: Vec<FileEntry>,
    pub functions: Vec<FunctionInfo>,
    pub ident_counts: HashMap<String, usize>,
    pub body_hashes: HashMap<u64, Vec<DuplicateSite>>,
}

impl ProjectIndex {
    pub fn build(entries: Vec<FileEntry>) -> ProjectIndex {
        let mut functions = Vec::new();
        let mut ident_counts: HashMap<String, usize> = HashMap::new();
        let mut body_hashes: HashMap<u64, Vec<DuplicateSite>> = HashMap::new();
        for entry in &entries {
            let mut collector = Collector {
                entry,
                functions: Vec::new(),
                ident_counts: HashMap::new(),
                body_hashes: Vec::new(),
                trait_impl_depth: 0,
            };
            collector.visit_file(&entry.ast);
            functions.append(&mut collector.functions);
            for (name, count) in collector.ident_counts {
                *ident_counts.entry(name).or_insert(0) += count;
            }
            for (hash, site) in collector.body_hashes {
                body_hashes.entry(hash).or_default().push(site);
            }
        }
        ProjectIndex {
            entries,
            functions,
            ident_counts,
            body_hashes,
        }
    }

    pub fn file(&self, path: &std::path::Path) -> Option<&FileEntry> {
        self.entries.iter().find(|entry| entry.path == path)
    }
}

struct Collector<'a> {
    entry: &'a FileEntry,
    functions: Vec<FunctionInfo>,
    ident_counts: HashMap<String, usize>,
    body_hashes: Vec<(u64, DuplicateSite)>,
    trait_impl_depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImplKind {
    Inherent,
    Trait,
}

#[derive(Clone, Copy)]
struct FnShape<'a> {
    name: &'a Ident,
    sig: &'a Signature,
    attrs: &'a [Attribute],
    vis: &'a Visibility,
    body: Option<&'a syn::Block>,
    kind: ImplKind,
}

impl<'a> Collector<'a> {
    fn count_ident(&mut self, ident: &Ident) {
        *self.ident_counts.entry(ident.to_string()).or_insert(0) += 1;
    }

    fn count_tokens(&mut self, stream: TokenStream) {
        for token in stream {
            match token {
                TokenTree::Ident(ident) => {
                    let name = ident.to_string();
                    *self.ident_counts.entry(name).or_insert(0) += 1;
                }
                TokenTree::Group(group) => self.count_tokens(group.stream()),
                _ => {}
            }
        }
    }

    fn record_body(&mut self, block: &syn::Block) {
        let normalized = normalize_tokens(block);
        let token_count = normalized.split(' ').count();
        let site = body_site(self.entry, block, token_count);
        self.body_hashes.push((hash_string(&normalized), site));
    }

    fn record_fn(&mut self, shape: FnShape<'_>) {
        self.functions.push(self.describe_function(shape));
        if let Some(block) = shape.body {
            self.record_body(block);
        }
    }

    fn describe_function(&self, shape: FnShape<'_>) -> FunctionInfo {
        let FnShape {
            name,
            sig,
            attrs,
            vis,
            kind,
            ..
        } = shape;
        let receiver = sig.inputs.first().and_then(|input| match input {
            FnArg::Receiver(receiver) => Some(receiver),
            FnArg::Typed(_) => None,
        });
        FunctionInfo {
            name: name.to_string(),
            file: self.entry.path.clone(),
            name_span: self.entry.source.span_of(name),
            is_pub: !matches!(vis, Visibility::Inherited),
            in_trait_impl: kind == ImplKind::Trait,
            is_test: is_test_fn(attrs),
            is_method: receiver.is_some(),
            mut_self: is_mut_self(receiver),
            returns_value: !matches!(sig.output, syn::ReturnType::Default),
            no_mangle: is_no_mangle(attrs),
        }
    }
}

fn body_site(entry: &FileEntry, block: &syn::Block, token_count: usize) -> DuplicateSite {
    DuplicateSite {
        file: entry.path.clone(),
        span: entry.source.span_of(block),
        statement_count: block.stmts.len(),
        token_count,
    }
}

fn hash_string(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn normalize_tokens(block: &syn::Block) -> String {
    collapse_whitespace(&block.to_token_stream().to_string())
}

fn collapse_whitespace(text: &str) -> String {
    let tokens = text.split_whitespace();
    tokens.collect::<Vec<_>>().join(" ")
}

fn is_test_fn(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .any(|attr| attr_last_segment(attr).as_deref() == Some("test"))
}

fn is_mut_self(receiver: Option<&syn::Receiver>) -> bool {
    matches!(
        receiver,
        Some(receiver) if receiver.reference.is_some() && receiver.mutability.is_some()
    )
}

fn is_no_mangle(attrs: &[Attribute]) -> bool {
    attrs.iter().any(is_no_mangle_attr)
}

fn is_no_mangle_attr(attr: &Attribute) -> bool {
    let name = attr_last_segment(attr);
    let direct = matches!(name.as_deref(), Some("no_mangle" | "export_name"));
    let wrapped = matches!(name.as_deref(), Some("unsafe"))
        && (tokens_contain_ident(attr, "no_mangle") || tokens_contain_ident(attr, "export_name"));
    direct || wrapped
}

impl<'a> Visit<'a> for Collector<'a> {
    fn visit_ident(&mut self, node: &'a Ident) {
        self.count_ident(node);
    }

    fn visit_attribute(&mut self, node: &'a Attribute) {
        self.count_tokens(node.meta.to_token_stream());
    }

    fn visit_macro(&mut self, node: &'a syn::Macro) {
        self.count_tokens(node.tokens.clone());
        syn::visit::visit_macro(self, node);
    }

    fn visit_item_impl(&mut self, node: &'a ItemImpl) {
        let is_trait_impl = node.trait_.is_some();
        if is_trait_impl {
            self.trait_impl_depth += 1;
        }
        syn::visit::visit_item_impl(self, node);
        if is_trait_impl {
            self.trait_impl_depth -= 1;
        }
    }

    fn visit_item_fn(&mut self, node: &'a ItemFn) {
        let ItemFn {
            sig,
            attrs,
            vis,
            block,
            ..
        } = node;
        self.record_fn(FnShape {
            name: &sig.ident,
            sig,
            attrs,
            vis,
            body: Some(block),
            kind: ImplKind::Inherent,
        });
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'a ImplItemFn) {
        let kind = if self.trait_impl_depth > 0 {
            ImplKind::Trait
        } else {
            ImplKind::Inherent
        };
        let ImplItemFn {
            sig,
            attrs,
            vis,
            block,
            ..
        } = node;
        self.record_fn(FnShape {
            name: &sig.ident,
            sig,
            attrs,
            vis,
            body: Some(block),
            kind,
        });
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &'a TraitItemFn) {
        let TraitItemFn {
            sig,
            attrs,
            default,
            ..
        } = node;
        self.record_fn(FnShape {
            name: &sig.ident,
            sig,
            attrs,
            vis: &Visibility::Inherited,
            body: default.as_ref(),
            kind: ImplKind::Inherent,
        });
        syn::visit::visit_trait_item_fn(self, node);
    }
}
