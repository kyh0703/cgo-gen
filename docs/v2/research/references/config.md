# Config

Configuration is YAML-first.

Top-level sections:
- `version`
- `input.headers`
- `input.compile_commands`
- `input.clang_args`
- `output.dir`
- `output.header`
- `output.source`
- `output.ir`
- `files.model`
- `files.facade`
- `naming.*`
- `policies.*`

Relative paths are resolved from the config file location.

Current output layout:
- generated native wrapper artifacts: `output.dir/`
- generated Go artifacts: `output.dir/`
- generated files are co-located so downstream cgo packages can consume package-local `.go`, `.h`, `.cpp`, and `.ir.yaml` outputs together
