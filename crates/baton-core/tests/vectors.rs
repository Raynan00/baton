use baton_core::{pb, restore_url};
use serde_json::Value;

#[test]
fn restore_url_vectors() {
    let v: Vec<Value> = serde_json::from_str(include_str!("vectors/restore_url.json")).unwrap();
    assert!(!v.is_empty());
    for c in v {
        let cont = pb::Continue {
            url: c["url"].as_str().unwrap().into(),
            text_fragment: c["fragment"].as_str().unwrap().into(),
            media: c["t"].as_f64().map(|t| pb::Media {
                current_time_s: t,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(restore_url::build(&cont), c["want"].as_str().unwrap());
    }
}
