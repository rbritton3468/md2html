//! Markdown -> HTML conversion and document assembly.

use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Print-ready stylesheet embedded in every standalone document.
pub const DEFAULT_CSS: &str = include_str!("style.css");

/// CommonMark plus the GitHub-flavoured extensions people actually write.
fn parser_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION
        | Options::ENABLE_HEADING_ATTRIBUTES
}

/// Render `markdown` as an HTML fragment: no `<html>` wrapper, no styles.
pub fn to_fragment(markdown: &str) -> String {
    let parser = Parser::new_ext(markdown, parser_options());
    let mut html_out = String::with_capacity(markdown.len() * 3 / 2);
    html::push_html(&mut html_out, parser);
    html_out
}

/// Wrap a fragment in a standalone document. `css` of `None` omits the `<style>` block.
pub fn to_document(markdown: &str, title: &str, css: Option<&str>) -> String {
    let body = to_fragment(markdown);
    let mut doc = String::with_capacity(body.len() + css.map_or(0, str::len) + 512);

    doc.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    doc.push_str("<meta charset=\"utf-8\">\n");
    doc.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    doc.push_str("<title>");
    escape_into(&mut doc, title);
    doc.push_str("</title>\n");

    if let Some(css) = css {
        doc.push_str("<style>\n");
        doc.push_str(css);
        doc.push_str("</style>\n");
    }

    doc.push_str("</head>\n<body>\n");
    doc.push_str(&body);
    doc.push_str("</body>\n</html>\n");
    doc
}

/// Text of the first level-1 heading, used as a default document title.
pub fn first_h1(markdown: &str) -> Option<String> {
    let mut inside = false;
    let mut text = String::new();

    for event in Parser::new_ext(markdown, parser_options()) {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) => inside = true,
            Event::End(TagEnd::Heading(HeadingLevel::H1)) if inside => break,
            Event::Text(t) | Event::Code(t) if inside => text.push_str(&t),
            _ => {}
        }
    }

    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// Escape the five characters that matter inside markup, appending to `out`.
fn escape_into(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_basic_markdown() {
        assert_eq!(
            to_fragment("# Hi\n\nthere\n"),
            "<h1>Hi</h1>\n<p>there</p>\n"
        );
    }

    #[test]
    fn renders_gfm_extensions() {
        assert!(to_fragment("| a |\n|---|\n| b |\n").contains("<table>"));
        assert!(to_fragment("~~gone~~").contains("<del>"));
        assert!(to_fragment("- [x] done").contains("type=\"checkbox\""));
        assert!(to_fragment("note[^1]\n\n[^1]: here").contains("footnote"));
    }

    #[test]
    fn escapes_raw_html_in_title() {
        let doc = to_document("", "a & <b>", None);
        assert!(doc.contains("<title>a &amp; &lt;b&gt;</title>"));
        assert!(!doc.contains("<style>"));
    }

    #[test]
    fn document_embeds_css_and_body() {
        let doc = to_document("hello", "T", Some("p{color:red}"));
        assert!(doc.starts_with("<!DOCTYPE html>"));
        assert!(doc.contains("<style>\np{color:red}</style>"));
        assert!(doc.contains("<p>hello</p>"));
        assert!(doc.ends_with("</html>\n"));
    }

    #[test]
    fn finds_first_h1_only() {
        assert_eq!(first_h1("# One\n\n# Two\n").as_deref(), Some("One"));
        assert_eq!(first_h1("## Sub\n\ntext\n"), None);
        assert_eq!(first_h1("").as_deref(), None);
    }

    #[test]
    fn first_h1_joins_inline_markup() {
        assert_eq!(first_h1("# a *b* `c`").as_deref(), Some("a b c"));
    }
}
