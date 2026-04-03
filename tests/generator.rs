use cgo_gen::{
    config::Config,
    generator::{render_go_structs, render_header, render_source},
    ir, parser,
};
use std::{env, fs};

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
    let root = std::env::temp_dir().join(format!("c_go_const_char_value_{}", std::process::id()));
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
    let root = std::env::temp_dir().join(format!("c_go_typedef_enum_alias_{}", std::process::id()));
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
    let go = render_go_structs(&config, &ir).unwrap();

    assert!(parsed.enums.iter().any(|item| item.name == "FooState"));
    assert!(!header.contains("FooState"));
    assert!(go[0].contents.contains("type FooState int64"));
    assert!(go[0].contents.contains("FooDisabled FooState = 0"));
    assert!(go[0].contents.contains("FooEnabled FooState = 1"));
}

#[test]
fn normalizes_primitive_alias_pointer_and_reference_c_types_in_header() {
    let root =
        std::env::temp_dir().join(format!("c_go_alias_pointer_header_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Api.hpp"),
        r#"
        #include <stdint.h>
        typedef int32_t int32;
        typedef uint32_t uint32;

        bool TakeAliasPtr(int32* value);
        bool TakeAliasRef(uint32& value);
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

    let ptr = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "TakeAliasPtr")
        .unwrap();
    assert_eq!(ptr.params[0].ty.cpp_type, "int32*");
    assert_eq!(ptr.params[0].ty.c_type, "int32_t*");

    let r#ref = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "TakeAliasRef")
        .unwrap();
    assert_eq!(r#ref.params[0].ty.cpp_type, "uint32&");
    assert_eq!(r#ref.params[0].ty.c_type, "uint32_t*");

    assert!(header.contains("bool cgowrap_TakeAliasPtr(int32_t* value);"));
    assert!(header.contains("bool cgowrap_TakeAliasRef(uint32_t* value);"));
    assert!(!header.contains("int32* value"));
    assert!(!header.contains("uint32* value"));
}

#[test]
fn generate_with_go_module_writes_build_flags_and_go_mod() {
    let root = env::temp_dir().join(format!("c_go_go_package_metadata_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(root.join("include/Api.hpp"), "int Add(int lhs, int rhs);").unwrap();
    fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  headers:
    - include/Api.hpp
  clang_args:
    - -I${SDK_INCLUDE}
    - -DMODE=1
    - -std=c++20
    - -Wall
    - -Winvalid-offsetof
output:
  dir: out
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("SDK_INCLUDE", root.join("sdk/include"));
    }

    let config = Config::load(root.join("config.yaml"))
        .unwrap()
        .with_go_module(Some("example.com/demo/pkg".to_string()));
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    cgo_gen::generator::generate(&config, &ir, false).unwrap();

    let go_mod = fs::read_to_string(root.join("out/go.mod")).unwrap();
    assert_eq!(go_mod, "module example.com/demo/pkg\n\ngo 1.25\n");

    let build_flags = fs::read_to_string(root.join("out/build_flags.go")).unwrap();
    assert!(build_flags.contains("package out"));
    assert!(build_flags.contains("#cgo CFLAGS: -I${SRCDIR}"));
    assert!(
        build_flags.contains("#cgo CXXFLAGS: -I${SRCDIR} -I${SDK_INCLUDE} -DMODE=1 -std=c++20")
    );
    assert!(!build_flags.contains("sdk/include"));
    assert!(!build_flags.contains("-Winvalid-offsetof"));
    assert!(!build_flags.contains("-Wall"));
}

#[test]
fn renders_model_value_return_as_owned_handle_copy() {
    let root = env::temp_dir().join(format!("c_go_model_value_return_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        class MTime {
        public:
            MTime() : value_(7) {}
            int GetValue() const { return value_; }
        private:
            int value_;
        };

        class Api {
        public:
            MTime GetCreateTime() const { return MTime(); }
        };
        "#,
    )
    .unwrap();
    fs::write(
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
    let source = render_source(&config, &ir);

    let getter = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "Api::GetCreateTime")
        .unwrap();
    assert_eq!(getter.returns.kind, "model_value");
    assert_eq!(getter.returns.c_type, "MTimeHandle*");
    assert!(header.contains("MTimeHandle* cgowrap_Api_GetCreateTime(const ApiHandle* self);"));
    assert!(source.contains(
        "return reinterpret_cast<MTimeHandle*>(new MTime(reinterpret_cast<const Api*>(self)->GetCreateTime()));"
    ));
}
