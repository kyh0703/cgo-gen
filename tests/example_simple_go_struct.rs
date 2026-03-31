use std::{
    env, fs,
    path::{Path, PathBuf},
};

use c_go::{config::Config, generator};

fn temp_output_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_example_simple_go_struct_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn checked_in_simple_go_struct_example_uses_handle_backed_model_and_reference_cursor() {
    let example_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/simple-go-struct");
    let mut config = Config::load(example_dir.join("config.yaml")).unwrap();
    config.output.dir = temp_output_dir("generate");

    generator::generate_all(&config, true).unwrap();

    let go_model = fs::read_to_string(config.output.dir.join("thing_model_wrapper.go")).unwrap();
    let go_facade = fs::read_to_string(config.output.dir.join("thing_api_wrapper.go")).unwrap();
    let main_go = fs::read_to_string(example_dir.join("cmd/simple-go-struct/main.go")).unwrap();

    assert!(go_model.contains("type ThingModel struct {"));
    assert!(go_model.contains("ptr *C.ThingModelHandle"));
    assert!(go_model.contains("func NewThingModel() (*ThingModel, error) {"));
    assert!(go_model.contains("func (t *ThingModel) SetName(value string) {"));
    assert!(go_model.contains("func (t *ThingModel) SetValue(value int32) {"));

    assert!(go_facade.contains("func (t *ThingApi) SelectThing(id int32, out *ThingModel) bool {"));
    assert!(go_facade.contains("func (t *ThingApi) NextThing(pos *int32, out *ThingModel) bool {"));
    assert!(go_facade.contains("cArg0 := C.int32_t(*pos)"));
    assert!(go_facade.contains("*pos = int32(cArg0)"));

    assert!(main_go.contains("item.SetName(\"seed-from-go\")"));
    assert!(main_go.contains("item.SetValue(99)"));
    assert!(main_go.contains("api.SelectThing(1, item)"));
    assert!(main_go.contains("api.NextThing(&pos, item)"));
}
