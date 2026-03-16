use std::{env, fs, path::PathBuf};

use c_go::{config::Config, ir, parser};

fn temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_typedef_alias_resolution_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("include")).unwrap();
    path
}

#[test]
fn resolves_typedef_aliases_via_canonical_types() {
    let root = temp_dir("generate");
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        typedef unsigned int ModuleId;
        typedef int ResultCode;

        ModuleId get_id();
        ResultCode reset_id(ModuleId value);

        class Api {
        public:
            ModuleId GetId() const;
            ResultCode SetId(ModuleId value);
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

    let get_id = ir
        .functions
        .iter()
        .find(|item| item.name == "cgowrap_get_id")
        .unwrap();
    assert_eq!(get_id.returns.cpp_type, "ModuleId");
    assert_eq!(get_id.returns.c_type, "unsigned int");

    let reset_id = ir
        .functions
        .iter()
        .find(|item| item.name == "cgowrap_reset_id")
        .unwrap();
    assert_eq!(reset_id.returns.cpp_type, "ResultCode");
    assert_eq!(reset_id.returns.c_type, "int");
    assert_eq!(reset_id.params[0].ty.cpp_type, "ModuleId");
    assert_eq!(reset_id.params[0].ty.c_type, "unsigned int");

    let method = ir
        .functions
        .iter()
        .find(|item| item.name == "cgowrap_Api_SetId")
        .unwrap();
    assert_eq!(method.returns.cpp_type, "ResultCode");
    assert_eq!(method.params[1].ty.cpp_type, "ModuleId");
}
