use crate::ast_util::line_comment_start;
use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};
use crate::source::SourceFile;

const DEFAULT_PATTERNS: &[&str] = &[
    "previously",
    "used to",
    "old version",
    "older version",
    "was removed",
    "removed on",
    "changed on",
    "before this",
    "after this",
    "added by",
    "moved here from",
    "replaced the old",
];

const DATE_LENGTH: usize = 10;
const DATE_DASH_POSITIONS: [usize; 2] = [4, 7];

pub struct NoInappropriateComments;

impl Rule for NoInappropriateComments {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-inappropriate-comments",
            clean_code_id: "C1",
            clean_code_rule: "Inappropriate Information",
            description:
                "Disallow comments holding information managed by version control (Clean Code C1)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: true,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let patterns: Vec<String> = ctx.options_names_or("patterns", DEFAULT_PATTERNS);
        let file = ctx.file;
        let mut findings = Vec::new();
        for (index, line) in file.lines().enumerate() {
            let site = CommentSite { file, index, line };
            if let Some(finding) = comment_finding(site, &patterns) {
                findings.push(finding);
            }
        }
        findings
    }
}

struct CommentSite<'a> {
    file: &'a SourceFile,
    index: usize,
    line: &'a str,
}

fn comment_finding(site: CommentSite<'_>, patterns: &[String]) -> Option<Finding> {
    let start = line_comment_start(site.line)?;
    let comment = comment_text(site.line, start)?;
    let reason = matched_reason(comment, patterns)?;
    Some(range_finding(site, start, reason))
}

fn comment_text(line: &str, start: usize) -> Option<&str> {
    let rest = line.get(start + 2..)?;
    Some(rest.trim())
}

fn matched_reason(comment: &str, patterns: &[String]) -> Option<String> {
    let lowered = comment.to_ascii_lowercase();
    let marker = patterns
        .iter()
        .find(|pattern| lowered.contains(pattern.as_str()));
    match marker {
        Some(pattern) => Some(format!("history marker `{pattern}`")),
        None => has_date(&lowered).then(|| "date stamp".to_string()),
    }
}

fn range_finding(site: CommentSite<'_>, start: usize, reason: String) -> Finding {
    let line_number = site.index + 1;
    let byte_start = site.file.byte_offset(line_number, start);
    let byte_end = site.file.byte_offset(line_number, site.line.len());
    let message = format!(
        "inappropriate comment ({reason}); version control keeps this history (Clean Code C1)"
    );
    let finding = Finding::new(
        site.file.path.clone(),
        site.file.span_at(byte_start, byte_end),
        message,
    );
    finding.with_help("delete the metadata or move it to the commit message")
}

fn has_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.windows(DATE_LENGTH).any(window_matches_date)
}

fn window_matches_date(window: &[u8]) -> bool {
    window
        .iter()
        .enumerate()
        .all(|(index, byte)| date_byte_matches(index, *byte))
}

fn date_byte_matches(index: usize, byte: u8) -> bool {
    if DATE_DASH_POSITIONS.contains(&index) {
        return byte == b'-';
    }
    byte.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_history_comment() {
        assert_findings(
            &NoInappropriateComments,
            "fn run() {\n    // previously we used a map here\n    run();\n}",
            1,
        );
    }

    #[test]
    fn flags_date_stamp() {
        assert_findings(
            &NoInappropriateComments,
            "fn run() {\n    // changed 2024-01-02 to fix the bug\n    run();\n}",
            1,
        );
    }

    #[test]
    fn allows_intent_comment() {
        assert_clean(
            &NoInappropriateComments,
            "fn run() {\n    // checks whether the user can log in\n    run();\n}",
        );
    }
}
