use std::{env, fs, path::PathBuf};

use c_go::{config::Config, generator};

fn temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_dir_only_generate_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn dir_only_generation_uses_classified_headers_for_model_and_facade_outputs() {
    let root = temp_dir("classified_outputs");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            ThingModel() {}
            int GetId() const { return 7; }
            void SetId(int value) { (void)value; }
        };
        "#,
    )
    .unwrap();

    fs::write(
        include_dir.join("Api.hpp"),
        r#"
        #include "ThingModel.hpp"

        class Api {
        public:
            Api() {}
            bool GetThingById(int id, ThingModel* out) { return id > 0; }
        };
        "#,
    )
    .unwrap();

    fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  dir: include
output:
  dir: gen
files:
  model:
    - include/ThingModel.hpp
  facade:
    - include/Api.hpp
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    generator::generate_all(&config, true).unwrap();

    let output_dir = root.join("gen");

    assert!(output_dir.join("thing_model_wrapper.h").exists());
    assert!(output_dir.join("api_wrapper.h").exists());
    assert!(output_dir.join("thing_model_wrapper.go").exists());
    assert!(output_dir.join("api_wrapper.go").exists());
}

#[test]
fn nested_output_dir_places_all_generated_files_at_output_root() {
    let root = temp_dir("nested_output");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("Thing.hpp"),
        r#"
        class Thing {
        public:
            Thing() {}
            int GetValue() const { return 7; }
        };
        "#,
    )
    .unwrap();

    fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  dir: include
output:
  dir: ./gen/test
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    generator::generate_all(&config, true).unwrap();

    assert!(root.join("gen/test").is_dir());
    assert!(root.join("gen/test/thing_wrapper.go").exists());
    assert!(root.join("gen/test/thing_wrapper.h").exists());
    assert!(root.join("gen/test/thing_wrapper.cpp").exists());
    assert!(root.join("gen/test/thing_wrapper.ir.yaml").exists());
}
