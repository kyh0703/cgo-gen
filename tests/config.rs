use std::{env, fs};

use c_go::config::{Config, HeaderRole};

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

#[test]
fn loads_file_level_model_and_facade_classification() {
    let mut dir = env::temp_dir();
    dir.push(format!(
        "c_go_config_file_roles_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/model.hpp"), "class ModelThing {};").unwrap();
    fs::write(dir.join("include/facade.hpp"), "int init();").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  headers:
    - include/model.hpp
    - include/facade.hpp
files:
  model:
    - include/model.hpp
  facade:
    - include/facade.hpp
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    assert_eq!(
        config.header_role(&config.input.headers[0]),
        HeaderRole::Model
    );
    assert_eq!(
        config.header_role(&config.input.headers[1]),
        HeaderRole::Facade
    );
}

#[test]
fn rejects_overlapping_file_roles() {
    let mut dir = env::temp_dir();
    dir.push(format!(
        "c_go_config_file_role_overlap_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/shared.hpp"), "class Shared {};").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  headers:
    - include/shared.hpp
files:
  model:
    - include/shared.hpp
  facade:
    - include/shared.hpp
output:
  dir: gen
"#,
    )
    .unwrap();

    let error = Config::load(&config_path).unwrap_err().to_string();
    assert!(error.contains("both model and facade"));
}

#[test]
fn loads_sil_wrapper_example_config_with_expected_roles() {
    let config = Config::load("configs/sil-wrapper.example.yaml").unwrap();

    assert_eq!(config.naming.prefix, "sil");
    assert_eq!(config.input.headers.len(), 2);
    assert_eq!(config.files.model.len(), 1);
    assert_eq!(config.files.facade.len(), 1);
    assert_eq!(
        config.header_role(&config.files.model[0]),
        HeaderRole::Model
    );
    assert_eq!(
        config.header_role(&config.files.facade[0]),
        HeaderRole::Facade
    );
    assert!(config.output.dir.ends_with("configs/pkg/sil"));
}

#[test]
fn sil_wrapper_example_scopes_per_header_output_names() {
    let config = Config::load("configs/sil-wrapper.example.yaml").unwrap();

    let master = config.scoped_to_header(config.input.headers[0].clone());
    let user = config.scoped_to_header(config.input.headers[1].clone());

    assert_eq!(master.output.header, "is_aa_master_wrapper.h");
    assert_eq!(master.output.source, "is_aa_master_wrapper.cpp");
    assert_eq!(master.output.ir, "is_aa_master_wrapper.ir.yaml");
    assert_eq!(master.go_filename(""), "is_aa_master_wrapper.go");
    assert_eq!(
        master.header_role(&master.input.headers[0]),
        HeaderRole::Model
    );

    assert_eq!(user.output.header, "is_aa_user_wrapper.h");
    assert_eq!(user.output.source, "is_aa_user_wrapper.cpp");
    assert_eq!(user.output.ir, "is_aa_user_wrapper.ir.yaml");
    assert_eq!(user.go_filename(""), "is_aa_user_wrapper.go");
    assert_eq!(user.header_role(&user.input.headers[0]), HeaderRole::Facade);
}
