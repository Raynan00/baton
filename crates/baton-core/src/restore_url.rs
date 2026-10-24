//! Restore-URL builder (BATON.md §7.1): YouTube `t=` and text fragments.
use crate::pb;
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use url::Url;

/// Keep already-encoded text (`%`) and directive punctuation (`-`, `,`, `_`, `.`, `~`) intact.
const FRAGMENT_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'%')
    .remove(b'-')
    .remove(b',')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

const TRACKING: [&str; 2] = ["fbclid", "gclid"];

/// Strip tracking params (`utm_*`, `fbclid`, `gclid`). Returns `None` for unparsable input.
pub fn canonicalize(raw: &str) -> Option<String> {
    let mut u = Url::parse(raw).ok()?;
    let kept: Vec<(String, String)> = u
        .query_pairs()
        .filter(|(k, _)| !k.starts_with("utm_") && !TRACKING.contains(&k.as_ref()))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if kept.is_empty() {
        u.set_query(None);
    } else {
        u.query_pairs_mut().clear().extend_pairs(kept);
    }
    Some(u.to_string())
}

fn youtube_id(u: &Url) -> Option<String> {
    let host = u
        .host_str()?
        .trim_start_matches("www.")
        .trim_start_matches("m.");
    match host {
        "youtu.be" => u
            .path_segments()?
            .next()
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
        "youtube.com" if u.path() == "/watch" => u
            .query_pairs()
            .find(|(k, _)| k == "v")
            .map(|(_, v)| v.into_owned()),
        _ => None, // includes /shorts (excluded by spec)
    }
}

/// Build the URL the iPhone opens: YouTube at the saved second, article at the saved paragraph,
/// otherwise the plain URL.
pub fn build(c: &pb::Continue) -> String {
    let Some(u) = Url::parse(&c.url).ok() else {
        return c.url.clone();
    };
    if let Some(id) = youtube_id(&u) {
        let t = c
            .media
            .as_ref()
            .map_or(0.0, |m| m.current_time_s)
            .max(0.0)
            .floor() as u64;
        return format!("https://youtu.be/{id}?t={t}");
    }
    if c.text_fragment.is_empty() {
        return u.to_string();
    }
    let directive = utf8_percent_encode(&c.text_fragment, FRAGMENT_SET);
    let mut u = u;
    let base = match u.fragment() {
        // Existing `#hash`: append the directive as `#hash:~:text=…`.
        Some(f) if !f.is_empty() && !f.contains(":~:") => format!("{f}:~:text={directive}"),
        _ => format!(":~:text={directive}"),
    };
    u.set_fragment(Some(&base));
    u.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cont(url: &str, frag: &str, yt_t: Option<f64>) -> pb::Continue {
        pb::Continue {
            url: url.into(),
            text_fragment: frag.into(),
            media: yt_t.map(|t| pb::Media {
                current_time_s: t,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn table() {
        let cases = [
            (
                "https://www.youtube.com/watch?v=abc123&list=x",
                "",
                Some(761.9),
                "https://youtu.be/abc123?t=761",
            ),
            (
                "https://youtu.be/abc123",
                "",
                Some(5.0),
                "https://youtu.be/abc123?t=5",
            ),
            (
                "https://www.youtube.com/shorts/abc",
                "",
                Some(5.0),
                "https://www.youtube.com/shorts/abc",
            ),
            (
                "https://ex.com/a",
                "Ownership%20rules",
                None,
                "https://ex.com/a#:~:text=Ownership%20rules",
            ),
            (
                "https://ex.com/a",
                "two words",
                None,
                "https://ex.com/a#:~:text=two%20words",
            ),
            (
                "https://ex.com/a#sec",
                "x",
                None,
                "https://ex.com/a#sec:~:text=x",
            ),
            ("https://ex.com/a", "", None, "https://ex.com/a"),
        ];
        for (url, frag, t, want) in cases {
            assert_eq!(build(&cont(url, frag, t)), want, "{url}");
        }
    }

    #[test]
    fn strips_tracking() {
        assert_eq!(
            canonicalize("https://ex.com/p?a=1&utm_source=x&fbclid=y&gclid=z").unwrap(),
            "https://ex.com/p?a=1"
        );
        assert_eq!(
            canonicalize("https://ex.com/p?utm_medium=m").unwrap(),
            "https://ex.com/p"
        );
    }
}
