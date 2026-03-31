use std::{env, fs, path::Path};

use c_go::config::{Config, HeaderRole};

fn normalize_expected_path(path: &Path) -> String {
    let value = path.canonicalize().unwrap().display().to_string();
    if cfg!(windows) {
        value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
    } else {
        value
    }
}

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
    assert!(!config.input.allow_diagnostics);
    assert_eq!(config.output.header, "foo_wrapper.h");
    assert!(config.input.headers[0].is_absolute());
}

#[test]
fn loads_optional_allow_diagnostics_flag() {
    let mut dir = env::temp_dir();
    dir.push(format!(
        "c_go_config_allow_diagnostics_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/foo.hpp"), "int foo();").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  headers:
    - include/foo.hpp
  allow_diagnostics: true
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    assert!(config.input.allow_diagnostics);
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
    assert!(config.raw_output_dir().ends_with("gen/raw"));
    assert!(config.model_output_dir().ends_with("gen"));
    assert!(config.facade_output_dir().ends_with("gen"));
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

#[test]
fn resolves_relative_clang_include_args_from_config_dir() {
    let mut dir = env::temp_dir();
    dir.push(format!(
        "c_go_config_relative_clang_args_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::create_dir_all(dir.join("deps/inc")).unwrap();
    fs::create_dir_all(dir.join("deps/sys")).unwrap();
    fs::write(dir.join("include/foo.hpp"), "int foo();").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  headers:
    - include/foo.hpp
  clang_args:
    - -Ideps/inc
    - -isystem
    - deps/sys
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();

    assert_eq!(
        config.input.clang_args,
        vec![
            format!("-I{}", normalize_expected_path(&dir.join("deps/inc"))),
            "-isystem".to_string(),
            normalize_expected_path(&dir.join("deps/sys")),
        ]
    );
}

#[test]
fn loads_real_sil_model_config() {
    let config = Config::load("configs/sil-real-model.yaml").unwrap();

    assert_eq!(config.naming.prefix, "sil");
    assert_eq!(config.input.headers.len(), 1);
    assert_eq!(config.files.model.len(), 1);
    assert_eq!(
        config.header_role(&config.files.model[0]),
        HeaderRole::Model
    );
    assert!(
        config.output.dir.ends_with(
            Path::new("IPRON")
                .join("IE")
                .join("PSC")
                .join("pkg")
                .join("sil")
        )
    );
}
