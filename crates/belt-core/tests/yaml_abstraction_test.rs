//! Integration tests for `belt_core::yaml` abstraction layer.
//!
//! Test files are separate compilation units, so the `cfg_attr(test, allow(...))`
//! in `lib.rs` does NOT apply. Allow the panic-adjacent lints locally for tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use belt_core::yaml;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Simple {
    name: String,
    count: u32,
}

#[test]
fn parses_typed_value_via_abstraction() {
    let yaml_text = "name: belt\ncount: 3\n";
    let v: Simple = yaml::parse(yaml_text).expect("parse ok");
    assert_eq!(v.name, "belt");
    assert_eq!(v.count, 3);
}

#[test]
fn parses_dynamic_value() {
    let yaml_text = "a: 1\nb: [2, 3]\n";
    let v = yaml::parse_value(yaml_text).expect("parse ok");
    assert!(v.is_mapping());
}

#[test]
fn reports_duplicate_key_as_error_by_default() {
    let yaml_text = "a: 1\na: 2\n";
    let err = yaml::parse_value(yaml_text).expect_err("expected duplicate-key error");
    assert!(format!("{err}").contains("duplicate"));
}

#[test]
fn serializes_typed_value() {
    let v = Simple {
        name: "belt".into(),
        count: 7,
    };
    let out = yaml::serialize(&v).expect("serialize ok");
    assert!(out.contains("name: belt"));
    assert!(out.contains("count: 7"));
}

#[test]
fn round_trip_simple_struct() {
    let v = Simple {
        name: "belt".into(),
        count: 7,
    };
    let out = yaml::serialize(&v).expect("serialize ok");
    let back: Simple = yaml::parse(&out).expect("parse ok");
    assert_eq!(back.name, "belt");
    assert_eq!(back.count, 7);
}

#[test]
fn quote_rule_reserved_word_true() {
    // A struct field whose string value equals the YAML reserved word "true"
    // must be emitted quoted so that re-parsing yields a string (not a bool).
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Wrapper {
        value: String,
    }
    let v = Wrapper {
        value: "true".into(),
    };
    let out = yaml::serialize(&v).expect("serialize ok");
    // The emitted form must contain the explicitly quoted value.
    assert!(
        out.contains("\"true\""),
        "expected quoted form in output: {out}"
    );
    // Round-trip keeps it as a string, not a bool.
    let back: Wrapper = yaml::parse(&out).expect("parse ok");
    assert_eq!(back.value, "true");
}

#[test]
fn quote_rule_numeric_string() {
    // A string that parses as integer must be quoted to survive round-trip.
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Wrapper {
        value: String,
    }
    let v = Wrapper {
        value: "123".into(),
    };
    let out = yaml::serialize(&v).expect("serialize ok");
    assert!(
        out.contains("\"123\""),
        "expected quoted numeric form: {out}"
    );
    let back: Wrapper = yaml::parse(&out).expect("parse ok");
    assert_eq!(back.value, "123");
}

#[test]
fn to_value_and_back() {
    let v = Simple {
        name: "belt".into(),
        count: 42,
    };
    let yv = yaml::to_value(&v).expect("to_value ok");
    assert!(yv.is_mapping());
    let back: Simple = yaml::from_value(yv).expect("from_value ok");
    assert_eq!(back, v);
}

#[test]
fn value_get_helpers() {
    let yaml_text = "name: belt\ncount: 3\nitems:\n  - a\n  - b\n";
    let v = yaml::parse_value(yaml_text).expect("parse ok");
    let m = v.as_mapping().expect("is mapping");
    let name = m
        .get(&yaml::Value::String("name".into()))
        .expect("name present");
    assert_eq!(name.as_str(), Some("belt"));
    let count = v.get("count").expect("count present").as_i64();
    assert_eq!(count, Some(3));
    let items = v.get("items").expect("items present");
    assert!(items.is_sequence());
    assert_eq!(items.as_sequence().map(<[_]>::len), Some(2));
}

#[test]
fn round_trip_string_with_leading_dash_whitespace() {
    let original: Vec<String> = vec!["-v".into(), "--help".into(), "- foo".into()];
    let yaml = yaml::serialize(&original).expect("serialize ok");
    let back: Vec<String> = yaml::parse(&yaml).expect("parse ok");
    assert_eq!(back, original);
}

#[test]
fn round_trip_string_with_leading_question_mark() {
    let original: Vec<String> = vec!["? key".into(), "?".into()];
    let yaml = yaml::serialize(&original).expect("serialize ok");
    let back: Vec<String> = yaml::parse(&yaml).expect("parse ok");
    assert_eq!(back, original);
}

#[test]
fn round_trip_string_document_markers() {
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct Doc {
        marker: String,
    }
    let original = Doc {
        marker: "---".into(),
    };
    let yaml = yaml::serialize(&original).expect("serialize ok");
    let back: Doc = yaml::parse(&yaml).expect("parse ok");
    assert_eq!(back, original);
}
