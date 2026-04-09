use cgo_gen::{
    config::Config,
    domain::kind::IrTypeKind,
    generator::{self, render_go_structs, render_header, render_source},
    ir, parser,
    pipeline::context::PipelineContext,
};
use std::{env, fs};

#[test]
fn renders_header_and_source_from_fixture() {
    let config = Config::load("tests/fixtures/simple/config.yaml").unwrap();
    let ctx = PipelineContext::new(config.clone());
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();

    let header = render_header(&ctx, &ir);
    assert!(header.contains("typedef struct fooBarHandle fooBarHandle;"));
    assert!(header.contains("fooBarHandle* cgowrap_foo_bar_new(int value);"));
    assert!(header.contains("char* cgowrap_foo_bar_name(const fooBarHandle* self);"));

    let source = render_source(&ctx, &ir);
    assert!(source.contains(&format!("#include \"{}\"", config.output.header)));
    assert!(source.contains("return reinterpret_cast<fooBarHandle*>(new foo::Bar(value));"));
    assert!(source.contains("delete reinterpret_cast<foo::Bar*>(self);"));
}

#[test]
fn renders_unified_go_wrapper() {
    let config = Config::load("tests/fixtures/simple/config.yaml").unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config)).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();

    let go = render_go_structs(&ctx, &ir).unwrap();
    assert_eq!(go.len(), 1);
    assert!(go[0].contents.contains("type Bar struct {"));
    assert!(go[0].contents.contains("func Add(lhs int, rhs int) int {"));
}

#[test]
fn preserves_const_char_spelling_but_normalizes_c_value_type() {
    let root = std::env::temp_dir().join(format!("c_go_const_char_value_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Api.hpp"),
        r#"
        class Api {
        public:
            const char GetMarker() const { return 'A'; }
        };
        "#,
    )
    .unwrap();
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

    let config = generator::prepare_config(&PipelineContext::new(
        Config::load(root.join("config.yaml")).unwrap(),
    ))
    .unwrap();
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();

    let marker = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "Api::GetMarker")
        .unwrap();
    assert_eq!(marker.returns.cpp_type, "const char");
    assert_eq!(marker.returns.c_type, "char");
}

#[test]
fn renders_typedef_anonymous_enums_with_alias_name() {
    let root = std::env::temp_dir().join(format!("c_go_typedef_enum_alias_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Api.hpp"),
        r#"
        typedef enum {
            FooDisabled = 0,
            FooEnabled = 1,
        } FooState;
        "#,
    )
    .unwrap();
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

    let config = generator::prepare_config(&PipelineContext::new(
        Config::load(root.join("config.yaml")).unwrap(),
    ))
    .unwrap();
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    let header = render_header(&config, &ir);
    let go = render_go_structs(&config, &ir).unwrap();

    assert!(parsed.enums.iter().any(|item| item.name == "FooState"));
    assert!(!header.contains("FooState"));
    assert!(go[0].contents.contains("type FooState int64"));
    assert!(go[0].contents.contains("FooDisabled FooState = 0"));
    assert!(go[0].contents.contains("FooEnabled FooState = 1"));
}

