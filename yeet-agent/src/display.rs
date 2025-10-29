use std::io::Write as _;

use anyhow::ensure;
use api::hash_hex;
use console::style;
use jiff::Zoned;
use similar::{ChangeTag, DiffOp, TextDiff};

// pub trait Fragment {
//     fn fragment(&self, fragment: &mut IndexMap<String, String>);
//     fn as_fragment(&self) -> String {
//         let mut fragment = IndexMap::new();
//         self.fragment(&mut fragment);
//         fragment.en
//     }
// }

pub fn host(host: &api::Host) -> anyhow::Result<String> {
    let status_emoji = match host.status {
        api::HostState::New => "✨",
        api::HostState::Detached => "🫥",
        api::HostState::Provisioned(api::ProvisionState::UpToDate) => "✅",
        api::HostState::Provisioned(api::ProvisionState::NewVersionAvailable(_)) => "🔄",
    };

    let status = match host.status {
        api::HostState::New => " (New)",
        api::HostState::Detached => " (Detached)",
        api::HostState::Provisioned(api::ProvisionState::UpToDate) => " (Latest)",
        api::HostState::Provisioned(api::ProvisionState::NewVersionAvailable(_)) => {
            " (Will Update)"
        }
    };

    let key_status = match host.key {
        api::Key::Verified(_) => "",
        api::Key::Unverified => " (Unverified)",
    };

    let mut w = Vec::new();
    writeln!(&mut w, "[{status_emoji}] {}{status}{key_status}", host.name)?;

    if host.store_path.is_empty() {
        ensure!(host.last_ping == None);
        writeln!(&mut w, " • Version: Host not rolled out",)?;
    } else {
        writeln!(&mut w, " • Version: {}", hash_hex(&host.store_path))?;
    }

    if let api::HostState::Provisioned(api::ProvisionState::NewVersionAvailable(ref next_version)) =
        host.status
    {
        writeln!(
            &mut w,
            " • Next Version: {}",
            hash_hex(&next_version.store_path)
        )?;
    }

    writeln!(
        &mut w,
        " • Last Seen: {}",
        host.last_ping.clone().map_or("Never".to_owned(), |zoned| {
            format!("{:#}", &Zoned::now() - &zoned)
        })
    )?;

    Ok(String::from_utf8(w)?)
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
                    let change = diffs.get(index).unwrap();
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
