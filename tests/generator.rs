use c_go::{
    config::Config,
    generator::{render_go_structs, render_header, render_source},
    ir, parser,
};

#[test]
fn renders_header_and_source_from_fixture() {
    let config = Config::load("tests/fixtures/simple/config.yaml").unwrap();
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();

    let header = render_header(&config, &ir);
    assert!(header.contains("typedef struct fooBarHandle fooBarHandle;"));
    assert!(header.contains("fooBarHandle* cgowrap_foo_bar_new(int value);"));
    assert!(header.contains("char* cgowrap_foo_bar_name(const fooBarHandle* self);"));

    let source = render_source(&config, &ir);
    assert!(source.contains(&format!("#include \"{}\"", config.output.header)));
    assert!(source.contains("return reinterpret_cast<fooBarHandle*>(new foo::Bar(value));"));
    assert!(source.contains("delete reinterpret_cast<foo::Bar*>(self);"));
}

#[test]
fn renders_unified_go_wrapper() {
    let config = Config::load("tests/fixtures/simple/config.yaml").unwrap();
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();

    let go = render_go_structs(&config, &ir).unwrap();
    assert_eq!(go.len(), 1);
    assert!(go[0].contents.contains("type Bar struct {"));
    assert!(go[0].contents.contains("func Add(lhs int, rhs int) int {"));
}

#[test]
fn preserves_const_char_spelling_but_normalizes_c_value_type() {
    let root = std::env::temp_dir().join(format!(
        "c_go_const_char_value_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Api.hpp"),
        r#"
        class Api {
        public:
            const char GetMarker() const { return 'A'; }
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
    - include/Api.hpp
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();

    let marker = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "Api::GetMarker")
        .unwrap();
    assert_eq!(marker.returns.cpp_type, "const char");
    assert_eq!(marker.returns.c_type, "char");
}

#[test]
fn renders_typedef_anonymous_enums_with_alias_name() {
    let root = std::env::temp_dir().join(format!(
        "c_go_typedef_enum_alias_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Api.hpp"),
        r#"
        typedef enum {
            FooDisabled = 0,
            FooEnabled = 1,
        } FooState;
        "#,
    )
    .unwrap();
    std::fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  headers:
    - include/Api.hpp
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    let header = render_header(&config, &ir);

    assert!(parsed.enums.iter().any(|item| item.name == "FooState"));
    assert!(header.contains("typedef enum FooState {"));
    assert!(header.contains("} FooState;"));
    assert!(!header.contains("(unnamed enum at"));
}
