use std::{env, fs, path::PathBuf};

use c_go::{config::Config, generator};

fn temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_multi_header_{}_{}",
        label,
        std::process::id()
    ));
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
files:
  model:
    - include/AlphaThing.hpp
    - include/BetaThing.hpp
naming:
  prefix: cgowrap
  style: preserve
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    generator::generate_all(&config, true).unwrap();

    let output_dir = root.join("gen");
    let raw_dir = output_dir.join("raw");
    let model_dir = output_dir.join("model");

    let alpha_header = raw_dir.join("alpha_thing_wrapper.h");
    let alpha_source = raw_dir.join("alpha_thing_wrapper.cpp");
    let alpha_ir = raw_dir.join("alpha_thing_wrapper.ir.yaml");
    let alpha_go = model_dir.join("alpha_thing_wrapper.go");

    let beta_header = raw_dir.join("beta_thing_wrapper.h");
    let beta_source = raw_dir.join("beta_thing_wrapper.cpp");
    let beta_ir = raw_dir.join("beta_thing_wrapper.ir.yaml");
    let beta_go = model_dir.join("beta_thing_wrapper.go");

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

#[test]
fn file_classification_limits_go_projection_to_model_headers() {
    let root = temp_dir("classification");
    fs::write(
        root.join("include/ModelThing.hpp"),
        r#"
        class ModelThing {
        public:
            int GetValue() const;
            void SetValue(int value);
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("include/FacadeThing.hpp"),
        r#"
        class FacadeThing {
        public:
            int GetCount() const;
            void SetCount(int count);
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
    - include/ModelThing.hpp
    - include/FacadeThing.hpp
files:
  model:
    - include/ModelThing.hpp
  facade:
    - include/FacadeThing.hpp
output:
  dir: gen
naming:
  prefix: cgowrap
  style: preserve
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    generator::generate_all(&config, true).unwrap();

    let output_dir = root.join("gen");
    let model_go = fs::read_to_string(output_dir.join("model/model_thing_wrapper.go")).unwrap();
    let facade_go_path = output_dir.join("facade/facade_thing_wrapper.go");
    let facade_go = fs::read_to_string(&facade_go_path).unwrap();

    assert!(model_go.contains("type ModelThing struct {"));
    assert!(!model_go.contains("type FacadeThing struct {"));
    assert!(
        facade_go.contains("type FacadeThing struct {")
            && facade_go.contains("ptr *C.FacadeThingHandle")
            && !facade_go.contains("    Count int"),
        "facade-classified headers should not emit Go model projections"
    );
}

#[test]
fn model_classification_emits_go_enum_models_without_go_struct_targets() {
    let root = temp_dir("model-enum");
    fs::write(
        root.join("include/ModelTypes.hpp"),
        r#"
        enum Mode {
            MODE_A = 0,
            MODE_B = 1,
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
    - include/ModelTypes.hpp
files:
  model:
    - include/ModelTypes.hpp
output:
  dir: gen
naming:
  prefix: cgowrap
  style: preserve
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    generator::generate_all(&config, true).unwrap();

    let output_dir = root.join("gen");
    let go_models = fs::read_to_string(output_dir.join("model/model_types_wrapper.go")).unwrap();

    assert!(go_models.contains("type Mode int64"));
    assert!(go_models.contains("MODE_A Mode = 0"));
    assert!(go_models.contains("MODE_B Mode = 1"));
}

#[test]
fn rejects_go_struct_targets_from_unclassified_headers() {
    let root = temp_dir("unclassified-go-structs");
    fs::write(
        root.join("include/Thing.hpp"),
        r#"
        class Thing {
        public:
            int GetValue() const;
            void SetValue(int value);
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
    - include/Thing.hpp
output:
  dir: gen
naming:
  prefix: cgowrap
  style: preserve
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    generator::generate_all(&config, true).unwrap();

    let facade_go_path = root.join("gen/model/thing_wrapper.go");
    let facade_layer_go_path = root.join("gen/facade/thing_wrapper.go");
    assert!(
        !facade_go_path.exists(),
        "unclassified headers should not emit Go model files"
    );
    assert!(
        !facade_layer_go_path.exists(),
        "unclassified headers should not emit Go facade files"
    );
}
