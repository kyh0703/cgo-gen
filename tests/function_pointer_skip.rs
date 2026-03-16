use std::{env, fs, path::PathBuf};

use c_go::{config::Config, ir, parser};

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
        typedef void (*Callback)(int code);

        int add(int lhs, int rhs);
        void set_callback(Callback cb);

        class Api {
        public:
            int GetValue() const;
            void SetCallback(Callback cb);
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
            .any(|item| item.cpp_name == "set_callback" && item.reason.contains("Callback"))
    );
    assert!(
        ir.support
            .skipped_declarations
            .iter()
            .any(|item| item.cpp_name == "Api::SetCallback" && item.reason.contains("Callback"))
    );
}
