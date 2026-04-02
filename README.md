# cgo-gen

[한국어](./README.ko.md)

`cgo-gen` is a Rust CLI that parses a conservative subset of C/C++ headers and emits:

- C ABI wrapper headers and sources
- optional normalized IR dumps
- Go `cgo` facade files that live beside the generated native wrapper

It is designed for controlled header surfaces, not for arbitrary modern C++.

## Status

`cgo-gen` is intentionally conservative. The public contract is the current CLI and config behavior described in this README. The repository also contains historical planning and design notes under [`docs/`](./docs/), but those are not a stronger source of truth than the code.

## What It Generates

For each supported entry header, `cgo-gen` can write these files into one output directory:

- `<name>_wrapper.h`
- `<name>_wrapper.cpp`
- `<name>_wrapper.go`
- `<name>_wrapper.ir.yaml` when `--dump-ir` is enabled

The generated `.go`, `.h`, `.cpp`, and `.ir.yaml` files are intentionally co-located so a downstream `cgo` package can build them together.

## Requirements

- Rust toolchain
- `libclang` discoverable at runtime
- a Clang-compatible compile environment for non-trivial headers
- Go toolchain only if you plan to build the generated Go package

This crate is built with `clang-sys` feature `clang_18_0`, so an LLVM/Clang 18 era `libclang` setup is the safest target.

## Install

Run directly from the repository:

```bash
cargo run --bin cgo-gen -- check --config cppgo-wrap.yaml
```

Or install the CLI locally:

```bash
cargo install --path .
cgo-gen check --config cppgo-wrap.yaml
```

## Quick Start

The checked-in root config is a minimal end-to-end example:

```yaml
version: 1

input:
  headers:
    - examples/simple-cpp/include/foo.hpp
  compile_commands: examples/simple-cpp/build/compile_commands.json

output:
  dir: gen

naming:
  prefix: cgowrap
  style: snake_case
```

Common commands:

```bash
cargo run --bin cgo-gen -- check --config cppgo-wrap.yaml
cargo run --bin cgo-gen -- ir --config cppgo-wrap.yaml --format yaml
cargo run --bin cgo-gen -- generate --config cppgo-wrap.yaml --dump-ir
```

Example projects:

- [`examples/simple-go`](./examples/simple-go)
- [`examples/simple-go-struct`](./examples/simple-go-struct)

## CLI

`cgo-gen` currently exposes three subcommands:

- `generate --config <path> [--dump-ir]`
- `ir --config <path> [--output <path>] [--format yaml|json]`
- `check --config <path>`

## Configuration Reference

All supported user-facing knobs are YAML config keys. Relative paths are resolved from the config file directory and existing paths are canonicalized, so symlink paths collapse to their real target as soon as the config is loaded.

| Key | Current behavior |
| --- | --- |
| `version` | Optional schema marker. Parsed, but not used for behavior branching today. |
| `input.dir` | Directory-owned parsing mode. `generate` emits one wrapper set per header directly under this directory. |
| `input.headers` | Explicit entry headers. Use this when you want a narrow, deterministic surface. |
| `input.header_dirs` | Recursively expands header files from directories into `input.headers`. Good for header-only samples. |
| `input.dirs` | Recursively expands both headers and translation units from directories. |
| `input.translation_units` | Explicit parse entries. When present, parsing prefers these over `input.headers`. |
| `input.compile_commands` | Imports compiler flags and source translation unit discovery from `compile_commands.json`. |
| `input.include_dirs` | Prepends `-I...` include flags before `input.clang_args`. |
| `input.clang_args` | Extra libclang arguments. Relative `-I...`, `-I <path>`, and `-isystem` paths are resolved from the config file directory. Exact env tokens in the forms `$VAR`, `$(VAR)`, and `${VAR}` are also expanded from the current OS environment. |
| `input.allow_diagnostics` | If `true`, translation units that produce libclang diagnostics are skipped instead of failing the run. |
| `output.dir` | Output directory. Relative paths resolve from the config file directory. |
| `output.header` / `output.source` / `output.ir` | Optional output filenames. When left at defaults in single-header mode, names are inferred as `<header_stem>_wrapper.*`. |
| `naming.prefix` | Prefix for generated C ABI symbols, including `<prefix>_string_free`. |
| `naming.style` | `preserve` keeps symbol case closer to the source spelling. Any other value currently falls back to lowercasing symbol parts; checked-in configs use `snake_case` for that behavior. |

