use console::style;
use jiff::{Unit, Zoned};
use similar::{ChangeTag, DiffOp, TextDiff};

// pub trait Fragment {
//     fn fragment(&self, fragment: &mut IndexMap<String, String>);
//     fn as_fragment(&self) -> String {
//         let mut fragment = IndexMap::new();
//         self.fragment(&mut fragment);
//         fragment.en
//     }
// }

pub fn time_diff(zoned: &Zoned, unit: Unit, threshold: f64, smallest: Unit) -> String {
    let span = (zoned - &jiff::Zoned::now())
        .round(
            jiff::SpanRound::new()
                .largest(jiff::Unit::Month)
                .smallest(smallest)
                .relative(zoned)
                .mode(jiff::RoundMode::Trunc),
        )
        .unwrap();

    if span.total((unit, zoned)).unwrap().abs() < threshold {
        style(format!("{span:#}")).green().bold()
    } else {
        style(format!("{span:#}")).red().bold()
    }
    .to_string()
}

pub fn diff_inline<T: similar::DiffableStrRef + ?Sized>(old: &T, new: &T) -> String {
    let diff = TextDiff::configure().diff_unicode_words(old, new);

    let mut output = String::new();

    for op in diff.ops() {
        let change = match op {
            DiffOp::Replace { .. } => {
                let mut replace_output = String::new();
                let diffs = diff.iter_changes(op).collect::<Vec<_>>();
                for index in 0..diffs.len() {
                    let change = &diffs[index];
                    // we need the change from insert -> deleted so that we can input the arrow
                    // This is because each word is a change and not only the before / after
                    let next = diffs.get(index + 1).unwrap_or(change);

                    let styled_output = match change.tag() {
                        ChangeTag::Equal => change.to_string_lossy().to_string(),
                        ChangeTag::Delete => style(change.to_string_lossy()).red().to_string(),
                        ChangeTag::Insert => style(change.to_string_lossy()).green().to_string(),
                    };
                    replace_output.push_str(styled_output.as_str());
                    // if the tag (+/-) changes we input an arrow
                    if change.tag() != next.tag() {
                        replace_output.push_str(" -> ");
                    }
                }
                replace_output
            }
            // This branch means it does not have an equivalent in the new text
            DiffOp::Equal { .. } | DiffOp::Delete { .. } | DiffOp::Insert { .. } => {
                let change = diff
                    .iter_changes(op)
                    .map(|c| c.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("");
                if matches!(op, DiffOp::Delete { .. }) {
                    style(change).red().to_string()
                } else if matches!(op, DiffOp::Insert { .. }) {
                    style(change).green().to_string()
                } else {
                    change
                }
            }
        };
        output.push_str(change.as_str());
    }
    output
}

#[cfg(test)]
mod test_display {
    use console::strip_ansi_codes;

    use crate::display::diff_inline;

    #[test]
    fn diff() {
        let old = r#"[✨] aegis (New)
 • Version: 8234757c917ea6a8
 • Last Seen: Never ⏳
 • Comment: Hi there, i wont last long so listen
 Also: you very beautiful"#;

        let new = r#"[✅] aegis (UpToDate)
 • Version: 167510b529f7c924
 • Last Seen: Never ⏳
 • Comment: quick
 Also: are very today"#;

        let expected = r#"[✨ -> ✅] aegis (New -> UpToDate)
 • Version: 8234757c917ea6a8 -> 167510b529f7c924
 • Last Seen: Never ⏳
 • Comment: Hi there, i wont last long so listen -> quick
 Also: you -> are very beautiful -> today"#;

        assert_eq!(expected, strip_ansi_codes(&diff_inline(old, new)))
    }
}
