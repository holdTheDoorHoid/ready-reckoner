//! Helpers shared by the native tests and the WebAssembly tests.
#![allow(dead_code)]

/// JSON text without the whitespace between tokens (strings untouched), so the pretty golden
/// files compare with the compact envelope as text. Comparing text, not parsed values, matters:
/// serde_json's default float parser can land one unit in the last place off, so a PlanOutput
/// read back into Rust and written again can differ in the 17th digit although the engine's own
/// output does not.
pub fn minify(json: &str) -> String {
    let mut out = String::with_capacity(json.len());
    let (mut in_string, mut escaped) = (false, false);
    for c in json.chars() {
        if in_string {
            out.push(c);
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_string = false,
                _ => {}
            }
        } else if c == '"' {
            in_string = true;
            out.push(c);
        } else if !c.is_whitespace() {
            out.push(c);
        }
    }
    out
}

/// The `value` of an ok envelope, as the exact text the engine wrote.
pub fn value_text(envelope: &str) -> &str {
    envelope
        .strip_prefix(r#"{"ok":true,"value":"#)
        .and_then(|rest| rest.strip_suffix('}'))
        .unwrap_or_else(|| panic!("not an ok envelope: {envelope:.300}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn minify_keeps_strings_whole() {
        assert_eq!(
            super::minify("{\n  \"a b\": [1, 2],\n  \"q\": \"say \\\"hi\\\" \"\n}\n"),
            r#"{"a b":[1,2],"q":"say \"hi\" "}"#
        );
    }
}
