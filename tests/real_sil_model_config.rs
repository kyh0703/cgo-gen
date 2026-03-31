use std::{
    env, fs,
    path::{Path, PathBuf},
};

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
fn checked_in_real_sil_model_config_generates_model_when_sources_exist() {
    let mut config = Config::load("configs/sil-real-model.yaml").unwrap();

    if !config.input.headers[0].exists() {
        eprintln!(
            "skipping real SIL model config test because source header is missing: {}",
            config.input.headers[0].display()
        );
        return;
    }

    config.output.dir = temp_output_dir("generate");

    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    generator::generate(&config, &ir, true).unwrap();

    let go_path = config.model_output_dir().join(config.go_filename(""));
    let go_model = fs::read_to_string(&go_path).unwrap();

    assert!(go_path.exists());
    assert!(go_model.contains("package sil"));
    assert!(go_model.contains("type IsAAMaster struct {"));
    assert!(go_model.contains("ptr *C.IsAAMasterHandle"));
    assert!(go_model.contains("func NewIsAAMaster() (*IsAAMaster, error)"));
    assert!(go_model.contains("func (i *IsAAMaster) GetAAMasterID() uint32 {"));
    assert!(
        Path::new(
            config.input.clang_args[3]
                .strip_prefix("-I")
                .unwrap_or(&config.input.clang_args[3])
        )
        .is_absolute()
    );
}
