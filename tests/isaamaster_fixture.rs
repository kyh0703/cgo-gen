use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use c_go::{config::Config, generator, ir, parser};
use serde_yaml::Value;

fn temp_output_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!("c_go_isaamaster_{}_{}", label, std::process::id()));
    let _ = fs::remove_dir_all(&path);
    path.push("sil");
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

fn fixture_dir() -> PathBuf {
    project_root().join("tests/fixtures/isaamaster")
}

fn normalize_ir_yaml_sources(yaml: &str) -> Value {
    let mut value: Value = serde_yaml::from_str(yaml).unwrap();
    if let Some(headers) = value
        .get_mut("source_headers")
        .and_then(Value::as_sequence_mut)
    {
        for header in headers {
            if let Some(path) = header.as_str() {
                *header = Value::String(
                    Path::new(path)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    value
}

#[test]
fn parses_and_generates_wrapper_for_isaamaster_fixture() {
    let mut config = Config::load("tests/fixtures/isaamaster/config.yaml").unwrap();
    config.output.dir = temp_output_dir("generate");

    let parsed = parser::parse(&config).unwrap();
    assert_eq!(parsed.classes.len(), 1);
    assert_eq!(parsed.functions.len(), 0);
    assert_eq!(parsed.enums.len(), 0);
    assert_eq!(parsed.classes[0].name, "IsAAMaster");
    assert_eq!(parsed.classes[0].methods.len(), 70);

    let ir = ir::normalize(&config, &parsed).unwrap();
    assert_eq!(ir.functions.len(), 72);
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "sil_IsAAMaster_GetAADn")
    );
    assert!(
        ir.functions
            .iter()
            .any(|item| item.name == "sil_IsAAMaster_SetDigit1_Num")
    );

    generator::generate(&config, &ir, true).unwrap();

    let header = fs::read_to_string(config.raw_output_dir().join(&config.output.header)).unwrap();
    let source = fs::read_to_string(config.raw_output_dir().join(&config.output.source)).unwrap();
    let ir_yaml = fs::read_to_string(config.raw_output_dir().join(&config.output.ir)).unwrap();
    let go_struct_path = config
        .model_output_dir()
        .join(config.go_filename("IsAAMaster"));
    let go_structs = fs::read_to_string(go_struct_path).unwrap();
    let expected_dir = fixture_dir().join("expected");
    let expected_header = fs::read_to_string(expected_dir.join("is_aa_master_wrapper.h")).unwrap();
    let expected_source =
        fs::read_to_string(expected_dir.join("is_aa_master_wrapper.cpp")).unwrap();
    let expected_ir_yaml =
        fs::read_to_string(expected_dir.join("is_aa_master_wrapper.ir.yaml")).unwrap();

    assert!(header.contains("typedef struct IsAAMasterHandle IsAAMasterHandle;"));
    assert!(header.contains("IsAAMasterHandle* sil_IsAAMaster_new(void);"));
    assert!(header.contains("const char* sil_IsAAMaster_GetAADn(IsAAMasterHandle* self);"));
    assert!(header.contains(
        "void sil_IsAAMaster_SetDigit1_Num(IsAAMasterHandle* self, const char* sDigitNum);"
    ));
    assert!(source.contains("return reinterpret_cast<IsAAMasterHandle*>(new IsAAMaster());"));
    assert!(source.contains("reinterpret_cast<IsAAMaster*>(self)->SetDigit1_Num(sDigitNum);"));
    assert!(go_structs.contains("type IsAAMaster struct {"));
    assert!(go_structs.contains("ptr *C.IsAAMasterHandle"));
    assert!(go_structs.contains("func NewIsAAMaster() (*IsAAMaster, error)"));
    assert!(
        go_structs
            .contains("func requireIsAAMasterHandle(value *IsAAMaster) *C.IsAAMasterHandle {")
    );
    assert!(go_structs.contains("func (i *IsAAMaster) GetAAMasterID() uint32 {"));
    assert!(go_structs.contains("func (i *IsAAMaster) GetAADn() string {"));
    assert!(go_structs.contains("func (i *IsAAMaster) SetDigit1Num(value string) {"));
    assert_eq!(header, expected_header);
    assert_eq!(source, expected_source);
    assert_eq!(
        normalize_ir_yaml_sources(&ir_yaml),
        normalize_ir_yaml_sources(&expected_ir_yaml)
    );
}

#[test]
fn generated_wrapper_compiles_and_runs_against_isaamaster_fixture() {
    let mut config = Config::load("tests/fixtures/isaamaster/config.yaml").unwrap();
    config.output.dir = temp_output_dir("compile");

    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    generator::generate(&config, &ir, true).unwrap();

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        r#"
        #include "is_aa_master_wrapper.h"
        #include <cstring>

        int main() {
            IsAAMasterHandle* item = sil_IsAAMaster_new();
            if (item == nullptr) return 10;

            sil_IsAAMaster_SetAAMasterId(item, 42);
            if (sil_IsAAMaster_GetAAMasterId(item) != 42) return 11;

            sil_IsAAMaster_SetTenantId(item, 7);
            if (sil_IsAAMaster_GetTenantId(item) != 7) return 12;

            sil_IsAAMaster_SetAADn(item, "1001");
            if (std::strcmp(sil_IsAAMaster_GetAADn(item), "1001") != 0) return 13;

            sil_IsAAMaster_SetDigit1_Act(item, 9);
            if (sil_IsAAMaster_GetDigit1_Act(item) != 9) return 14;

            sil_IsAAMaster_SetDigit1_Num(item, "2002");
            if (std::strcmp(sil_IsAAMaster_GetDigit1_Num(item), "2002") != 0) return 15;

            sil_IsAAMaster_delete(item);
            return 0;
        }
        "#,
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let root = project_root();
    let fixture_dir = fixture_dir();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++11")
        .arg(config.raw_output_dir().join(&config.output.source))
        .arg(fixture_dir.join("src/IsAAMaster.cpp"))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(config.raw_output_dir())
        .arg("-I")
        .arg(fixture_dir.join("include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}

#[test]
fn model_classification_auto_projects_isaamaster_without_go_structs() {
    let mut config = Config::load("tests/fixtures/isaamaster/config.yaml").unwrap();
    let model_header = config.input.headers[0].clone();
    config.output.dir = temp_output_dir("model-auto");
    config.files.model = vec![model_header];

    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    generator::generate(&config, &ir, true).unwrap();

    let go_struct_path = config
        .model_output_dir()
        .join(config.go_filename("IsAAMaster"));
    let go_structs = fs::read_to_string(go_struct_path).unwrap();

    assert!(go_structs.contains("type IsAAMaster struct {"));
    assert!(go_structs.contains("ptr *C.IsAAMasterHandle"));
    assert!(go_structs.contains("func (i *IsAAMaster) GetAAMasterID() uint32 {"));
    assert!(go_structs.contains("func (i *IsAAMaster) GetAADn() string {"));
    assert!(go_structs.contains("func (i *IsAAMaster) GetDigit1Act() uint16 {"));
}
