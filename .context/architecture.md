# Architecture

Pipeline:

1. Load YAML config.
2. Parse a conservative C++ header subset.
3. Normalize into IR.
4. Emit `wrapper.h`, `wrapper.cpp`, and optional `wrapper.ir.yaml`.

The current codebase uses a heuristic bootstrap parser and is intentionally structured so a future Clang/libclang backend can replace only the parser layer.
