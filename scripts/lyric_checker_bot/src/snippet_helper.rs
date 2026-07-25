use annotate_snippets::{AnnotationKind, Level, Renderer, Snippet, renderer::DecorStyle};
use ttml_processor::error::{ParseErrorKind, TTMLProcessorError};

const fn get_error_name(kind: &ParseErrorKind) -> &'static str {
    match kind {
        ParseErrorKind::AttrError(_) => "AttrError",
        ParseErrorKind::EntityError(_) => "EntityError",
        ParseErrorKind::InvalidTimestamp(_) => "InvalidTimestamp",
        ParseErrorKind::MissingAttribute(_) => "MissingAttribute",
        ParseErrorKind::UnexpectedEof => "UnexpectedEof",
        ParseErrorKind::XmlError(_) => "XmlError",
    }
}

const fn get_help_suggestion(kind: &ParseErrorKind) -> &'static str {
    match kind {
        ParseErrorKind::AttrError(_) => {
            "Please check that the XML attributes are correctly formatted"
        }
        ParseErrorKind::EntityError(_) => {
            "Check that the XML predefined entity references used are valid."
        }
        ParseErrorKind::InvalidTimestamp(_) => {
            "The allowed timestamp format is 'hh:mm:ss.sss'; leading zeros may be omitted. For example, '1:03:36.120'."
        }
        ParseErrorKind::MissingAttribute(_) => {
            "Please check the log above and add the required attributes."
        }
        ParseErrorKind::UnexpectedEof => {
            "Please ensure that all open tags (such as </tt> or </body>) are properly closed."
        }
        ParseErrorKind::XmlError(_) => "Please check the file for XML syntax errors.",
    }
}

fn calculate_span(raw_text: &str, byte_offset: u64, kind: &ParseErrorKind) -> (usize, usize) {
    let offset = (byte_offset as usize).min(raw_text.len());

    let search_target = match kind {
        ParseErrorKind::InvalidTimestamp(s) | ParseErrorKind::EntityError(s) => Some(s.as_str()),
        _ => None,
    };

    if let Some(target) = search_target {
        let line_start = raw_text[..offset].rfind('\n').map_or(0, |i| i + 1);
        if let Some(match_idx) = raw_text[line_start..offset].rfind(target) {
            let absolute_start = line_start + match_idx;
            return (absolute_start, absolute_start + target.len());
        }
    }

    (offset, offset)
}

pub fn render_parse_error(error: &TTMLProcessorError, raw_text: &str, file_name: &str) -> String {
    match error {
        TTMLProcessorError::ParseError { kind, context } => {
            let (start, end) = calculate_span(raw_text, context.byte_offset, kind);
            let id_name = get_error_name(kind);

            Renderer::plain()
                .decor_style(DecorStyle::Unicode)
                .render(&[Level::ERROR
                    .primary_title(format!("{kind}"))
                    .id(id_name)
                    .element(
                        Snippet::source(raw_text)
                            .path(file_name)
                            .annotation(AnnotationKind::Primary.span(start..end).label(id_name)),
                    )
                    .element(
                        Level::NOTE
                            .message(format!("Tag stack: {}", context.tag_stack.join(" > "))),
                    )
                    .element(Level::HELP.message(get_help_suggestion(kind)))])
        }
        _ => format!("{error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ttml_processor::parse_ttml;

    #[test]
    fn test_parse_error_rendering() {
        let broken_xml = r#"<tt xmlns="http://www.w3.org/ns/ttml" xml:lang="zh">
    <head>
        <metadata>
            <ttm:agent type="person" xml:id="v1">
                <ttm:name type="full">Ryan Gosling</ttm:name>
            </ttm:agent>
        </metadata>
    </head>
    <body>
        <div itunes:songPart="Verse">
            <p begin="10.522" end="13.518" itunes:key="L1" ttm:agent="v1">
                <span begin="aaaaa" end="13.518">test</span>
            </p>
        </div>
    </body>
</tt>"#;

        let result = parse_ttml(broken_xml);

        let err = result.expect_err("预期应该发生解析错误");

        assert!(matches!(
            err,
            TTMLProcessorError::ParseError {
                kind: ParseErrorKind::InvalidTimestamp(_),
                ..
            }
        ));

        if let TTMLProcessorError::ParseError { kind, context } = &err {
            if let ParseErrorKind::InvalidTimestamp(s) = kind {
                assert_eq!(s.as_str(), "aaaaa");
            }

            assert_eq!(context.tag_stack.join(" > "), "tt > body > div > p > span");
        }

        let rendered_snippet = render_parse_error(&err, broken_xml, "test.ttml");
        assert!(rendered_snippet.contains("error[InvalidTimestamp]"));
        assert!(rendered_snippet.contains("Invalid timestamp format: aaaaa"));

        println!("{err:#?} \n\n {rendered_snippet}");

        // error[InvalidTimestamp]: Invalid timestamp format: aaaaa
        //    ╭▸ test.ttml:12:30
        //    │
        // 12 │                 <span begin="aaaaa" end="13.518">test</span>
        //    │                              ━━━━━ InvalidTimestamp
        //    │
        //    ├ note: Tag stack: tt > body > div > p > span
        //    ╰ help: The allowed timestamp format is 'hh:mm:ss.sss'; leading zeros may be omitted. For example, '1:03:36.120'.
    }
}
