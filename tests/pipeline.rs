use std::{env, fs};

use cgo_gen::{config::Config, domain::kind::IrTypeKind, generator, ir, parser};

fn temp_output_dir(label: &str) -> std::path::PathBuf {
    let mut path = env::temp_dir();
    path.push(format!("c_go_test_{}_{}", label, std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn parses_fixture_and_builds_ir() {
    let config = Config::load("tests/fixtures/simple/config.yaml").unwrap();
    let parsed = parser::parse(&config).unwrap();
    assert_eq!(parsed.classes.len(), 1);
    assert_eq!(parsed.functions.len(), 1);
    assert_eq!(parsed.enums.len(), 1);

    let ir = ir::normalize(&config, &parsed).unwrap();
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_foo_bar_new")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "cgowrap_foo_add")
    );
    assert!(ir.functions.iter().any(|item| {
        item.name == "cgowrap_foo_bar_name" && item.returns.kind == IrTypeKind::String
    }));
}

#[test]
fn generates_wrapper_files() {
    let mut config = Config::load("tests/fixtures/simple/config.yaml").unwrap();
    config.output.dir = temp_output_dir("generate");
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    generator::generate(&config, &ir, true).unwrap();

    let header = fs::read_to_string(config.output_dir().join(&config.output.header)).unwrap();
    let source = fs::read_to_string(config.output_dir().join(&config.output.source)).unwrap();
    let ir_yaml = fs::read_to_string(config.output_dir().join(&config.output.ir)).unwrap();

    assert!(header.contains("typedef struct fooBarHandle fooBarHandle;"));
    assert!(header.contains("int cgowrap_foo_add(int lhs, int rhs);"));
    assert!(source.contains("new foo::Bar(value)"));
    assert!(
        source.contains("std::string result = reinterpret_cast<const foo::Bar*>(self)->name();")
    );
    assert!(ir_yaml.contains("parser_backend: libclang"));
}
