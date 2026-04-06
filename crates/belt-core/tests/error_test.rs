use belt_core::error::BeltError;
use miette::Diagnostic;

#[test]
fn config_parse_error_display_includes_path_and_detail() {
    let err = BeltError::ConfigParse {
        path: "belt.toml".to_string(),
        detail: "expected `=`, found newline".to_string(),
    };
    assert_eq!(
        err.to_string(),
        "config parse error in belt.toml: expected `=`, found newline"
    );
}

#[test]
fn config_parse_error_diagnostic_code() {
    let err = BeltError::ConfigParse {
        path: "belt.toml".to_string(),
        detail: "missing field".to_string(),
    };
    let code = err
        .code()
        .expect("ConfigParse should have a diagnostic code");
    assert_eq!(code.to_string(), "belt::config_parse");
}
