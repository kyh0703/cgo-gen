use std::{env, fs};

use c_go::{config::Config, generator, ir, parser};

fn temp_output_dir(label: &str) -> std::path::PathBuf {
    let mut path = env::temp_dir();
    path.push(format!("c_go_facade_test_{}_{}", label, std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn generates_go_facade_for_simple_free_function_header() {
    let mut config = Config::load("tests/fixtures/simple/config.yaml").unwrap();
    let facade_header = config.input.headers[0].clone();
    config.output.dir = temp_output_dir("generate");
    config.files.facade = vec![facade_header];

    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    generator::generate(&config, &ir, true).unwrap();

    let go_facade =
        fs::read_to_string(config.facade_output_dir().join(config.go_filename(""))).unwrap();

    assert!(go_facade.contains("import \"C\""));
    assert!(go_facade.contains(&format!(
        "#include \"{}\"",
        config.raw_include_for_go(&config.output.header)
    )));
    assert!(go_facade.contains("func Add(lhs int, rhs int) int {"));
    assert!(go_facade.contains("C.cgowrap_foo_add(C.int(lhs), C.int(rhs))"));
}

#[test]
fn generates_go_facade_for_bool_and_string_returns() {
    let root = temp_output_dir("bool-string");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    let header_path = include_dir.join("Api.hpp");
    fs::write(
        &header_path,
        r#"
        #pragma once
        #include <string>

        bool is_ready();
        std::string version();
        const char* banner();
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
files:
  facade:
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
    generator::generate(&config, &ir, true).unwrap();

    let go_facade =
        fs::read_to_string(config.facade_output_dir().join(config.go_filename(""))).unwrap();

    assert!(go_facade.contains("import \"errors\""));
    assert!(go_facade.contains("func IsReady() bool {"));
    assert!(go_facade.contains("return bool(C.cgowrap_is_ready())"));
    assert!(go_facade.contains("func Version() (string, error) {"));
    assert!(go_facade.contains("raw := C.cgowrap_version()"));
    assert!(go_facade.contains("defer C.cgowrap_string_free(raw)"));
    assert!(go_facade.contains("return C.GoString(raw), nil"));
    assert!(go_facade.contains("func Banner() (string, error) {"));
    assert!(go_facade.contains("raw := C.cgowrap_banner()"));
    let banner_section = go_facade
        .split("func Banner() (string, error) {")
        .nth(1)
        .unwrap();
    let banner_body = banner_section.split("}\n").next().unwrap();
    assert!(!banner_body.contains("string_free"));
}

#[test]
fn rejects_namespaced_facade_functions_that_collide_in_go_exports() {
    let root = temp_output_dir("namespace-collision");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    let header_path = include_dir.join("Api.hpp");
    fs::write(
        &header_path,
        r#"
        #pragma once

        namespace alpha { int init(); }
        namespace beta { int init(); }
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
files:
  facade:
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
    let error = generator::generate(&config, &ir, true)
        .unwrap_err()
        .to_string();

    assert!(error.contains("facade export collision"));
}

#[test]
fn lifts_known_model_out_param_methods_into_model_returning_facade_methods() {
    let root = temp_output_dir("model-method");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            ThingModel() = default;
            ~ThingModel() = default;
            int GetValue() const;
            void SetValue(int value);
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
            Api() = default;
            ~Api() = default;
            bool IsReady() const;
            int Clear();
            bool GetThing(int id, ThingModel& out);
            bool GetThingByKey(const char* key, ThingModel* out);
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
  dir: out
naming:
  prefix: cgowrap
  style: preserve
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    generator::generate_all(&config, true).unwrap();

    let go_facade = fs::read_to_string(root.join("out/go/api_wrapper.go")).unwrap();

    assert!(go_facade.contains("type Api struct {"));
    assert!(go_facade.contains("ptr *C.ApiHandle"));
    assert!(go_facade.contains("func NewApi() (*Api, error) {"));
    assert!(go_facade.contains("C.cgowrap_Api_new()"));
    assert!(go_facade.contains("func (a *Api) Close() {"));
    assert!(go_facade.contains("C.cgowrap_Api_delete(a.ptr)"));
    assert!(go_facade.contains("func (a *Api) IsReady() bool {"));
    assert!(go_facade.contains("return bool(C.cgowrap_Api_IsReady(a.ptr))"));
    assert!(go_facade.contains("func (a *Api) Clear() int {"));
    assert!(go_facade.contains("return int(C.cgowrap_Api_Clear(a.ptr))"));
    assert!(go_facade.contains("func (a *Api) GetThing(id int) (ThingModel, error) {"));
    assert!(go_facade.contains("out := C.cgowrap_ThingModel_new()"));
    assert!(go_facade.contains("C.cgowrap_Api_GetThing(a.ptr, C.int(id), out)"));
    assert!(go_facade.contains("return mapThingModelFromHandle(out), nil"));
    assert!(go_facade.contains("func (a *Api) GetThingByKey(key string) (ThingModel, error) {"));
    assert!(go_facade.contains("cArg0 := C.CString(key)"));
    assert!(go_facade.contains("defer C.free(unsafe.Pointer(cArg0))"));
    assert!(go_facade.contains("C.cgowrap_Api_GetThingByKey(a.ptr, cArg0, out)"));
    assert!(
        go_facade.contains("func mapThingModelFromHandle(handle *C.ThingModelHandle) ThingModel {")
    );
    assert!(go_facade.contains("model.Value = int(C.cgowrap_ThingModel_GetValue(handle))"));
}

#[test]
fn keeps_unknown_model_refs_in_raw_wrappers_but_filters_them_from_go_facade() {
    let root = temp_output_dir("unknown-model-raw-first");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            ThingModel() = default;
            ~ThingModel() = default;
            int GetValue() const;
            void SetValue(int value);
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("UnknownThing.hpp"),
        r#"
        class UnknownThing {
        public:
            UnknownThing() = default;
            ~UnknownThing() = default;
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("Api.hpp"),
        r#"
        #include "ThingModel.hpp"
        #include "UnknownThing.hpp"

        class Api {
        public:
            Api() = default;
            ~Api() = default;
            int Count() const;
            bool GetThing(int id, ThingModel& out);
            bool GetUnknown(int id, UnknownThing& out);
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
  dir: out
naming:
  prefix: cgowrap
  style: preserve
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    generator::generate_all(&config, true).unwrap();

    let raw_header = fs::read_to_string(root.join("out/raw/api_wrapper.h")).unwrap();
    let raw_source = fs::read_to_string(root.join("out/raw/api_wrapper.cpp")).unwrap();
    let ir_yaml = fs::read_to_string(root.join("out/raw/api_wrapper.ir.yaml")).unwrap();
    let go_facade = fs::read_to_string(root.join("out/go/api_wrapper.go")).unwrap();

    assert!(raw_header.contains("typedef struct UnknownThingHandle UnknownThingHandle;"));
    assert!(raw_header.contains(
        "bool cgowrap_Api_GetUnknown(ApiHandle* self, int id, UnknownThingHandle* out);"
    ));
    assert!(raw_source.contains("cgowrap_Api_GetUnknown"));
    assert!(raw_source.contains("*reinterpret_cast<UnknownThing*>(out)"));
    assert!(ir_yaml.contains("cpp_name: Api::GetUnknown"));
    assert!(go_facade.contains("func (a *Api) Count() int {"));
    assert!(go_facade.contains("return int(C.cgowrap_Api_Count(a.ptr))"));
    assert!(go_facade.contains("func (a *Api) GetThing(id int) (ThingModel, error) {"));
    assert!(!go_facade.contains("GetUnknown("));
    assert!(!go_facade.contains("UnknownThingHandle"));
}

#[test]
fn skips_raw_unsafe_by_value_internal_types_without_aborting_supported_facade_output() {
    let root = temp_output_dir("unknown-model-by-value-skip");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            ThingModel() = default;
            ~ThingModel() = default;
            int GetValue() const;
            void SetValue(int value);
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("UnknownThing.hpp"),
        r#"
        class UnknownThing {
        public:
            UnknownThing() = default;
            ~UnknownThing() = default;
        };
        "#,
    )
    .unwrap();
    fs::write(
        include_dir.join("Api.hpp"),
        r#"
        #include "ThingModel.hpp"
        #include "UnknownThing.hpp"

        class Api {
        public:
            Api() = default;
            ~Api() = default;
            int Count() const;
            bool GetThing(int id, ThingModel& out);
            bool SaveUnknown(UnknownThing value);
            UnknownThing BuildUnknown() const;
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
  dir: out
naming:
  prefix: cgowrap
  style: preserve
"#,
    )
    .unwrap();

    let prepared = generator::prepare_config(&Config::load(&config_path).unwrap()).unwrap();
    let config = prepared.scoped_to_header(prepared.files.facade[0].clone());
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    generator::generate(&config, &ir, true).unwrap();

    let raw_header = fs::read_to_string(root.join("out/raw/api_wrapper.h")).unwrap();
    let raw_source = fs::read_to_string(root.join("out/raw/api_wrapper.cpp")).unwrap();
    let ir_yaml = fs::read_to_string(root.join("out/raw/api_wrapper.ir.yaml")).unwrap();
    let go_facade = fs::read_to_string(root.join("out/go/api_wrapper.go")).unwrap();

    assert!(
        raw_header
            .contains("bool cgowrap_Api_GetThing(ApiHandle* self, int id, ThingModelHandle* out);")
    );
    assert!(raw_source.contains("cgowrap_Api_GetThing"));
    assert!(!raw_header.contains("SaveUnknown"));
    assert!(!raw_header.contains("BuildUnknown"));
    assert!(!raw_source.contains("SaveUnknown"));
    assert!(!raw_source.contains("BuildUnknown"));
    assert!(ir.support.skipped_declarations.iter().any(|item| {
        item.cpp_name == "Api::SaveUnknown"
            && item.reason.contains("UnknownThing")
            && item.reason.contains("by-value")
    }));
    assert!(ir.support.skipped_declarations.iter().any(|item| {
        item.cpp_name == "Api::BuildUnknown"
            && item.reason.contains("UnknownThing")
            && item.reason.contains("by-value")
    }));
    assert!(ir_yaml.contains("cpp_name: Api::SaveUnknown"));
    assert!(ir_yaml.contains("cpp_name: Api::BuildUnknown"));
    assert!(ir_yaml.contains("raw-unsafe by-value"));
    assert!(go_facade.contains("func (a *Api) Count() int {"));
    assert!(go_facade.contains("func (a *Api) GetThing(id int) (ThingModel, error) {"));
    assert!(!go_facade.contains("SaveUnknown("));
    assert!(!go_facade.contains("BuildUnknown("));
}

#[test]
fn keeps_non_model_methods_on_general_api_path_even_if_names_look_like_lookup_apis() {
    let root = temp_output_dir("general-api-routing");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("Api.hpp"),
        r#"
        class Api {
        public:
            Api() = default;
            ~Api() = default;
            bool ListThing(int id) const;
            int NextThing(int cursor);
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
files:
  facade:
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
    generator::generate_all(&config, true).unwrap();

    let go_facade = fs::read_to_string(root.join("out/go/api_wrapper.go")).unwrap();

    assert!(go_facade.contains("func (a *Api) ListThing(id int) bool {"));
    assert!(go_facade.contains("return bool(C.cgowrap_Api_ListThing(a.ptr, C.int(id)))"));
    assert!(go_facade.contains("func (a *Api) NextThing(cursor int) int {"));
    assert!(!go_facade.contains("(ThingModel, error)"));
    assert!(!go_facade.contains("mapThingModelFromHandle"));
}

#[test]
fn does_not_lift_known_model_when_it_is_not_the_final_supported_out_param() {
    let root = temp_output_dir("model-not-last");
    let include_dir = root.join("include");
    fs::create_dir_all(&include_dir).unwrap();

    fs::write(
        include_dir.join("ThingModel.hpp"),
        r#"
        class ThingModel {
        public:
            ThingModel() = default;
            ~ThingModel() = default;
            int GetValue() const;
            void SetValue(int value);
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
            Api() = default;
            ~Api() = default;
            bool IsReady() const;
            bool GetThing(ThingModel& out, int id);
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
  dir: out
naming:
  prefix: cgowrap
  style: preserve
"#,
    )
    .unwrap();

    let config = Config::load(&config_path).unwrap();
    generator::generate_all(&config, true).unwrap();

    let go_facade = fs::read_to_string(root.join("out/go/api_wrapper.go")).unwrap();

    assert!(go_facade.contains("type Api struct {"));
    assert!(go_facade.contains("func (a *Api) IsReady() bool {"));
    assert!(!go_facade.contains("func (a *Api) GetThing("));
    assert!(!go_facade.contains("(ThingModel, error)"));
    assert!(!go_facade.contains("mapThingModelFromHandle"));
}
