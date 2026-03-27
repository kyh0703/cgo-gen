# CLI

## Commands
- `generate --config <path> [--dump-ir]`
- `ir --config <path> [--output <path>] [--format yaml|json]`
- `check --config <path>`

## Behavior
- `generate` writes wrapper files into the configured output directory.
- `ir` writes or prints the normalized IR.
- `check` validates that parsing and normalization succeed and prints a short summary.
