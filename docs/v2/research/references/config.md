# Config

Configuration is YAML-first.

Top-level sections:
- `version`
- `input.*`
- `output.*`

Current supported input keys:
- `input.dir`
- `input.clang_args`
- `input.ldflags`

`input.dir` is scanned recursively.
Wrapper symbol naming is fixed in source and is no longer configurable.

Relative paths are resolved from the config file location.
Unknown keys are rejected when the config is loaded.

Current output layout:
- generated native wrapper artifacts: `output.dir/`
- generated Go artifacts: `output.dir/`
- generated files are co-located so downstream cgo packages can consume package-local `.go`, `.h`, `.cpp`, and `.ir.yaml` outputs together
