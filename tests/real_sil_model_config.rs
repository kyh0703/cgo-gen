use std::{env, fs, path::PathBuf};

use c_go::{config::Config, generator, ir, parser};

fn temp_output_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_real_sil_model_config_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    path.push("sil");
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn checked_in_real_sil_model_config_uses_dir_only_input_shape() {
    let config = Config::load("configs/sil-real-model.yaml").unwrap();

    assert!(config.input.headers.is_empty());
    assert!(config.input.dir.as_ref().is_some_and(|path| path.is_absolute()));
}

#[test]
fn checked_in_real_sil_model_config_generates_go_wrapper_when_sources_exist() {
    let config = Config::load("configs/sil-real-model.yaml").unwrap();
    let header = config.input.dir.as_ref().unwrap().join("IsAAMaster.h");
    assert!(header.exists());

    let prepared = generator::prepare_config(&config).unwrap();
    let mut scoped = prepared.scoped_to_header(header);
    scoped.output.dir = temp_output_dir("generate");

    let parsed = parser::parse(&scoped).unwrap();
    let ir = ir::normalize(&scoped, &parsed).unwrap();
    generator::generate(&scoped, &ir, true).unwrap();

    let go_path = scoped.go_output_dir().join(scoped.go_filename(""));
    let go_wrapper = fs::read_to_string(&go_path).unwrap();

    assert!(go_path.exists());
    assert!(go_wrapper.contains("package sil"));
    assert!(go_wrapper.contains("type IsAAMaster struct {"));
    assert!(go_wrapper.contains("func NewIsAAMaster() (*IsAAMaster, error) {"));
}
