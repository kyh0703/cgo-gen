use std::{env, fs, path::Path};

use c_go::config::Config;

fn normalize_expected_path(path: &Path) -> String {
    let value = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string();
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
    assert_eq!(config.input.dir, None);
    assert_eq!(config.input.headers.len(), 1);
    assert!(!config.input.allow_diagnostics);
    assert_eq!(config.output.header, "foo_wrapper.h");
    assert!(config.input.headers[0].is_absolute());
}

#[test]
fn loads_dir_only_input_config() {
    let mut dir = env::temp_dir();
    dir.push(format!("c_go_config_dir_only_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/model.hpp"), "class ModelThing {};").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let expected_dir = dir.join("include").canonicalize().unwrap();
    assert!(config.input.headers.is_empty());
    assert_eq!(config.input.dir.as_ref(), Some(&expected_dir));
    assert_eq!(config.output.header, "wrapper.h");
    assert_eq!(config.output.source, "wrapper.cpp");
    assert_eq!(config.output.ir, "wrapper.ir.yaml");
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
fn rejects_config_without_dir_or_headers() {
    let mut dir = env::temp_dir();
    dir.push(format!("c_go_config_missing_input_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  allow_diagnostics: true
output:
  dir: gen
"#,
    )
    .unwrap();

    let error = Config::load(&config_path).unwrap_err().to_string();
    assert!(error.contains("config.input.dir or config.input.headers must be set"));
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
    assert!(config.go_output_dir().ends_with("gen"));
}

#[test]
fn scoped_header_from_dir_only_config_switches_back_to_header_mode() {
    let mut dir = env::temp_dir();
    dir.push(format!("c_go_config_scoped_dir_only_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("include/model.hpp"), "class ModelThing {};").unwrap();

    let config_path = dir.join("cppgo-wrap.yaml");
    fs::write(
        &config_path,
        r#"
version: 1
input:
  dir: include
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    let scoped = config.scoped_to_header(config.input.dir.as_ref().unwrap().join("model.hpp"));

    assert_eq!(scoped.input.dir, config.input.dir);
    assert!(scoped.input.headers.is_empty());
    assert_eq!(
        scoped.target_header,
        Some(config.input.dir.as_ref().unwrap().join("model.hpp"))
    );
    assert_eq!(scoped.output.header, "model_wrapper.h");
    assert_eq!(scoped.output.source, "model_wrapper.cpp");
    assert_eq!(scoped.output.ir, "model_wrapper.ir.yaml");
}

#[test]
fn loads_sil_wrapper_example_config() {
    let config = Config::load("configs/sil-wrapper.example.yaml").unwrap();

    assert_eq!(config.naming.prefix, "sil");
    assert_eq!(
        config.input.dir.as_ref(),
        Some(&std::path::PathBuf::from("/absolute/path/to/src/IE/SIL"))
    );
    assert!(config.output.dir.ends_with("configs/pkg/sil"));
}

#[test]
fn sil_wrapper_example_scopes_per_header_output_names() {
    let config = Config::load("configs/sil-wrapper.example.yaml").unwrap();

    let dir = config.input.dir.as_ref().unwrap();
    let master = config.scoped_to_header(dir.join("IsAAMaster.h"));
    let user = config.scoped_to_header(dir.join("IsAAUser.h"));

    assert_eq!(master.output.header, "is_aa_master_wrapper.h");
    assert_eq!(master.output.source, "is_aa_master_wrapper.cpp");
    assert_eq!(master.output.ir, "is_aa_master_wrapper.ir.yaml");
    assert_eq!(master.go_filename(""), "is_aa_master_wrapper.go");

    assert_eq!(user.output.header, "is_aa_user_wrapper.h");
    assert_eq!(user.output.source, "is_aa_user_wrapper.cpp");
    assert_eq!(user.output.ir, "is_aa_user_wrapper.ir.yaml");
    assert_eq!(user.go_filename(""), "is_aa_user_wrapper.go");
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
