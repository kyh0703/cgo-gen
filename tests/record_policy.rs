use cgo_gen::{
    config::Config,
    domain::kind::{IrTypeKind, RecordKind, RecordLayout},
    generator, parser,
    generator::{render_go_structs, render_header},
    ir,
    pipeline::context::PipelineContext,
};

fn write_fixture(name: &str, header: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("c_go_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(root.join("include/Api.hpp"), header).unwrap();
    std::fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  headers:
    - include/Api.hpp
output:
  dir: gen
"#,
    )
    .unwrap();
    root
}

#[test]
fn parser_marks_record_kind_and_layout() {
    let root = write_fixture(
        "record_kind_layout",
        r#"
        class Widget {
        public:
            int value;
        };

        struct PlainRecord {
            int a;
            char b;
        };

        #pragma pack(push, 1)
        struct PackedRecord {
            int a;
            char b;
        };
        #pragma pack(pop)
        "#,
    );

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();

    let widget = parsed.classes.iter().find(|record| record.name == "Widget").unwrap();
    assert!(!widget.is_struct);
    assert_eq!(widget.layout, RecordLayout::Normal);

    let plain = parsed
        .classes
        .iter()
        .find(|record| record.name == "PlainRecord")
        .unwrap();
    assert!(plain.is_struct);
    assert_eq!(plain.layout, RecordLayout::Normal);

    let packed = parsed
        .classes
        .iter()
        .find(|record| record.name == "PackedRecord")
        .unwrap();
    assert!(packed.is_struct);
    assert_eq!(packed.layout, RecordLayout::Packed);
}

#[test]
fn pipeline_context_separates_model_and_struct_records() {
    let root = write_fixture(
        "record_policy_context",
        r#"
        class Widget {
        public:
            int value;
        };

        struct PlainRecord {
            int a;
            char b;
        };

        #pragma pack(push, 1)
        struct PackedRecord {
            int a;
            char b;
        };
        #pragma pack(pop)
        "#,
    );

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config);
    let (prepared, _) = generator::prepare_with_parsed(&ctx).unwrap();

    assert!(prepared.is_known_model_type("Widget"));
    assert!(!prepared.is_known_model_type("PlainRecord"));
    assert!(!prepared.is_known_model_type("PackedRecord"));

    let widget = prepared.known_record_type("Widget").unwrap();
    assert_eq!(widget.kind, RecordKind::Class);
    assert_eq!(widget.layout, RecordLayout::Normal);

    let plain = prepared.known_record_type("PlainRecord").unwrap();
    assert_eq!(plain.kind, RecordKind::Struct);
    assert_eq!(plain.layout, RecordLayout::Normal);
    assert!(prepared.is_known_struct_type("PlainRecord"));

    let packed = prepared.known_record_type("PackedRecord").unwrap();
    assert_eq!(packed.kind, RecordKind::Struct);
    assert_eq!(packed.layout, RecordLayout::Packed);
    assert_eq!(
        prepared.known_record_layout("PackedRecord"),
        Some(RecordLayout::Packed)
    );
}

#[test]
fn known_struct_tags_follow_the_same_wrapper_policy_as_aliases() {
    let root = write_fixture(
        "record_tag_alias_policy",
        r#"
        #pragma pack(push, 1)
        struct PackedRecord {
            int a;
            char b;
        };
        #pragma pack(pop)

        bool UseAlias(PackedRecord* value);
        bool UseTag(struct PackedRecord* value);
        bool UseRef(struct PackedRecord& value);
        "#,
    );

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config);
    let (prepared, parsed) = generator::prepare_with_parsed(&ctx).unwrap();
    let ir = ir::normalize(&prepared, &parsed).unwrap();

    let alias = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "UseAlias")
        .unwrap();
    assert_eq!(alias.params[0].ty.kind, IrTypeKind::ModelPointer);
    assert_eq!(alias.params[0].ty.c_type, "PackedRecordHandle*");
    assert_eq!(
        alias.params[0].ty.handle.as_deref(),
        Some("PackedRecordHandle")
    );

    let tag = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "UseTag")
        .unwrap();
    assert_eq!(tag.params[0].ty.kind, IrTypeKind::ModelPointer);
    assert_eq!(tag.params[0].ty.c_type, "PackedRecordHandle*");
    assert_eq!(tag.params[0].ty.handle, alias.params[0].ty.handle);

    let reference = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "UseRef")
        .unwrap();
    assert_eq!(reference.params[0].ty.kind, IrTypeKind::ModelReference);
    assert_eq!(reference.params[0].ty.c_type, "PackedRecordHandle*");
    assert_eq!(reference.params[0].ty.handle, alias.params[0].ty.handle);

    let header = render_header(&prepared, &ir);
    assert!(header.contains("bool cgowrap_UseAlias(PackedRecordHandle* value);"));
    assert!(header.contains("bool cgowrap_UseTag(PackedRecordHandle* value);"));
    assert!(header.contains("bool cgowrap_UseRef(PackedRecordHandle* value);"));

    let go = render_go_structs(&prepared, &ir).unwrap();
    let go_text = go
        .iter()
        .map(|file| file.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(go_text.contains("func UseAlias(value *PackedRecord) bool {"));
    assert!(go_text.contains("func UseTag(value *PackedRecord) bool {"));
    assert!(go_text.contains("func UseRef(value *PackedRecord) bool {"));
}

#[test]
fn forward_declared_tag_structs_use_opaque_wrapper_policy() {
    let root = write_fixture(
        "record_forward_declared_tag",
        r#"
        struct Hidden;

        bool TakeHidden(struct Hidden* value);
        bool TakeHiddenRef(struct Hidden& value);
        struct Hidden* EchoHidden(struct Hidden* value);
        "#,
    );

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config);
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();

    let take_ptr = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "TakeHidden")
        .unwrap();
    assert_eq!(take_ptr.params[0].ty.kind, IrTypeKind::ModelPointer);
    assert_eq!(take_ptr.params[0].ty.c_type, "HiddenHandle*");
    assert_eq!(take_ptr.params[0].ty.handle.as_deref(), Some("HiddenHandle"));

    let take_ref = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "TakeHiddenRef")
        .unwrap();
    assert_eq!(take_ref.params[0].ty.kind, IrTypeKind::ModelReference);
    assert_eq!(take_ref.params[0].ty.c_type, "HiddenHandle*");

    let echo = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "EchoHidden")
        .unwrap();
    assert_eq!(echo.returns.kind, IrTypeKind::ModelPointer);
    assert_eq!(echo.returns.c_type, "HiddenHandle*");
    assert_eq!(echo.returns.handle.as_deref(), Some("HiddenHandle"));

    let header = render_header(&ctx, &ir);
    assert!(header.contains("typedef struct HiddenHandle HiddenHandle;"));
    assert!(header.contains("bool cgowrap_TakeHidden(HiddenHandle* value);"));
    assert!(header.contains("bool cgowrap_TakeHiddenRef(HiddenHandle* value);"));
    assert!(header.contains("HiddenHandle* cgowrap_EchoHidden(HiddenHandle* value);"));

    let go = render_go_structs(&ctx, &ir).unwrap();
    let go_text = go
        .iter()
        .map(|file| file.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(go_text.contains("type Hidden struct {\n    ptr *C.HiddenHandle\n}"));
    assert!(go_text.contains("func TakeHidden(value *Hidden) bool {"));
    assert!(go_text.contains("func TakeHiddenRef(value *Hidden) bool {"));
    assert!(go_text.contains("func EchoHidden(value *Hidden) *Hidden {"));
}
