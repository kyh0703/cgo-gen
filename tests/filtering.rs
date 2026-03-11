use c_go::{config::Config, parser};

#[test]
fn supports_qualified_and_exclude_filters() {
    let config = Config::load("tests/fixtures/filtering/qualified.yaml").unwrap();
    let parsed = parser::parse(&config).unwrap();

    assert_eq!(parsed.classes.len(), 1);
    assert_eq!(parsed.enums.len(), 1);
    assert_eq!(parsed.functions.len(), 1);
    assert_eq!(parsed.functions[0].name, "add");
    assert_eq!(parsed.classes[0].methods.len(), 1);
    assert_eq!(parsed.classes[0].methods[0].name, "name");
    assert!(parsed.classes[0].has_destructor);
    assert_eq!(parsed.classes[0].constructors.len(), 1);
}

#[test]
fn supports_type_filters_for_methods_and_functions() {
    let config = Config::load("tests/fixtures/filtering/type_filter.yaml").unwrap();
    let parsed = parser::parse(&config).unwrap();

    assert_eq!(parsed.classes.len(), 1);
    assert_eq!(parsed.classes[0].methods.len(), 2);
    assert!(
        parsed.classes[0]
            .methods
            .iter()
            .any(|method| method.name == "name")
    );
    assert!(
        parsed.classes[0]
            .methods
            .iter()
            .any(|method| method.name == "set_label")
    );
    assert_eq!(parsed.functions.len(), 0);
    assert_eq!(parsed.enums.len(), 1);
}
