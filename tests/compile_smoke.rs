use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use c_go::{config::Config, generator, ir, parser};

fn temp_output_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_compile_test_{}_{}",
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

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn generated_wrapper_compiles_and_runs_against_sample_cpp_library() {
    let mut config = Config::load("cppgo-wrap.yaml").unwrap();
    config.output.dir = temp_output_dir("link");
    let parsed = parser::parse(&config).unwrap();
    let ir = ir::normalize(&config, &parsed).unwrap();
    generator::generate(&config, &ir, true).unwrap();

    let smoke_cpp = config.output.dir.join("smoke.cpp");
    fs::write(
        &smoke_cpp,
        r#"
        #include "wrapper.h"
        int main() {
            fooBarHandle* bar = cgowrap_foo_bar_new(7);
            if (bar == nullptr) return 10;
            if (cgowrap_foo_add(1, 2) != 3) return 11;
            if (cgowrap_foo_bar_value(bar) != 7) return 12;
            cgowrap_foo_bar_set_value(bar, 9);
            if (cgowrap_foo_bar_value(bar) != 9) return 13;
            char* name = cgowrap_foo_bar_name(bar);
            if (name == nullptr) return 14;
            cgowrap_string_free(name);
            cgowrap_foo_bar_delete(bar);
            return 0;
        }
        "#,
    )
    .unwrap();

    let binary = config.output.dir.join("smoke");
    let compiler = pick_clangxx();
    let root = project_root();
    let status = Command::new(&compiler)
        .current_dir(&root)
        .arg("-std=c++17")
        .arg(config.output.dir.join(&config.output.source))
        .arg(root.join("examples/simple-cpp/src/foo.cpp"))
        .arg(&smoke_cpp)
        .arg("-I")
        .arg(root.join("examples/simple-cpp/include"))
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap();

    assert!(status.success(), "generated wrapper did not compile/link");

    let status = Command::new(&binary).status().unwrap();
    assert!(status.success(), "generated smoke binary failed: {status}");
}
