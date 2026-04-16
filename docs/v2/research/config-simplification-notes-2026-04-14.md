# Config Simplification Notes

Date: 2026-04-14

## Goal

Reduce the public config surface to this shape only:

```yaml
version: 1

input:
  dir: include
  clang_args:
    - -Iinclude
    - -Ivendor/foo/include
    - -std=c++17
  ldflags:
    - -Lvendor/foo/lib
    - -lfoo

output:
  dir: gen
```

## Keys To Remove

- `input.headers`
- `input.dirs`
- `input.header_dirs`
- `input.translation_units`
- `input.compile_commands`
- `naming.prefix`
- `naming.style`
- already removed: `input.allow_diagnostics`

## Intended Behavior

- `input.dir` becomes the single recursive input root.
- Parsing should work from files discovered under `input.dir` only.
- `input.clang_args` becomes the only parse-context customization path.
- `input.ldflags` stays for generated Go package metadata.
- Generated C symbol prefix should be hardcoded in source, not configurable.

## Important Current Code Paths

### Config surface
- `src/config.rs`
  - `InputConfig` still contains removed candidates.
  - `NamingConfig` still exists and should be removed.
  - `validate()` still allows `config.input.dir or config.input.headers must be set`.
  - `resolve_relative_paths()` still expands removed keys.
  - `apply_output_defaults()` currently depends on `input.headers.len() == 1`.

### Translation-unit discovery
- `src/parsing/compiler.rs`
  - `collect_clang_args()` still reads `compile_commands.json`.
  - `collect_translation_units()` still branches on `headers` and `compile_commands`.
  - `scan_dir_translation_units()` only scans one directory level, not recursive.

### Parsing / filtered headers
- `src/parsing/parser.rs`
  - `api.headers` currently prefers `ctx.input.headers` when present.

### Generation
- `src/codegen/c_abi.rs`
  - `generation_headers()` still uses `ctx.input.headers` fallback.
  - `scan_generation_headers()` only scans one directory level.
  - header/source helper names still use `ctx.naming.prefix`.

### IR / symbol generation
- `src/codegen/ir_norm.rs`
  - module name and symbol names still use `config.naming.prefix`.
  - symbol formatting still branches on `config.naming.style`.

### Go facade
- `src/codegen/go_facade.rs`
  - string/array free helper names still use `config.naming.prefix`.

## Current Recommendation

Do not keep compatibility for removed keys.

Preferred next implementation:

1. Remove old config fields from `Config`.
2. Remove `NamingConfig`.
3. Hardcode the wrapper prefix in code.
4. Make `input.dir` recursive for:
   - generation header discovery
   - translation unit discovery
5. Remove all `compile_commands.json` usage.
6. Update examples and test fixtures to `dir + clang_args + ldflags` only.

## Known Fallout Areas

These areas will need updates when the refactor starts:

- `README.md`
- `README.ko.md`
- `examples/simple-go/config.yaml`
- `examples/simple-go-struct/config.yaml`
- many inline test YAML snippets under `tests/`
- fixture YAML files under `tests/fixtures/`
- tests that assert custom prefixes like `sdk_` or `gen_`

## Notes From Today

- User explicitly wants old keys removed, not accepted as legacy compatibility.
- User prefers one root directory over enumerating headers.
- User wants `ldflags` kept.
- User wants `naming` removed entirely and hardcoded in source.
- User also pointed out that the current `dir` behavior feels too shallow; recursive behavior is expected.
