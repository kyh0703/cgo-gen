use std::{env, fs, path::PathBuf};

use cgo_gen::{compiler, config::Config, parser};

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

    assert!(
        parsed
            .headers
            .iter()
            .any(|header| header.ends_with("owned.hpp"))
    );
    assert!(
        !parsed
            .headers
            .iter()
            .any(|header| header.ends_with("foreign.hpp"))
    );
    assert!(parsed.classes.iter().any(|class| class.name == "Owned"));
    assert!(!parsed.classes.iter().any(|class| class.name == "LocalOnly"));
    assert!(!parsed.classes.iter().any(|class| class.name == "Foreign"));
}

#[test]
fn dir_only_config_ignores_header_entries_from_compile_commands_when_sources_exist() {
    let fixture = temp_fixture_dir("header_entries_ignored");
    fs::create_dir_all(fixture.join("include")).unwrap();
    fs::create_dir_all(fixture.join("build")).unwrap();

    fs::write(
        fixture.join("include/IEMemory.h"),
        r#"
        class IEMemory {
        public:
            int GetValue() const { return 7; }
        };
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("include/DBHandler.cpp"),
        r#"
        #include "IEMemory.h"
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("build/compile_commands.json"),
        r#"
[
  {
    "directory": ".",
    "file": "../include/IEMemory.h",
    "arguments": [
      "clang++",
      "-std=c++17",
      "-x",
      "c++",
      "-I../include"
    ]
  },
  {
    "directory": ".",
    "file": "../include/DBHandler.cpp",
    "arguments": [
      "clang++",
      "-std=c++17",
      "-x",
      "c++",
      "-I../include"
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
    let units = compiler::collect_translation_units(&config).unwrap();

    assert_eq!(units.len(), 1);
    assert!(units[0].ends_with("DBHandler.cpp"));
}

#[test]
fn dir_only_config_expands_classified_header_directory_into_all_grouped_headers() {
    let fixture = temp_fixture_dir("classified_dir_expansion");
    fs::create_dir_all(fixture.join("include")).unwrap();

    fs::write(
        fixture.join("include/entry.hpp"),
        r#"
        #include "shared.hpp"

        namespace demo {
        class Entry {
        public:
            int GetValue() const { return 7; }
        };
        }
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("include/shared.hpp"),
        r#"
        namespace demo {
        class Shared {
        public:
            int GetValue() const { return 9; }
        };
        }
        "#,
    )
    .unwrap();

    fs::write(
        fixture.join("config.yaml"),
        r#"
version: 1
input:
  dir: include
output:
  dir: gen
"#,
    )
    .unwrap();

    let config = Config::load(fixture.join("config.yaml")).unwrap();
    let units = compiler::collect_translation_units(&config).unwrap();

    assert_eq!(units.len(), 2);
    assert!(units.iter().any(|path| path.ends_with("entry.hpp")));
    assert!(units.iter().any(|path| path.ends_with("shared.hpp")));
}

#[test]
fn scoped_header_keeps_dir_translation_unit_context() {
    let fixture = temp_fixture_dir("scoped_dir_context");
    fs::create_dir_all(fixture.join("include")).unwrap();
    fs::create_dir_all(fixture.join("build")).unwrap();

    fs::write(
        fixture.join("include/types.hpp"),
        r#"
        typedef unsigned long long SharedId;
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("include/entry.hpp"),
        r#"
        class Entry {
        public:
            SharedId GetId() const { return 7; }
        };
        "#,
    )
    .unwrap();
    fs::write(
        fixture.join("include/tu.cpp"),
        r#"
        #include "types.hpp"
        #include "entry.hpp"
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
      "-I../include"
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
    let scoped = config.scoped_to_header(fixture.join("include/entry.hpp"));
    let units = compiler::collect_translation_units(&scoped).unwrap();
    let parsed = parser::parse(&scoped).unwrap();

    assert_eq!(units.len(), 1);
    assert!(units[0].ends_with("tu.cpp"));
    assert!(parsed.classes.iter().any(|class| class.name == "Entry"));
    assert!(
        !parsed
            .headers
            .iter()
            .any(|header| header.ends_with("types.hpp"))
    );
}