## Reserved Or Historical Knobs

These keys are worth calling out because they may appear in internal docs or old configs, but they are not full public behavior switches today:

| Key | Current status |
| --- | --- |
| `project_root` | Parsed in the Rust config struct, but not used by the generator. |
| `policies.string_mode` | Parsed, but not used for behavior branching today. |
| `policies.enum_mode` | Parsed, but not used for behavior branching today. |
| `policies.unsupported.templates` | Parsed, but not used for behavior branching today. |
| `policies.unsupported.stl_containers` | Parsed, but not used for behavior branching today. |
| `policies.unsupported.exceptions` | Parsed, but not used for behavior branching today. |
| `files.model` / `files.facade` | Mentioned in historical internal docs and tests, but not consumed by the current public `Config` loader. Do not rely on them as active config keys. |

## Using A Symlinked External Project

If you want to keep an external SDK or private C++ project outside this repository, a symlinked vendor directory works well:

```bash
mkdir -p third_party
ln -s /absolute/path/to/external-sdk third_party/external-sdk
```

Then point your config at the symlink inside this repository:

```yaml
version: 1

input:
  dir: third_party/external-sdk/include
  compile_commands: third_party/external-sdk/build/compile_commands.json
  clang_args:
    - -Ithird_party/external-sdk/include

output:
  dir: gen/external-sdk

naming:
  prefix: ext
  style: preserve
```

What happens in practice:

- relative paths are resolved from the YAML file location, not from the shell working directory
- symlink targets are canonicalized during config loading, so parsing and TU matching operate on real paths
- if `compile_commands.json` contains source files inside `input.dir`, those source TUs are preferred over header entries
- imported headers outside `input.dir` can still help type resolution, but they are not treated as owned public entry headers

When the external project already has a good `compile_commands.json`, prefer that over duplicating many `clang_args`.

## Supported Today

- free functions
- non-template classes
- constructors and destructors
- simple public methods
- deterministic overload disambiguation in generated wrapper names
- primitive scalars and fixed-width aliases such as `int32`, `uint64`, and `size_t`
- `const char*`, `char*`, `std::string`, and `std::string_view`
- primitive pointer and reference write-back in Go
- named callback typedefs used by supported APIs
- `struct timeval*` and `struct timeval&`
- handle-backed Go wrappers emitted beside the native wrapper files

## Not Supported Or Intentionally Limited

- operator declarations such as `operator+` and `operator==`
- raw inline function pointer parameters such as `void (*cb)(int)`
- templates and STL-heavy APIs
- anonymous classes
- exception translation
- advanced inheritance modeling
- raw-unsafe by-value object parameters or returns

Some unsupported declarations are skipped instead of aborting the whole run. When that happens, the reason is recorded in `support.skipped_declarations` inside the normalized IR.

## Practical Notes

- `input.allow_diagnostics: true` is a recovery switch, not a quality switch. It skips failing translation units entirely.
- In multi-header directory mode, leave `output.header`, `output.source`, and `output.ir` at defaults so `cgo-gen` can infer one output set per header.
- If your platform cannot find `libclang`, fix your system loader or LLVM setup first.
- `input.clang_args` supports exact env token expansion for `$VAR`, `$(VAR)`, and `${VAR}` only. It is not a general shell interpolation layer and does not support forms like `${VAR:-default}` or partial-string substitution.

## License

[MIT](./LICENSE)
