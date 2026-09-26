//! Section 10: sources. Every citation the plan and the packet use, numbered as the packet's
//! brackets refer to them, with URLs and retrieval dates; the expert estimates marked; and the
//! data credits, with the National Risk Index statement exactly as its terms require.

use rr_types::Citation;

use super::text::{self, md};
use crate::pipeline::Assessment;

pub(super) fn write(a: &Assessment, provenance: &[Citation], out: &mut Vec<String>) {
    out.push("## Sources".to_owned());
    out.push(String::new());
    out.push(
        "Every number in this packet comes from one of these sources; the numbers in brackets \
         point to this list. Sources marked \"expert estimate\" are judgements, not measured \
         data, and the packet shows those numbers as estimates. The targets, the plan and the \
         packet are Ready Reckoner's own calculations from these sources."
            .to_owned(),
    );
    out.push(String::new());
    for (i, c) in provenance.iter().enumerate() {
        let year = c.year.map(|y| format!(", {y}")).unwrap_or_default();
        let prior = if c.prior { " Expert estimate." } else { "" };
        out.push(format!(
            "{}. **{}.** {}{year}. {} (retrieved {}).{prior}",
            i + 1,
            md(&c.title),
            md(&c.publisher),
            c.url,
            text::date(c.retrieved)
        ));
    }
    out.push(String::new());
    if !a.attributions.is_empty() {
        out.push("### Data credits".to_owned());
        out.push(String::new());
        for at in &a.attributions {
            let version = at
                .version
                .as_deref()
                .map(|v| format!(", version {v}"))
                .unwrap_or_default();
            out.push(format!(
                "> **{}{version}, accessed {}.** {} {}",
                md(&at.source),
                text::date(at.accessed),
                md(&at.text),
                at.url
            ));
            out.push(String::new());
        }
    }
}
