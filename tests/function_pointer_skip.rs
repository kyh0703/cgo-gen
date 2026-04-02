use std::{env, fs, path::PathBuf};

use cgo_gen::{config::Config, ir, parser};

fn temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_function_pointer_skip_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("include")).unwrap();
    path
}

#[test]
fn skips_declarations_using_function_pointer_types() {
    let root = temp_dir("generate");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        int add(int lhs, int rhs);
        void set_callback(void (*cb)(int code));

        class Api {
        public:
            int GetValue() const;
            void SetCallback(void (*cb)(int code));
        };
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  headers:
    - include/Api.hpp
output:
  dir: out
naming:
  prefix: cgowrap
  style: preserve
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();

    assert!(ir.functions.iter().any(|item| item.name == "cgowrap_add"));
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_Api_GetValue")
    );
    assert!(
        !ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_set_callback")
    );
    assert!(
        !ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_Api_SetCallback")
    );

    assert_eq!(ir.support.skipped_declarations.len(), 2);
    assert!(
        ir.support
            .skipped_declarations
            .iter()
            .any(|item| item.cpp_name == "set_callback" && item.reason.contains("function pointer"))
    );
    assert!(
        ir.support
            .skipped_declarations
            .iter()
            .any(|item| item.cpp_name == "Api::SetCallback" && item.reason.contains("function pointer"))
    );
}

#[test]
fn skips_operator_declarations() {
    let root = temp_dir("operators");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        class Value {
        public:
            Value operator+(const Value& rhs) const;
            bool operator==(const Value& rhs) const;
            int GetCode() const;
        };

        Value operator-(const Value& lhs, const Value& rhs);
        int plain_add(int lhs, int rhs);
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  headers:
    - include/Api.hpp
output:
  dir: out
naming:
  prefix: cgowrap
  style: preserve
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();

    assert!(ir.functions.iter().any(|item| item.name == "cgowrap_plain_add"));
    assert!(ir.functions.iter().any(|item| item.name == "cgowrap_Value_GetCode"));
    assert!(
        !ir.functions
            .iter()
            .any(|item| item.cpp_name.contains("operator+"))
    );
    assert!(
        !ir.functions
            .iter()
            .any(|item| item.cpp_name.contains("operator=="))
    );
    assert!(
        !ir.functions
            .iter()
            .any(|item| item.cpp_name.contains("operator-"))
    );

    assert!(
        ir.support
            .skipped_declarations
            .iter()
            .any(|item| item.cpp_name == "Value::operator+" && item.reason.contains("operator declarations"))
    );
    assert!(
        ir.support
            .skipped_declarations
            .iter()
            .any(|item| item.cpp_name == "Value::operator==" && item.reason.contains("operator declarations"))
    );
    assert!(
        ir.support
            .skipped_declarations
            .iter()
            .any(|item| item.cpp_name == "operator-" && item.reason.contains("operator declarations"))
    );
}