#[test]
fn normalizes_primitive_alias_pointer_and_reference_c_types_in_header() {
    let root =
        std::env::temp_dir().join(format!("c_go_alias_pointer_header_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("include/Api.hpp"),
        r#"
        #include <stdint.h>
        typedef int32_t int32;
        typedef uint32_t uint32;

        bool TakeAliasPtr(int32* value);
        bool TakeAliasRef(uint32& value);
        "#,
    )
    .unwrap();
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

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config.clone());
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let header = render_header(&ctx, &ir);

    let ptr = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "TakeAliasPtr")
        .unwrap();
    assert_eq!(ptr.params[0].ty.cpp_type, "int32*");
    assert_eq!(ptr.params[0].ty.c_type, "int32_t*");

    let r#ref = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "TakeAliasRef")
        .unwrap();
    assert_eq!(r#ref.params[0].ty.cpp_type, "uint32&");
    assert_eq!(r#ref.params[0].ty.c_type, "uint32_t*");

    assert!(header.contains("bool cgowrap_TakeAliasPtr(int32_t* value);"));
    assert!(header.contains("bool cgowrap_TakeAliasRef(uint32_t* value);"));
    assert!(!header.contains("int32* value"));
    assert!(!header.contains("uint32* value"));
}

#[test]
fn generate_with_go_module_writes_build_flags_and_go_mod() {
    let root = env::temp_dir().join(format!("c_go_go_package_metadata_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(root.join("include/Api.hpp"), "int Add(int lhs, int rhs);").unwrap();
    fs::write(
        root.join("config.yaml"),
        r#"
version: 1
input:
  headers:
    - include/Api.hpp
  clang_args:
    - -I${SDK_INCLUDE}
    - -DMODE=1
    - -std=c++20
    - -Wall
    - -Winvalid-offsetof
output:
  dir: out
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("SDK_INCLUDE", root.join("sdk/include"));
    }

    let config = PipelineContext::from_config_path(root.join("config.yaml"))
        .unwrap()
        .with_go_module(Some("example.com/demo/pkg".to_string()));
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    cgo_gen::generator::generate(&config, &ir, false, &Default::default()).unwrap();

    let go_mod = fs::read_to_string(root.join("out/go.mod")).unwrap();
    assert_eq!(go_mod, "module example.com/demo/pkg\n\ngo 1.25\n");

    let build_flags = fs::read_to_string(root.join("out/build_flags.go")).unwrap();
    assert!(build_flags.contains("package out"));
    assert!(build_flags.contains("#cgo CFLAGS: -I${SRCDIR}"));
    assert!(build_flags.contains("#cgo CXXFLAGS: -I${SRCDIR} -I${SDK_INCLUDE} -DMODE=1 -std=c++20"));
    assert!(!build_flags.contains("sdk/include"));
    assert!(!build_flags.contains("-Winvalid-offsetof"));
    assert!(!build_flags.contains("-Wall"));
}

#[test]
fn struct_fields_generate_synthetic_accessors() {
    let root = env::temp_dir().join(format!(
        "c_go_struct_field_accessors_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Counter.hpp"),
        r#"
        #include <stdint.h>

        struct Counter {
            int value;
            uint32_t total_count;
            const int read_only = 7;
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
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config)).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let header = render_header(&ctx, &ir);
    let source = render_source(&ctx, &ir);
    let go = render_go_structs(&ctx, &ir).unwrap();

    assert!(header.contains("int cgowrap_Counter_GetValue(const CounterHandle* self);"));
    assert!(header.contains("void cgowrap_Counter_SetValue(CounterHandle* self, int value);"));
    assert!(
        header.contains("unsigned int cgowrap_Counter_GetTotalCount(const CounterHandle* self);")
    );
    assert!(header
        .contains("void cgowrap_Counter_SetTotalCount(CounterHandle* self, unsigned int value);"));
    assert!(header.contains("int cgowrap_Counter_GetReadOnly(const CounterHandle* self);"));
    assert!(!header.contains("cgowrap_Counter_SetReadOnly"));

    assert!(source.contains("return reinterpret_cast<const Counter*>(self)->value;"));
    assert!(source.contains("reinterpret_cast<Counter*>(self)->value = value;"));
    assert!(source.contains("return reinterpret_cast<const Counter*>(self)->total_count;"));
    assert!(source.contains("reinterpret_cast<Counter*>(self)->total_count = value;"));

    assert_eq!(go.len(), 1);
    assert!(go[0].contents.contains("type Counter struct {"));
    assert!(go[0]
        .contents
        .contains("func (c *Counter) GetValue() int {"));
    assert!(go[0]
        .contents
        .contains("func (c *Counter) SetValue(value int) {"));
    assert!(go[0]
        .contents
        .contains("func (c *Counter) GetTotalCount() uint32 {"));
    assert!(go[0]
        .contents
        .contains("func (c *Counter) SetTotalCount(value uint32) {"));
    assert!(!go[0].contents.contains("SetReadOnly("));
}

#[test]
fn struct_fixed_model_array_fields_render_element_type_in_source() {
    let root = env::temp_dir().join(format!(
        "c_go_fixed_model_array_fields_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
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
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config)).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let header = render_header(&ctx, &ir);
    let source = render_source(&ctx, &ir);

    assert!(header.contains("ItemHandle** cgowrap_Holder_GetItems(const HolderHandle* self);"));
    assert!(
        header.contains("void cgowrap_Holder_SetItems(HolderHandle* self, ItemHandle** value);")
    );

    assert!(source.contains(
        "_r[_i] = reinterpret_cast<ItemHandle*>(new Item(reinterpret_cast<const Holder*>(self)->items[_i]));"
    ));
    assert!(source.contains(
        "reinterpret_cast<Holder*>(self)->items[_i] = *reinterpret_cast<Item*>(value[_i]);"
    ));
    assert!(!source.contains("new Item[3](reinterpret_cast<const Holder*>(self)->items[_i])"));
    assert!(!source.contains("reinterpret_cast<Item[3]*>(value[_i])"));
}

#[test]
fn renders_model_value_return_as_owned_handle_copy() {
    let root = env::temp_dir().join(format!("c_go_model_value_return_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        class MTime {
        public:
            MTime() : value_(7) {}
            int GetValue() const { return value_; }
        private:
            int value_;
        };

        class Api {
        public:
            MTime GetCreateTime() const { return MTime(); }
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
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config.clone());
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let header = render_header(&ctx, &ir);
    let source = render_source(&ctx, &ir);

    let getter = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "Api::GetCreateTime")
        .unwrap();
    assert_eq!(getter.returns.kind, IrTypeKind::ModelValue);
    assert_eq!(getter.returns.c_type, "MTimeHandle*");
    assert!(header.contains("MTimeHandle* cgowrap_Api_GetCreateTime(const ApiHandle* self);"));
    assert!(source.contains(
        "return reinterpret_cast<MTimeHandle*>(new MTime(reinterpret_cast<const Api*>(self)->GetCreateTime()));"
    ));
}

#[test]
fn renders_model_view_pointer_return_as_owned_handle_copy() {
    let root = env::temp_dir().join(format!("c_go_model_view_return_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Api.hpp"),
        r#"
        struct Child {
            int value;
        };

        class Api {
        public:
            Child* GetChildPtr() { return &child_; }
            Child& GetChildRef() { return child_; }
        private:
            Child child_;
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
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = PipelineContext::new(config.clone());
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let source = render_source(&ctx, &ir);

    let ptr_getter = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "Api::GetChildPtr")
        .unwrap();
    let ref_getter = ir
        .functions
        .iter()
        .find(|function| function.cpp_name == "Api::GetChildRef")
        .unwrap();
    assert_eq!(ptr_getter.returns.kind, IrTypeKind::ModelView);
    assert_eq!(ref_getter.returns.kind, IrTypeKind::ModelView);
    assert!(source.contains("auto result = reinterpret_cast<Api*>(self)->GetChildPtr();"));
    assert!(source.contains("return reinterpret_cast<ChildHandle*>(new Child(*result));"));
    assert!(source.contains(
        "return reinterpret_cast<ChildHandle*>(new Child(reinterpret_cast<Api*>(self)->GetChildRef()));"
    ));
}

#[test]
fn renders_model_value_field_accessors_as_snapshot_get_and_explicit_set() {
    let root = env::temp_dir().join(format!("c_go_model_value_field_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("include/Models.hpp"),
        r#"
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
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(root.join("config.yaml")).unwrap();
    let ctx = generator::prepare_config(&PipelineContext::new(config)).unwrap();
    let parsed = parser::parse(&ctx).unwrap();
    let ir = ir::normalize(&ctx, &parsed).unwrap();
    let header = render_header(&ctx, &ir);
    let source = render_source(&ctx, &ir);
    let go = render_go_structs(&ctx, &ir).unwrap();
    let go_text = go
        .iter()
        .map(|file| file.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(header.contains("ChildHandle* cgowrap_Parent_GetChild(const ParentHandle* self);"));
    assert!(
        header.contains("void cgowrap_Parent_SetChild(ParentHandle* self, ChildHandle* value);")
    );
    assert!(source.contains(
        "return reinterpret_cast<ChildHandle*>(new Child(reinterpret_cast<const Parent*>(self)->child));"
    ));
    assert!(source
        .contains("reinterpret_cast<Parent*>(self)->child = *reinterpret_cast<Child*>(value);"));
    assert!(go_text.contains("func (p *Parent) GetChild() *Child {"));
    assert!(go_text.contains("func (p *Parent) SetChild(value *Child) {"));
}
