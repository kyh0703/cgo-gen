use std::{env, fs};

use c_go::config::Config;

#[test]
fn loads_yaml_config() {
    let mut dir = env::temp_dir();
    dir.push(format!("c_go_config_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  headers:
    - include/foo.hpp
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    assert_eq!(config.version, Some(1));
    assert_eq!(config.input.headers.len(), 1);
    assert_eq!(config.output.header, "foo_wrapper.h");
    assert!(config.input.headers[0].is_absolute());
}

#[test]
fn derives_output_filenames_from_header_stem() {
    let mut dir = env::temp_dir();
    dir.push(format!("c_go_config_basename_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  headers:
    - include/foo.hpp
filter:
  classes:
    - Foo
go_structs:
  - Foo
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    assert_eq!(config.output.header, "foo_wrapper.h");
    assert_eq!(config.output.source, "foo_wrapper.cpp");
    assert_eq!(config.output.ir, "foo_wrapper.ir.yaml");
    assert_eq!(config.go_filename("Foo"), "foo_wrapper.go");
}
