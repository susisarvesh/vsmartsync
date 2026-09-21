//! Parse Matrix CGI text bodies for `Response-Code`.
//!
//! COSEC Devices API User Guide v28: HTTP success alone is not enough.
//! A Matrix operation succeeded only when the body reports `Response-Code=0`.

/// Extract the first documented `Response-Code` value from a text (or light XML) body.
///
/// Returns `None` when the key is absent or the value is not an integer.
pub fn parse_response_code(body: &str) -> Option<i32> {
    for raw_line in body.lines() {
        let line = raw_line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() {
            continue;
        }

        if let Some(code) = parse_key_value_line(line) {
            return Some(code);
        }

        if let Some(code) = parse_xml_element(line) {
            return Some(code);
        }
    }

    // Single-line bodies without newlines (or compacted XML).
    parse_key_value_line(body.trim()).or_else(|| parse_xml_element(body))
}

fn parse_key_value_line(line: &str) -> Option<i32> {
    let (key, value) = line.split_once('=')?;
    if !key.trim().eq_ignore_ascii_case("Response-Code") {
        return None;
    }
    value.trim().parse().ok()
}

fn parse_xml_element(fragment: &str) -> Option<i32> {
    let lower = fragment.to_ascii_lowercase();
    let start_tag = "<response-code>";
    let end_tag = "</response-code>";
    let start = lower.find(start_tag)?;
    let after = start + start_tag.len();
    let end_rel = lower[after..].find(end_tag)?;
    fragment[after..after + end_rel].trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::parse_response_code;

    #[test]
    fn parses_plain_success() {
        assert_eq!(parse_response_code("Response-Code=0"), Some(0));
    }

    #[test]
    fn parses_multiline_body() {
        let body = "name=Lobby\r\nResponse-Code=21\r\n";
        assert_eq!(parse_response_code(body), Some(21));
    }

    #[test]
    fn parses_case_insensitive_key() {
        assert_eq!(parse_response_code("response-code=35"), Some(35));
    }

    #[test]
    fn parses_simple_xml_element() {
        assert_eq!(
            parse_response_code("<Response-Code>0</Response-Code>"),
            Some(0)
        );
    }

    #[test]
    fn missing_code_is_none() {
        assert_eq!(parse_response_code("ok"), None);
        assert_eq!(parse_response_code(""), None);
    }

    #[test]
    fn non_integer_value_is_none() {
        assert_eq!(parse_response_code("Response-Code=abc"), None);
    }
}
