use crate::ast_util::line_comment_start;
use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};
use crate::source::SourceFile;

const MIN_CODE_LENGTH: usize = 8;

pub struct NoCommentedOutCode;

impl Rule for NoCommentedOutCode {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-commented-out-code",
            clean_code_id: "C5",
            clean_code_rule: "Commented-Out Code",
            description: "Disallow commented-out code (Clean Code C5)",
            tier: Tier::Static,
            fixable: false,
            has_suggestions: true,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let file = ctx.file;
        let mut findings = Vec::new();
        for (index, line) in file.lines().enumerate() {
            if let Some(finding) = comment_finding(file, index, line) {
                findings.push(finding);
            }
        }
        findings
    }
}

fn comment_finding(file: &SourceFile, index: usize, line: &str) -> Option<Finding> {
    let start = line_comment_start(line)?;
    if !comment_is_code(line, start) {
        return None;
    }
    Some(comment_range_finding(CommentSite {
        file,
        index,
        line,
        start,
    }))
}

fn comment_is_code(line: &str, start: usize) -> bool {
    matches!(line.get(start + 2..), Some(rest) if looks_like_code(rest.trim()))
}

struct CommentSite<'a> {
    file: &'a SourceFile,
    index: usize,
    line: &'a str,
    start: usize,
}

fn comment_range_finding(site: CommentSite<'_>) -> Finding {
    let line_number = site.index + 1;
    let byte_start = site.file.byte_offset(line_number, site.start);
    let byte_end = site.file.byte_offset(line_number, site.line.len());
    let message = "commented-out code; delete it, version control remembers (Clean Code C5)";
    let finding = Finding::new(
        site.file.path.clone(),
        site.file.span_at(byte_start, byte_end),
        message,
    );
    finding.with_help("remove the dead comment or restore it as real code")
}

fn looks_like_code(comment: &str) -> bool {
    let long_enough = comment.len() >= MIN_CODE_LENGTH;
    let not_separator = !comment.starts_with('/');
    let punctuated = comment.contains(';')
        || comment.contains('=')
        || comment.contains('(')
        || comment.contains('{')
        || comment.contains('}');
    long_enough && not_separator && punctuated && parses_as_code(comment)
}

fn parses_as_code(comment: &str) -> bool {
    let statement = syn::parse_str::<syn::Stmt>(comment);
    let expression = syn::parse_str::<syn::Expr>(comment);
    let item = syn::parse_str::<syn::Item>(comment);
    statement.is_ok() || expression.is_ok() || item.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_commented_out_statement() {
        assert_findings(&NoCommentedOutCode, "fn run() {\n    // let x = 1;\n}", 1);
    }

    #[test]
    fn allows_prose_comment() {
        assert_clean(
            &NoCommentedOutCode,
            "fn run() {\n    // returns the sum of all values\n}",
        );
    }

    #[test]
    fn allows_url_inside_string() {
        assert_clean(
            &NoCommentedOutCode,
            "fn run() {\n    let url = \"http://example.com\";\n}",
        );
    }

    #[test]
    fn allows_separator_comment() {
        assert_clean(
            &NoCommentedOutCode,
            "fn run() {\n    //--------------------------------\n}",
        );
    }
}
