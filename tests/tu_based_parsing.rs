use std::{env, fs, path::PathBuf};

use c_go::{config::Config, parser};

fn temp_fixture_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "c_go_tu_based_parsing_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn dir_only_config_collects_only_owned_header_declarations() {
    let fixture = temp_fixture_dir("owned_only");
    fs::create_dir_all(fixture.join("include")).unwrap();
    fs::create_dir_all(fixture.join("external")).unwrap();
    fs::create_dir_all(fixture.join("build")).unwrap();

    fs::write(
        fixture.join("include/owned.hpp"),
        r#"
        namespace demo {
        class Owned {
        public:
            int GetValue() const { return 7; }
        };
        }
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("external/foreign.hpp"),
        r#"
        namespace demo {
        class Foreign {
        public:
            int GetValue() const { return 9; }
        };
        }
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("include/tu.cpp"),
        r#"
        #include "owned.hpp"
        #include "../external/foreign.hpp"

        namespace demo {
        class LocalOnly {
        public:
            int GetValue() const { return 11; }
        };
        }
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("build/compile_commands.json"),
        r#"
[
  {
    "directory": ".",
    "file": "../include/tu.cpp",
    "arguments": [
      "clang++",
      "-std=c++17",
      "-x",
      "c++",
      "-I../include",
      "-I../external"
    ]
  }
]
"#,
    )
    .unwrap();

    fs::write(
        fixture.join("config.yaml"),
        r#"
version: 1
input:
  dir: include
  compile_commands: build/compile_commands.json
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(fixture.join("config.yaml")).unwrap();
    let parsed = parser::parse(&config).unwrap();

    assert!(parsed.headers.iter().any(|header| header.ends_with("owned.hpp")));
    assert!(!parsed.headers.iter().any(|header| header.ends_with("foreign.hpp")));
    assert!(parsed.classes.iter().any(|class| class.name == "Owned"));
    assert!(!parsed.classes.iter().any(|class| class.name == "LocalOnly"));
    assert!(!parsed.classes.iter().any(|class| class.name == "Foreign"));
}
