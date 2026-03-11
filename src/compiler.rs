use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::config::Config;

#[derive(Debug, Clone, Deserialize)]
struct CompileCommand {
    directory: PathBuf,
    file: PathBuf,
    command: Option<String>,
    arguments: Option<Vec<String>>,
}

pub fn collect_clang_args(config: &Config, header: &Path) -> Result<Vec<String>> {
    let mut args = Vec::new();

    if let Some(path) = config.compile_commands_path() {
        let extra_args = read_compile_db_args(&path, header)?;
        args.extend(extra_args);
    }

    args.extend(config.input.clang_args.iter().cloned());

    if !args.iter().any(|arg| arg == "-x") {
        args.push("-x".to_string());
        args.push("c++".to_string());
    }

    if !args.iter().any(|arg| arg.starts_with("-std=")) {
        args.push("-std=c++17".to_string());
    }

    Ok(args)
}

fn read_compile_db_args(path: &Path, header: &Path) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read compile_commands.json: {}", path.display()))?;
    let commands: Vec<CompileCommand> = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse compile_commands.json: {}", path.display()))?;
    let db_dir = path.parent().unwrap_or_else(|| Path::new("."));

    let selected = commands
        .iter()
        .find(|command| resolve_command_file(db_dir, command) == header)
        .or_else(|| commands.first());

    let Some(command) = selected else {
        return Ok(Vec::new());
    };

    let command_dir = resolve_path_base(db_dir, &command.directory);

    let mut args = if let Some(arguments) = &command.arguments {
        arguments.clone()
    } else if let Some(command_line) = &command.command {
        split_command_line(command_line)
    } else {
        Vec::new()
    };

    args.retain(|arg| {
        !arg.ends_with(".cpp")
            && !arg.ends_with(".cc")
            && !arg.ends_with(".cxx")
            && !arg.ends_with(".hpp")
            && !arg.ends_with(".hh")
            && !arg.ends_with(".h")
            && arg != "-c"
            && !arg.starts_with("-o")
    });

    let mut resolved = Vec::new();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        if matches!(
            arg.as_str(),
            "clang" | "clang++" | "clang-18" | "clang++-18" | "cc"
        ) {
            continue;
        }
        if arg == "-I" || arg == "-isystem" {
            if let Some(value) = iter.next() {
                resolved.push(arg.clone());
                resolved.push(resolve_include(&command_dir, &value));
            }
            continue;
        }
        if let Some(value) = arg.strip_prefix("-I") {
            resolved.push(format!("-I{}", resolve_include(&command_dir, value)));
            continue;
        }
        if arg == "-MF" || arg == "-MT" || arg == "-MQ" {
            iter.next();
            continue;
        }
        if let Some(value) = arg.strip_prefix("-isystem") {
            if value.is_empty() {
                resolved.push(arg);
            } else {
                resolved.push(format!("-isystem{}", resolve_include(&command_dir, value)));
            }
            continue;
        }
        resolved.push(arg);
    }

    Ok(resolved)
}

fn resolve_command_file(db_dir: &Path, command: &CompileCommand) -> PathBuf {
    if command.file.is_absolute() {
        command.file.clone()
    } else {
        resolve_path_base(db_dir, &command.directory).join(&command.file)
    }
}

fn resolve_path_base(db_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    }

    let cwd_relative = path.to_path_buf();
    if cwd_relative.exists() {
        return cwd_relative.canonicalize().unwrap_or(cwd_relative);
    }

    let db_relative = db_dir.join(path);
    if db_relative.exists() {
        return db_relative.canonicalize().unwrap_or(db_relative);
    }

    db_relative
}

fn resolve_include(base: &Path, value: &str) -> String {
    let path = Path::new(value);
    if path.is_absolute() {
        value.to_string()
    } else {
        base.join(path).display().to_string()
    }
}

fn split_command_line(command: &str) -> Vec<String> {
    command
        .split_whitespace()
        .skip(1)
        .map(ToString::to_string)
        .collect()
}

pub fn ensure_header_exists(header: &Path) -> Result<()> {
    if !header.exists() {
        anyhow::bail!("header not found: {}", header.display());
    }
    Ok(())
}
