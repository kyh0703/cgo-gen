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
- `filter.*`
- `naming.*`
- `policies.*`

Filter fields support simple names, fully qualified names, and `::*` wildcard prefixes.
Include and exclude lists are available for namespaces, classes, functions, methods, enums, and signature types.

Relative paths are resolved from the config file location.
