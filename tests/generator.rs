use c_go::{
    config::Config,
    generator::{render_go_structs, render_header, render_source},
    ir, parser,
};

#[test]
fn renders_header_and_source_from_fixture() {
    let config = Config::load("tests/fixtures/simple/config.yaml").unwrap();
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();

    let header = render_header(&config, &ir);
    assert!(header.contains("typedef struct fooBarHandle fooBarHandle;"));
    assert!(header.contains("fooBarHandle* cgowrap_foo_bar_new(int value);"));
    assert!(header.contains("char* cgowrap_foo_bar_name(const fooBarHandle* self);"));

    let source = render_source(&config, &ir);
    assert!(source.contains("#include \"wrapper.h\""));
    assert!(source.contains("return reinterpret_cast<fooBarHandle*>(new foo::Bar(value));"));
    assert!(source.contains("delete reinterpret_cast<foo::Bar*>(self);"));
}

#[test]
fn skips_go_struct_generation_when_not_configured() {
    let config = Config::load("tests/fixtures/simple/config.yaml").unwrap();
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();

    let go = render_go_structs(&config, &ir).unwrap();
    assert!(go.is_empty());
}
