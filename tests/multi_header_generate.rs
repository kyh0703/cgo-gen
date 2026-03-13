use std::{env, fs, path::PathBuf};

use c_go::{config::Config, generator};

fn temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!("c_go_multi_header_{}_{}", label, std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("include")).unwrap();
    path
}

#[test]
fn generates_one_wrapper_set_per_header_from_single_config() {
    let root = temp_dir("generate");
    fs::write(
        root.join("include/AlphaThing.hpp"),
        r#"
        class AlphaThing {
        public:
            int GetValue() const;
            void SetValue(int value);
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("include/BetaThing.hpp"),
        r#"
        class BetaThing {
        public:
            const char* GetName() const;
            void SetName(const char* name);
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
    - include/AlphaThing.hpp
    - include/BetaThing.hpp
output:
  dir: gen
filter:
  classes: [AlphaThing, BetaThing]
  methods: [AlphaThing::*, BetaThing::*]
go_structs: [AlphaThing, BetaThing]
naming:
  prefix: cgowrap
  style: preserve
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    generator::generate_all(&config, true).unwrap();

    let output_dir = root.join("gen");

    let alpha_header = output_dir.join("alpha_thing_wrapper.h");
    let alpha_source = output_dir.join("alpha_thing_wrapper.cpp");
    let alpha_ir = output_dir.join("alpha_thing_wrapper.ir.yaml");
    let alpha_go = output_dir.join("alpha_thing_wrapper.go");

    let beta_header = output_dir.join("beta_thing_wrapper.h");
    let beta_source = output_dir.join("beta_thing_wrapper.cpp");
    let beta_ir = output_dir.join("beta_thing_wrapper.ir.yaml");
    let beta_go = output_dir.join("beta_thing_wrapper.go");

    for path in [
        &alpha_header,
        &alpha_source,
        &alpha_ir,
        &alpha_go,
        &beta_header,
        &beta_source,
        &beta_ir,
        &beta_go,
    ] {
        assert!(path.exists(), "missing generated file: {}", path.display());
    }

    let alpha_header_text = fs::read_to_string(alpha_header).unwrap();
    let alpha_go_text = fs::read_to_string(alpha_go).unwrap();
    let beta_header_text = fs::read_to_string(beta_header).unwrap();
    let beta_go_text = fs::read_to_string(beta_go).unwrap();

    assert!(alpha_header_text.contains("AlphaThingHandle"));
    assert!(!alpha_header_text.contains("BetaThingHandle"));
    assert!(alpha_go_text.contains("type AlphaThing struct {"));
    assert!(!alpha_go_text.contains("type BetaThing struct {"));

    assert!(beta_header_text.contains("BetaThingHandle"));
    assert!(!beta_header_text.contains("AlphaThingHandle"));
    assert!(beta_go_text.contains("type BetaThing struct {"));
    assert!(!beta_go_text.contains("type AlphaThing struct {"));
}
