use c_go::{config::Config, ir, parser};

#[test]
fn fails_when_declared_constructors_are_filtered_out() {
    let config = Config::load("tests/fixtures/ctor_filter/config.yaml").unwrap();
    let parsed = parser::parse(&config).unwrap();

    assert_eq!(parsed.classes.len(), 1);
    assert!(parsed.classes[0].has_declared_constructor);
    assert!(parsed.classes[0].constructors.is_empty());

    let error = ir::normalize(&config, &parsed).unwrap_err().to_string();
    assert!(error.contains("safe::Gadget"));
    assert!(error.contains("refusing to synthesize a default constructor"));
}
