use std::{env, fs, path::PathBuf};

use c_go::{config::Config, generator};

fn temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_model_out_params_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("include")).unwrap();
    path
}

#[test]
fn recognizes_known_model_out_params_in_facade_wrappers() {
    let root = temp_dir("generate");
    fs::write(
        root.join("include/ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            int GetValue() const;
            void SetValue(int value);
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        #include "ThingModel.hpp"

        class Api {
        public:
            bool GetThing(int id, ThingModel& out);
            bool GetThingPtr(int id, ThingModel* out);
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
    - include/ThingModel.hpp
    - include/Api.hpp
files:
  model:
    - include/ThingModel.hpp
  facade:
    - include/Api.hpp
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

    let raw_output_dir = root.join("gen/raw");
    let api_header = fs::read_to_string(raw_output_dir.join("api_wrapper.h")).unwrap();
    let api_source = fs::read_to_string(raw_output_dir.join("api_wrapper.cpp")).unwrap();

    assert!(api_header.contains("typedef struct ThingModelHandle ThingModelHandle;"));
    assert!(
        api_header
            .contains("bool cgowrap_Api_GetThing(ApiHandle* self, int id, ThingModelHandle* out);")
    );
    assert!(
        api_header.contains(
            "bool cgowrap_Api_GetThingPtr(ApiHandle* self, int id, ThingModelHandle* out);"
        )
    );
    assert!(api_source.contains("*reinterpret_cast<ThingModel*>(out)"));
    assert!(api_source.contains("reinterpret_cast<ThingModel*>(out)"));
}

#[test]
fn skips_mismatched_getter_setter_fields_without_failing_generation() {
    let root = temp_dir("mismatch_skip");
    fs::write(
        root.join("include/ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            int GetValue() const;
            void SetValue(int value);
            unsigned short GetNextHop() const;
            void SetNextHop(short value);
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        #include "ThingModel.hpp"

        class Api {
        public:
            bool GetThing(int id, ThingModel& out);
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
    - include/ThingModel.hpp
    - include/Api.hpp
files:
  model:
    - include/ThingModel.hpp
  facade:
    - include/Api.hpp
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

    let api_go = fs::read_to_string(root.join("gen/go/api_wrapper.go")).unwrap();

    assert!(root.join("gen/go/api_wrapper.go").exists());
    assert!(api_go.contains("package"));
}
