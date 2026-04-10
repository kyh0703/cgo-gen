use std::{
    env, fs,
    path::PathBuf,
    process::Command,
};

use cgo_gen::{
    config::Config,
    domain::kind::RecordLayout,
    generator::{self, render_go_structs, render_header},
    ir, parser,
    pipeline::context::PipelineContext,
};

fn temp_output_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_packed_struct_{}_{}",
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

#[test]
fn packed_struct_defined_in_main_header_compiles_via_handle_wrappers() {
    let root = temp_output_dir("main_header");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Packed.hpp"),
        r#"
        #include <stdint.h>

        #pragma pack(push, 1)
        struct PackedRecord {
            int value;
            uint16_t code;
        };
        #pragma pack(pop)
        "#,
    )
    .unwrap();
    fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  headers:
    - include/Packed.hpp
output:
  dir: out
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config.clone())).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    assert_eq!(parsed.classes.len(), 1);
    assert_eq!(parsed.classes[0].layout, RecordLayout::Packed);

    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let header = render_header(&ctx, &ir);
    assert!(header.contains("PackedRecordHandle* cgowrap_PackedRecord_new(void);"));
    assert!(header.contains("int cgowrap_PackedRecord_GetValue(const PackedRecordHandle* self);"));
    assert!(
        header.contains("void cgowrap_PackedRecord_SetCode(PackedRecordHandle* self, uint16_t value);")
            || header.contains(
                "void cgowrap_PackedRecord_SetCode(PackedRecordHandle* self, unsigned short value);"
            )
    );

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        int main() {{
            PackedRecordHandle* item = cgowrap_PackedRecord_new();
            if (item == nullptr) return 10;
            cgowrap_PackedRecord_SetValue(item, 0x12345678);
            if (cgowrap_PackedRecord_GetValue(item) != 0x12345678) return 11;
            cgowrap_PackedRecord_SetCode(item, 7);
            if (cgowrap_PackedRecord_GetCode(item) != 7) return 12;
            cgowrap_PackedRecord_delete(item);
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
fn include_only_packed_struct_tag_uses_opaque_wrapper_policy() {
    let root = temp_output_dir("include_only_tag");
    fs::create_dir_all(root.join("include")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("include/Packed.hpp"),
        r#"
        #include <stdint.h>

        #pragma pack(push, 1)
        struct PackedTag {
            uint16_t code;
            uint32_t value;
        };
        #pragma pack(pop)
        "#,
    )
    .unwrap();
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        #include "Packed.hpp"

        bool UsePacked(struct PackedTag* value);
        "#,
    )
    .unwrap();
    fs::write(
        root.join("src/Api.cpp"),
        r#"
        #include "Api.hpp"

        bool UsePacked(struct PackedTag* value) {
            return value != nullptr && value->code == 7 && value->value == 0x11223344u;
        }
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
    assert!(parsed.classes.is_empty());

    let ir = ir::normalize(&ctx, &parsed).unwrap();
    generator::generate(&ctx, &ir, true, &Default::default()).unwrap();

    let header = render_header(&ctx, &ir);
    assert!(header.contains("typedef struct PackedTagHandle PackedTagHandle;"));
    assert!(header.contains("bool cgowrap_UsePacked(PackedTagHandle* value);"));

    let go = render_go_structs(&ctx, &ir).unwrap();
    let go_text = go
        .iter()
        .map(|file| file.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(go_text.contains("type PackedTag struct {\n    ptr *C.PackedTagHandle\n}"));
    assert!(go_text.contains("func UsePacked(value *PackedTag) bool {"));
    assert!(!go_text.contains("*C.struct_PackedTag"));

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        format!(
            r#"
        #include "{}"
        #include "Packed.hpp"

        int main() {{
            PackedTag native{{7, 0x11223344u}};
            if (!cgowrap_UsePacked(reinterpret_cast<PackedTagHandle*>(&native))) return 10;
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
        .arg(root.join("src/Api.cpp"))
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
