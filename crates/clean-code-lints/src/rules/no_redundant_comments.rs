use crate::ast_util::line_comment_start;
use crate::rule::{Finding, Rule, RuleContext, RuleMeta, Tier};
use crate::source::SourceFile;

const MIN_WORD_LENGTH: usize = 3;

pub struct NoRedundantComments;

impl Rule for NoRedundantComments {
    fn meta(&self) -> &'static RuleMeta {
        static META: RuleMeta = RuleMeta {
            name: "no-redundant-comments",
            clean_code_id: "C3",
            clean_code_rule: "Redundant Comment",
            description: "Disallow comments that restate the adjacent code (Clean Code C3)",
            tier: Tier::Heuristic,
            fixable: false,
            has_suggestions: true,
        };
        &META
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let file = ctx.file;
        let lines: Vec<&str> = file.lines().collect();
        let mut findings = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            let site = LineSite {
                file,
                lines: &lines,
                index,
                line,
            };
            if let Some(finding) = redundant_finding(site) {
                findings.push(finding);
            }
        }
        findings
    }
}

struct LineSite<'a> {
    file: &'a SourceFile,
    lines: &'a [&'a str],
    index: usize,
    line: &'a str,
}

fn redundant_finding(site: LineSite<'_>) -> Option<Finding> {
    let start = line_comment_start(site.line)?;
    if is_doc_comment(site.line) {
        return None;
    }
    let comment = comment_text(site.line, start)?;
    let restates = restates_code(site.lines, site.index, comment);
    restates.then(|| comment_finding(site, start))
}

fn comment_text(line: &str, start: usize) -> Option<&str> {
    let rest = line.get(start + 2..)?;
    Some(rest.trim())
}

fn is_doc_comment(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("///") || trimmed.starts_with("//!")
}

fn restates_code(lines: &[&str], index: usize, comment: &str) -> bool {
    let comment_words = significant_words(comment);
    let long_enough = comment_words.len() >= 2;
    let code_line = next_code_line(lines, index);
    matches!(code_line, Some(code_line) if long_enough && all_words_present(&comment_words, code_line))
}

fn next_code_line<'a>(lines: &[&'a str], index: usize) -> Option<&'a str> {
    lines
        .iter()
        .skip(index + 1)
        .find(|candidate| is_code_line(candidate))
        .copied()
}

fn all_words_present(comment_words: &[String], code_line: &str) -> bool {
    let code_words = significant_words(code_line);
    comment_words.iter().all(|word| code_words.contains(word))
}

fn comment_finding(site: LineSite<'_>, start: usize) -> Finding {
    let line_number = site.index + 1;
    let byte_start = site.file.byte_offset(line_number, start);
    let byte_end = site.file.byte_offset(line_number, site.line.len());
    let message = "redundant comment; it restates the code below (Clean Code C3)";
    let finding = Finding::new(
        site.file.path.clone(),
        site.file.span_at(byte_start, byte_end),
        message,
    );
    finding.with_help("delete the comment or explain why, not what")
}

fn is_code_line(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with("//")
        && !trimmed.starts_with("/*")
        && !trimmed.starts_with('*')
}

fn significant_words(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| word.len() >= MIN_WORD_LENGTH)
        .map(|word| word.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_clean, assert_findings};

    #[test]
    fn flags_comment_restating_code() {
        assert_findings(
            &NoRedundantComments,
            "fn run() {\n    // user count\n    let user_count = 0;\n}",
            1,
        );
    }

    #[test]
    fn allows_comment_adding_context() {
        assert_clean(
            &NoRedundantComments,
            "fn run() {\n    // retry is needed because the server is flaky at night\n    retry();\n}",
        );
    }
}
