use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use cgo_gen::{config::Config, generator, ir, parser, pipeline::context::PipelineContext};

fn temp_output_dir(label: &str) -> PathBuf {
    let mut path = env::var_os("CGO_GEN_TEST_TEMP_ROOT")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("CARGO_TARGET_DIR").map(|dir| PathBuf::from(dir).join("compile_smoke"))
        })
        .unwrap_or_else(env::temp_dir);
    path.push(format!(
        "c_go_compile_test_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn pick_clangxx() -> String {
    for candidate in ["clang++-18", "clang++"] {
        if Command::new(candidate).arg("--version").output().is_ok() {
            return candidate.to_string();
        }
    }
    panic!("clang++ compiler not found")
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn write_simple_cpp_config(root: &Path) -> PathBuf {
    let config_path = root.join("config.yaml");
    let project_root = project_root();
    fs::write(
        &config_path,
        format!(
            r#"version: 1
input:
  headers:
    - {}
  compile_commands: {}
output:
  dir: out
naming:
  prefix: cgowrap
  style: snake_case
"#,
            project_root
                .join("examples/simple-cpp/include/foo.hpp")
                .display(),
            project_root
                .join("examples/simple-cpp/build/compile_commands.json")
                .display()
        ),
    )
    .unwrap();
    config_path
}

#[test]
fn generated_wrapper_compiles_and_runs_against_sample_cpp_library() {
    let root = temp_output_dir("link");
    let config = Config::load(write_simple_cpp_config(&root)).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        int main() {{
            fooBarHandle* bar = cgowrap_foo_bar_new(7);
            if (bar == nullptr) return 10;
            if (cgowrap_foo_add(1, 2) != 3) return 11;
            if (cgowrap_foo_bar_value(bar) != 7) return 12;
            cgowrap_foo_bar_set_value(bar, 9);
            if (cgowrap_foo_bar_value(bar) != 9) return 13;
            char* name = cgowrap_foo_bar_name(bar);
            if (name == nullptr) return 14;
            cgowrap_string_free(name);
            cgowrap_foo_bar_delete(bar);
            return 0;
        }}
        "#,
            config.output.header
        ),
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let project_root = project_root();
    let status = Command::new(&compiler)
        .current_dir(&project_root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(project_root.join("examples/simple-cpp/src/foo.cpp"))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(project_root.join("examples/simple-cpp/include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn generated_wrapper_compiles_for_enum_and_alias_overload_header() {
    let root = temp_output_dir("iserialize_alias");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/iSerialize.h"),
        r#"
        #include <stdint.h>
        typedef unsigned int uint32;
        typedef unsigned long long uint64;

        enum eSeriType {
            eSeriTypeNone = 0,
            eSeriTypeValue = 1,
        };

        class iSerialItem {
        public:
            iSerialItem() : value_(0) {}
            inline void GetVal(uint64 &val) { val = value_; }

        private:
            uint64 value_;
        };

        class iSerialize {
        public:
            iSerialize() = default;
            inline bool Add(uint32 nCode, uint64 val) { return nCode != 0 || val != 0; }
            inline bool Get(uint32 nCode, uint64 &val) {
                val = static_cast<uint64>(nCode) + 1;
                return true;
            }
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  headers:
    - include/iSerialize.h
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        int main() {{
            iSerializeHandle* ser = cgowrap_iSerialize_new();
            if (ser == nullptr) return 10;
            if (!cgowrap_iSerialize_Add(ser, 7, 9)) return 11;
            uint64_t value = 0;
            if (!cgowrap_iSerialize_Get(ser, 7, &value)) return 12;
            if (value != 8) return 13;
            iSerialItemHandle* item = cgowrap_iSerialItem_new();
            if (item == nullptr) return 14;
            cgowrap_iSerialItem_GetVal(item, &value);
            cgowrap_iSerialItem_delete(item);
            cgowrap_iSerialize_delete(ser);
            return 0;
        }}
        "#,
            config.output.header
        ),
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(root.join("include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn generated_wrapper_compiles_for_struct_field_accessors() {
    let root = temp_output_dir("struct_fields");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Counter.hpp"),
        r#"
        #include <stdint.h>

        struct Counter {
            int value;
            uint32_t total_count;
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  headers:
    - include/Counter.hpp
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        int main() {{
            CounterHandle* counter = cgowrap_Counter_new();
            if (counter == nullptr) return 10;
            cgowrap_Counter_SetValue(counter, 9);
            if (cgowrap_Counter_GetValue(counter) != 9) return 11;
            cgowrap_Counter_SetTotalCount(counter, 42);
            if (cgowrap_Counter_GetTotalCount(counter) != 42) return 12;
            cgowrap_Counter_delete(counter);
            return 0;
        }}
        "#,
            config.output.header
        ),
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(root.join("include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn generated_wrapper_compiles_for_char_array_field_accessors() {
    let root = temp_output_dir("char_array_fields");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Agent.hpp"),
        r#"
        struct Agent {
            char login_id[33];
            char pbx_login_id[11];
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  headers:
    - include/Agent.hpp
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let header = fs::read_to_string(config.output_dir().join(&config.output.header)).unwrap();
    let go_wrapper = fs::read_to_string(config.output_dir().join(config.go_filename(""))).unwrap();

    assert!(!header.contains("char[33]Handle"));
    assert!(!header.contains("char[11]Handle"));
    assert!(header.contains("const char* cgowrap_Agent_GetLoginId(const AgentHandle* self);"));
    assert!(go_wrapper.contains("func (a *Agent) GetLoginId() (string, error) {"));
    assert!(!go_wrapper.contains("func (a *Agent) SetLoginId("));
    assert!(!go_wrapper.contains("func (a *Agent) SetPbxLoginId("));

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        #include <cstring>
        int main() {{
            AgentHandle* agent = cgowrap_Agent_new();
            if (agent == nullptr) return 10;
            if (std::strcmp(cgowrap_Agent_GetLoginId(agent), "") != 0) return 11;
            if (std::strcmp(cgowrap_Agent_GetPbxLoginId(agent), "") != 0) return 12;
            cgowrap_Agent_delete(agent);
            return 0;
        }}
        "#,
            config.output.header
        ),
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(root.join("include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn generated_wrapper_compiles_for_fixed_model_array_field_accessors() {
    let root = temp_output_dir("fixed_model_array_fields");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Holder.hpp"),
        r#"
        struct Item {
            int value;
        };

        struct Holder {
            Item items[3];
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  headers:
    - include/Holder.hpp
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        #include <cstdlib>
        int main() {{
            HolderHandle* holder = cgowrap_Holder_new();
            if (holder == nullptr) return 10;

            ItemHandle** items = cgowrap_Holder_GetItems(holder);
            if (items == nullptr) return 11;
            cgowrap_Item_SetValue(items[0], 10);
            cgowrap_Item_SetValue(items[1], 20);
            cgowrap_Item_SetValue(items[2], 30);
            std::free(items);

            ItemHandle** roundtrip = cgowrap_Holder_GetItems(holder);
            if (roundtrip == nullptr) return 12;
            if (cgowrap_Item_GetValue(roundtrip[0]) != 10) return 13;
            if (cgowrap_Item_GetValue(roundtrip[1]) != 20) return 14;
            if (cgowrap_Item_GetValue(roundtrip[2]) != 30) return 15;

            std::free(roundtrip);
            cgowrap_Holder_delete(holder);
            return 0;
        }}
        "#,
            config.output.header
        ),
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(root.join("include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn generated_wrapper_compiles_for_model_value_field_alias_semantics() {
    let root = temp_output_dir("model_view_snapshot");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Models.hpp"),
        r#"
        #include <stdint.h>

        struct Child {
            int value;
        };

        struct Parent {
            Child child;
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  headers:
    - include/Models.hpp
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        int main() {{
            ParentHandle* parent = cgowrap_Parent_new();
            if (parent == nullptr) return 10;
            ChildHandle* initial = cgowrap_Parent_GetChild(parent);
            if (initial == nullptr) return 11;
            cgowrap_Child_SetValue(initial, 3);

            ChildHandle* roundtrip = cgowrap_Parent_GetChild(parent);
            if (roundtrip == nullptr) return 12;
            if (cgowrap_Child_GetValue(roundtrip) != 3) return 13;
            cgowrap_Child_SetValue(roundtrip, 9);

            ChildHandle* updated = cgowrap_Parent_GetChild(parent);
            if (updated == nullptr) return 14;
            if (cgowrap_Child_GetValue(updated) != 9) return 15;

            cgowrap_Parent_delete(parent);
            return 0;
        }}
        "#,
            config.output.header
        ),
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(root.join("include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn generated_wrapper_compiles_for_abstract_model_pointer_returns() {
    let root = temp_output_dir("abstract_model_pointer_return");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Factory.hpp"),
        r#"
        class DBHandler {
        public:
            virtual ~DBHandler() = default;
            int GetValue() const { return 7; }
            virtual void ProcDml() = 0;
        };

        class ConcreteHandler : public DBHandler {
        public:
            void ProcDml() override {}
        };

        class DBHandlerFactory {
        public:
            DBHandler* CreateHandler() { return new ConcreteHandler(); }
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  headers:
    - include/Factory.hpp
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let source = fs::read_to_string(config.output_dir().join(&config.output.source)).unwrap();
    assert!(source.contains(
        "return reinterpret_cast<DBHandlerHandle*>(reinterpret_cast<DBHandlerFactory*>(self)->CreateHandler());"
    ));
    assert!(!source.contains("new DBHandler(*result)"));

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        int main() {{
            DBHandlerFactoryHandle* factory = cgowrap_DBHandlerFactory_new();
            if (factory == nullptr) return 10;
            DBHandlerHandle* handler = cgowrap_DBHandlerFactory_CreateHandler(factory);
            if (handler == nullptr) return 11;
            if (cgowrap_DBHandler_GetValue(handler) != 7) return 12;
            cgowrap_DBHandler_delete(handler);
            cgowrap_DBHandlerFactory_delete(factory);
            return 0;
        }}
        "#,
            config.output.header
        ),
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(root.join("include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn generated_wrapper_compiles_for_model_view_method_alias_semantics() {
    let root = temp_output_dir("model_view_method_alias");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        class ThingModel {
        public:
            ThingModel() = default;
            int GetValue() const { return value_; }
            void SetValue(int value) { value_ = value; }
        private:
            int value_ = 7;
        };

        class Api {
        public:
            ThingModel* GetThingPtr() { return &thing_; }
            ThingModel& GetThingRef() { return thing_; }
        private:
            ThingModel thing_;
        };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  headers:
    - include/Api.hpp
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let source = fs::read_to_string(config.output_dir().join(&config.output.source)).unwrap();
    assert!(source.contains(
        "return reinterpret_cast<ThingModelHandle*>(result);"
    ));
    assert!(source.contains(
        "return reinterpret_cast<ThingModelHandle*>(&result);"
    ));
    assert!(!source.contains("new ThingModel(*result)"));

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        int main() {{
            ApiHandle* api = cgowrap_Api_new();
            if (api == nullptr) return 10;
            ThingModelHandle* ptr = cgowrap_Api_GetThingPtr(api);
            if (ptr == nullptr) return 11;
            cgowrap_ThingModel_SetValue(ptr, 10);
            ThingModelHandle* ref = cgowrap_Api_GetThingRef(api);
            if (ref == nullptr) return 12;
            if (cgowrap_ThingModel_GetValue(ref) != 10) return 13;
            cgowrap_Api_delete(api);
            return 0;
        }}
        "#,
            config.output.header
        ),
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(root.join("include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}
