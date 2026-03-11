use c_go::{config::Config, ir, parser};

#[test]
fn rejects_overloaded_free_functions_that_map_to_same_wrapper_symbol() {
    let config = Config::load("tests/fixtures/overload/free_function.yaml").unwrap();
    let parsed = parser::parse(&config).unwrap();

    assert_eq!(parsed.functions.len(), 2);

    let error = ir::normalize(&config, &parsed).unwrap_err().to_string();
    assert!(error.contains("overload collision detected"));
    assert!(error.contains("cgowrap_clash_add"));
    assert!(error.contains("clash::add"));
}

#[test]
fn rejects_overloaded_methods_that_map_to_same_wrapper_symbol() {
    let config = Config::load("tests/fixtures/overload/method.yaml").unwrap();
    let parsed = parser::parse(&config).unwrap();

    assert_eq!(parsed.classes.len(), 1);
    assert_eq!(parsed.classes[0].methods.len(), 2);

    let error = ir::normalize(&config, &parsed).unwrap_err().to_string();
    assert!(error.contains("overload collision detected"));
    assert!(error.contains("cgowrap_clash_widget_set"));
    assert!(error.contains("clash::Widget::set"));
}
