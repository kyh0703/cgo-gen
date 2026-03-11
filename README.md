# c-go

`c-go` is a Rust CLI that parses a conservative subset of C++ headers and generates a C ABI wrapper layer.

The current practical target is **SIL wrapper generation**:

- input: SIL C++ headers + include paths
- output: `sil_wrapper.h`, `sil_wrapper.cpp`, optional `sil_wrapper.ir.yaml`

The tool is intentionally **config-driven** and should not depend on a specific local project layout.

---

## What it does

Given a YAML config, `c-go` will:

1. parse target C++ headers with `libclang`
2. normalize the parsed API into an internal IR
3. generate C ABI wrapper files

Current output style is:

- opaque-handle based wrapper for C++ classes
- flat C-style exported functions
- cgo-friendly generated C/C++ files

---

## Commands

### Generate wrapper files

```bash
cargo run --bin c-go -- generate --config path/to/wrapper.yaml --dump-ir
```

### Print IR only

```bash
cargo run --bin c-go -- ir --config path/to/wrapper.yaml --format yaml
```

### Check parseability without generating files

```bash
cargo run --bin c-go -- check --config path/to/wrapper.yaml
```

---

## YAML config shape

Example:

```yaml
version: 1

input:
  headers:
    - /absolute/path/to/src/IE/SIL/IsIEApi.h
  clang_args:
    - -std=c++11
    - -x
    - c++
    - -I/absolute/path/to/CORE/inc
    - -I/absolute/path/to/CORE/inc/iCore
    - -I/absolute/path/to/CORE/inc/iJson
    - -I/absolute/path/to/CORE/inc/iSqlLib
    - -I/absolute/path/to/src/IE/inc
    - -I/absolute/path/to/src/IE/SIL
    - -I/absolute/path/to/src/LIB/inc
    - -I/absolute/path/to/src/LIB/inc/iBus
    - -I/absolute/path/to/src/LIB/inc/iUtil

output:
  dir: ./pkg/sil
  header: sil_wrapper.h
  source: sil_wrapper.cpp
  ir: sil_wrapper.ir.yaml

filter:
  classes:
    - IsIEApiSession
    - IsIEApiMonitor
  methods:
    - IsIEApiSession::*
    - IsIEApiMonitor::*

naming:
  prefix: sil
  style: preserve

policies:
  string_mode: c_str
  enum_mode: c_enum
  unsupported:
    templates: error
    stl_containers: skip
    exceptions: error
```

A generic template is included at:

```text
configs/sil-wrapper.example.yaml
```

Edit that file for your environment, then run:

```bash
cargo run --bin c-go -- generate --config configs/sil-wrapper.example.yaml --dump-ir
```

---

## SIL-focused usage

If your goal is simply:

- point the tool at SIL headers
- pass the required `-I...` include paths
- get wrapper files out

then the expected workflow is:

1. copy `configs/sil-wrapper.example.yaml`
2. change the absolute paths
3. run `generate`
4. use the generated files from your cgo project

Typical output:

```text
pkg/sil/sil_wrapper.h
pkg/sil/sil_wrapper.cpp
pkg/sil/sil_wrapper.ir.yaml
```

---

## Current scope

Working well for:

- free functions
- non-template classes
- constructors / destructors
- simple public methods
- simple enums
- common typedef aliases such as `NPCSTR`, `uint32`, `int32`

Not fully handled yet:

- templates
- broad STL container support
- overload-safe naming generation
- project-specific facade generation like PSC's handwritten `Init/Clear/NextWebhook` style API
- full `iSiLib`-style domain flattening

---

## Notes for cgo users

This tool generates the wrapper layer only.

You still provide the actual build flags in your Go/cgo project, such as:

- include paths via `#cgo CXXFLAGS: -I...`
- link flags via `#cgo LDFLAGS: ...`

In other words:

- `c-go` is responsible for generating wrapper files
- your consumer project is responsible for compiling and linking them

---

## Development status

- YAML-driven configuration
- `libclang` parser backend
- SIL example config included
- path hardcoding removed from the official workflow
- environment-variable dependency removed from the official workflow

---

## Test

```bash
cargo test
```
