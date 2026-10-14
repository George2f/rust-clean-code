use quote::ToTokens;
use syn::{Attribute, Expr, Path, Type};

pub fn path_last_segment(path: &Path) -> Option<String> {
    path.segments
        .last()
        .map(|segment| segment.ident.to_string())
}

pub fn type_last_segment(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(path) => path_last_segment(&path.path),
        Type::Reference(reference) => type_last_segment(&reference.elem),
        _ => None,
    }
}

pub fn attr_last_segment(attr: &Attribute) -> Option<String> {
    path_last_segment(attr.path())
}

pub fn has_cfg_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr_last_segment(attr).as_deref() == Some("cfg") && tokens_contain_ident(attr, "test")
    })
}

pub fn tokens_contain_ident<T: ToTokens + ?Sized>(node: &T, name: &str) -> bool {
    token_idents(node).iter().any(|ident| ident == name)
}

pub fn token_idents<T: ToTokens + ?Sized>(node: &T) -> Vec<String> {
    ident_tokens(node.to_token_stream())
}

fn ident_tokens(stream: proc_macro2::TokenStream) -> Vec<String> {
    let mut idents = Vec::new();
    for token in stream {
        match token {
            proc_macro2::TokenTree::Ident(ident) => idents.push(ident.to_string()),
            proc_macro2::TokenTree::Group(group) => idents.extend(ident_tokens(group.stream())),
            _ => {}
        }
    }
    idents
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanState {
    Code,
    String,
    StringEscape,
}

pub fn line_comment_start(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut state = ScanState::Code;
    for (index, window) in bytes.windows(2).enumerate() {
        let [byte, next] = window else {
            continue;
        };
        let in_code = state == ScanState::Code;
        if in_code && *byte == b'/' && *next == b'/' {
            return Some(index);
        }
        state = scan_next(state, *byte);
    }
    None
}

fn scan_next(state: ScanState, byte: u8) -> ScanState {
    match (state, byte) {
        (ScanState::StringEscape, _) => ScanState::String,
        (ScanState::String, b'"') => ScanState::Code,
        (ScanState::String, b'\\') => ScanState::StringEscape,
        (ScanState::Code, b'"') => ScanState::String,
        _ => state,
    }
}

pub fn macro_path_is_diverging(mac: &syn::Macro) -> bool {
    matches!(
        path_last_segment(&mac.path).as_deref(),
        Some("panic" | "todo" | "unimplemented" | "unreachable" | "exit")
    )
}

pub fn expr_is_diverging(expr: &Expr) -> bool {
    matches!(expr, Expr::Return(_) | Expr::Break(_) | Expr::Continue(_))
        || matches!(expr, Expr::Macro(mac) if macro_path_is_diverging(&mac.mac))
}
