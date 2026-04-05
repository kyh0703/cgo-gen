use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use cgo_gen::{config::Config, generator, ir, parser};

fn temp_output_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
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

#[test]
fn generated_wrapper_compiles_and_runs_against_sample_cpp_library() {
    let mut config = Config::load("cppgo-wrap.yaml").unwrap();
    config.output.dir = temp_output_dir("link");
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    generator::generate(&config, &ir, true).unwrap();

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
    let root = project_root();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(config.output_dir().join(&config.output.source))
        .arg(root.join("examples/simple-cpp/src/foo.cpp"))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.output_dir())
        .arg("-I")
        .arg(root.join("examples/simple-cpp/include"))
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
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    generator::generate(&config, &ir, true).unwrap();

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
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    generator::generate(&config, &ir, true).unwrap();

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
fn generated_wrapper_compiles_for_model_view_snapshot_copy_semantics() {
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
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    generator::generate(&config, &ir, true).unwrap();

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
            cgowrap_Parent_SetChild(parent, initial);
            cgowrap_Child_delete(initial);

            ChildHandle* snapshot = cgowrap_Parent_GetChild(parent);
            if (snapshot == nullptr) return 12;
            cgowrap_Child_SetValue(snapshot, 9);
            ChildHandle* unchanged = cgowrap_Parent_GetChild(parent);
            if (unchanged == nullptr) return 13;
            if (cgowrap_Child_GetValue(unchanged) != 3) return 14;
            cgowrap_Child_delete(unchanged);
            cgowrap_Parent_SetChild(parent, snapshot);
            ChildHandle* updated = cgowrap_Parent_GetChild(parent);
            if (updated == nullptr) return 15;
            if (cgowrap_Child_GetValue(updated) != 9) return 16;
            cgowrap_Child_delete(updated);
            cgowrap_Child_delete(snapshot);

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
