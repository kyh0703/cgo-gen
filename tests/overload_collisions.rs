use cgo_gen::{
    config::Config,
    generator::{render_header, render_source},
    ir, parser,
};

#[test]
fn disambiguates_overloaded_free_functions_with_signature_suffixes() {
    let config = Config::load("tests/fixtures/overload/free_function.yaml").unwrap();
    let parsed = parser::parse(&config).unwrap();

    assert_eq!(parsed.functions.len(), 2);

    let ir = ir::normalize(&config, &parsed).unwrap();
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_clash_add__int_int")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_clash_add__double_double")
    );

    let header = render_header(&config, &ir);
    let source = render_source(&config, &ir);
    assert!(header.contains("int cgowrap_clash_add__int_int(int lhs, int rhs);"));
    assert!(header.contains("double cgowrap_clash_add__double_double(double lhs, double rhs);"));
    assert!(source.contains("int cgowrap_clash_add__int_int(int lhs, int rhs)"));
    assert!(source.contains("double cgowrap_clash_add__double_double(double lhs, double rhs)"));
}

#[test]
fn disambiguates_overloaded_methods_with_signature_suffixes() {
    let config = Config::load("tests/fixtures/overload/method.yaml").unwrap();
    let parsed = parser::parse(&config).unwrap();

    assert_eq!(parsed.classes.len(), 1);
    assert_eq!(parsed.classes[0].methods.len(), 2);

    let ir = ir::normalize(&config, &parsed).unwrap();
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_clash_widget_set__int_mut")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_clash_widget_set__double_mut")
    );

    let header = render_header(&config, &ir);
    let source = render_source(&config, &ir);
    assert!(
        header
            .contains("int cgowrap_clash_widget_set__int_mut(clashWidgetHandle* self, int value);")
    );
    assert!(header.contains(
        "int cgowrap_clash_widget_set__double_mut(clashWidgetHandle* self, double value);"
    ));
    assert!(source.contains("cgowrap_clash_widget_set__int_mut"));
    assert!(source.contains("cgowrap_clash_widget_set__double_mut"));
}

#[test]
fn disambiguates_overloaded_constructors_without_panicking() {
    let root = std::env::temp_dir().join(format!("c_go_overload_ctor_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Widget.hpp"),
        r#"
        class Widget {
        public:
            Widget() {}
            Widget(int value) {}
        };
        "#,
    )
    .unwrap();
    std::fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  headers:
    - include/Widget.hpp
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();

    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_Widget_new__void")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_Widget_new__int")
    );
}
