# Config

Configuration is YAML-first.

Top-level sections:
- `version`
- `input.*`
- `output.*`
- `naming.*`

Relative paths are resolved from the config file location.
Unknown keys are rejected when the config is loaded.

Current output layout:
- generated native wrapper artifacts: `output.dir/`
- generated Go artifacts: `output.dir/`
- generated files are co-located so downstream cgo packages can consume package-local `.go`, `.h`, `.cpp`, and `.ir.yaml` outputs together
