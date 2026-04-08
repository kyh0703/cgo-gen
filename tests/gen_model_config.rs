use std::{env, fs, path::PathBuf};

use cgo_gen::{config::Config, generator, ir, parser, pipeline::context::PipelineContext};

fn temp_output_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_gen_model_config_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    path.push("gen");
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn gen_model_config_uses_dir_only_input_shape() {
    let config = Config::load("configs/gen-model-config.yaml").unwrap();

    assert!(config.input.headers.is_empty());
    assert!(config.input.dir.is_some());
}

#[test]
fn gen_model_config_generates_go_wrapper_when_sources_exist() {
    let config = Config::load("configs/gen-model-config.yaml").unwrap();
    let header = config
        .input
        .dir
        .as_ref()
        .unwrap()
        .join("DataRecord.h");
    assert!(header.exists(), "fixture header not found: {header:?}");

    let prepared = generator::prepare_config(&PipelineContext::new(config)).unwrap();
    let scoped = prepared
        .scoped_to_header(header)
        .with_output_dir(temp_output_dir("generate"));

    let parsed = parser::parse(&scoped).unwrap();
    let ir = ir::normalize(&scoped, &parsed).unwrap();
    generator::generate(&scoped, &ir, true, &Default::default()).unwrap();

    let go_path = scoped.output_dir().join(scoped.go_filename(""));
    let go_wrapper = fs::read_to_string(&go_path).unwrap();

    assert!(go_path.exists());
    assert!(go_wrapper.contains("package gen"));
    assert!(go_wrapper.contains("type DataRecord struct {"));
    assert!(go_wrapper.contains("func NewDataRecord() (*DataRecord, error) {"));
}
