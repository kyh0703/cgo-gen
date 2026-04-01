use std::{env, fs};

use c_go::{config::Config, parser};

fn temp_dir(label: &str) -> std::path::PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_tu_parsing_test_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn standalone_header_parse_fails_when_dependency_is_only_provided_by_translation_unit() {
    let root = temp_dir("standalone-fails");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();

    fs::write(root.join("include/dep.hpp"), "class Dep {};\n").unwrap();
    fs::write(
        root.join("include/model.hpp"),
        r#"
        #pragma once

        class Thing {
        public:
            Thing() = default;
            void SetDep(Dep& dep);
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
    - include/model.hpp
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let error = parser::parse(&config).unwrap_err().to_string();

    assert!(error.contains("unknown type name 'Dep'"));
}

#[test]
fn dirs_based_tu_parse_collects_target_header_declarations() {
    let root = temp_dir("dirs-succeeds");
    fs::create_dir_all(root.join("module")).unwrap();

    fs::write(root.join("module/dep.hpp"), "class Dep {};\n").unwrap();
    fs::write(
        root.join("module/model.hpp"),
        r#"
        #pragma once

        class Thing {
        public:
            Thing() = default;
            void SetDep(Dep& dep);
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("module/entry.cpp"),
        r#"
        #include "dep.hpp"
        #include "model.hpp"
        "#,
    )
    .unwrap();

    let config_path = root.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dirs:
    - module
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let parsed = parser::parse(&config).unwrap();

    assert_eq!(parsed.classes.len(), 1);
    assert_eq!(parsed.classes[0].name, "Thing");
}
