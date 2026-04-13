use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
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

pub fn collect_clang_args(config: &Config, parse_entry: &Path) -> Result<Vec<String>> {
    let mut args = Vec::new();

    if let Some(path) = config.compile_commands_path() {
        let extra_args = read_compile_db_args(&path, parse_entry)?;
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

    add_parse_entry_parent_include(&mut args, parse_entry);
    add_platform_fallback_sysroot(&mut args);
    add_platform_fallback_includes(&mut args);

    Ok(args)
}

fn add_parse_entry_parent_include(args: &mut Vec<String>, parse_entry: &Path) {
    add_header_parent_include(args, parse_entry);
}

pub fn collect_translation_units(config: &Config) -> Result<Vec<PathBuf>> {
    if config.input.dir.is_none() && !config.input.headers.is_empty() {
        return Ok(config.input.headers.clone());
    }

    let Some(dir) = &config.input.dir else {
        return Ok(Vec::new());
    };

    let mut units = if let Some(path) = config.compile_commands_path() {
        read_compile_db_translation_units(&path, dir)?
    } else {
        Vec::new()
    };

    units.extend(collect_classified_translation_units(config, dir)?);
    units = units
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    if units.is_empty() {
        units = scan_dir_translation_units(dir)?;
    }

    Ok(units)
}

fn collect_classified_translation_units(config: &Config, dir: &Path) -> Result<Vec<PathBuf>> {
    let grouped_dirs = config
        .input
        .headers
        .iter()
        .filter(|path| path_is_within(path, dir))
        .filter_map(|path| path.parent().map(Path::to_path_buf))
        .collect::<BTreeSet<_>>();

    let mut source_units = BTreeSet::new();
    let mut header_units = BTreeSet::new();

    for grouped_dir in grouped_dirs {
        for unit in scan_dir_translation_units(&grouped_dir)? {
            if is_source_translation_unit_file(&unit) {
                source_units.insert(unit);
            } else if is_header_file(&unit) {
                header_units.insert(unit);
            }
        }
    }

    if !source_units.is_empty() {
        Ok(source_units.into_iter().collect())
    } else {
        Ok(header_units.into_iter().collect())
    }
}

fn add_header_parent_include(args: &mut Vec<String>, header: &Path) {
    let Some(parent) = header.parent() else {
        return;
    };
    let include = normalize_clang_path(parent);
    let mut has_parent_include = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "-I" || arg == "-isystem" {
            if iter.next().is_some_and(|value| value == &include) {
                has_parent_include = true;
                break;
            }
            continue;
        }
        if arg == &format!("-I{include}") || arg == &format!("-isystem{include}") {
            has_parent_include = true;
            break;
        }
    }
    if !has_parent_include {
        args.push(format!("-I{include}"));
    }
}

fn add_platform_fallback_includes(args: &mut Vec<String>) {
    for include in discover_platform_fallback_include_dirs() {
        let include = normalize_clang_path(&include);
        let already_present = args
            .iter()
            .any(|arg| arg == &format!("-I{include}") || arg == &format!("-isystem{include}"));
        if !already_present {
            args.push(format!("-isystem{include}"));
        }
    }
}

fn add_platform_fallback_sysroot(args: &mut Vec<String>) {
    if env::consts::OS != "macos" || args.iter().any(|arg| arg == "-isysroot") {
        return;
    }

    let Some(sysroot) = discover_macos_sdk_path() else {
        return;
    };
    let sysroot = normalize_clang_path(&sysroot);
    args.push("-isysroot".to_string());
    args.push(sysroot);
}

fn discover_platform_fallback_include_dirs() -> Vec<PathBuf> {
    match env::consts::OS {
        "windows" => discover_windows_fallback_include_dirs(),
        "macos" => discover_macos_fallback_include_dirs(),
        "linux" => discover_linux_fallback_include_dirs(),
        _ => Vec::new(),
    }
}

fn discover_macos_fallback_include_dirs() -> Vec<PathBuf> {
    let mut includes = Vec::new();

    if let Some(resource_dir) = discover_command_output_dir(&["clang++", "-print-resource-dir"]) {
        includes.push(resource_dir.join("include"));
    }

    if let Some(developer_dir) = discover_command_output_dir(&["xcode-select", "-p"]) {
        includes.extend(macos_developer_include_candidates(&developer_dir));
    }

    if let Some(sdk_path) = discover_macos_sdk_path() {
        includes.extend(macos_sdk_include_candidates(&sdk_path));
    }

    if let Some(toolchain_bin) = discover_command_output_dir(&["xcrun", "--find", "clang++"]) {
        includes.extend(macos_toolchain_bin_include_candidates(&toolchain_bin));
    }

    includes
        .into_iter()
        .filter(|path| path.exists())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn macos_developer_include_candidates(developer_dir: &Path) -> Vec<PathBuf> {
    vec![
        developer_dir.join("Toolchains/XcodeDefault.xctoolchain/usr/include/c++/v1"),
        developer_dir.join("Toolchains/XcodeDefault.xctoolchain/usr/include"),
    ]
}

fn macos_sdk_include_candidates(sdk_path: &Path) -> Vec<PathBuf> {
    vec![
        sdk_path.join("usr/include/c++/v1"),
        sdk_path.join("usr/include"),
    ]
}

fn macos_toolchain_bin_include_candidates(clangxx_path: &Path) -> Vec<PathBuf> {
    let Some(toolchain_dir) = clangxx_path
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
    else {
        return Vec::new();
    };

    vec![
        toolchain_dir.join("usr/include/c++/v1"),
        toolchain_dir.join("usr/include"),
    ]
}

fn discover_macos_sdk_path() -> Option<PathBuf> {
    env::var_os("SDKROOT")
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .or_else(|| discover_command_output_dir(&["xcrun", "--show-sdk-path"]))
}

fn discover_windows_fallback_include_dirs() -> Vec<PathBuf> {
    let roots = [
        PathBuf::from("C:/msys64/ucrt64/lib/clang"),
        PathBuf::from("C:/Program Files/LLVM/lib/clang"),
    ];

    roots
        .into_iter()
        .filter_map(|root| latest_versioned_include_dir(&root))
        .filter(|path| path.exists())
        .collect()
}

fn discover_linux_fallback_include_dirs() -> Vec<PathBuf> {
    let mut includes = Vec::new();

    includes.extend(discover_linux_driver_include_dirs());

    if let Some(resource_dir) = discover_command_output_dir(&["clang", "-print-resource-dir"])
        .map(|dir| dir.join("include"))
    {
        includes.push(resource_dir);
    }

    if let Some(resource_dir) = discover_command_output_dir(&["clang++", "-print-resource-dir"])
        .map(|dir| dir.join("include"))
    {
        includes.push(resource_dir);
    }

    if let Some(gcc_include) = discover_command_output_dir(&["c++", "-print-file-name=include"]) {
        includes.push(gcc_include);
    }

    if let Some(gcc_include) = discover_command_output_dir(&["g++", "-print-file-name=include"]) {
        includes.push(gcc_include);
    }

    if let Some(sysroot) = discover_command_output_dir(&["c++", "-print-sysroot"]) {
        includes.extend(linux_sysroot_include_candidates(&sysroot));
    }

    if let Some(sysroot) = discover_command_output_dir(&["g++", "-print-sysroot"]) {
        includes.extend(linux_sysroot_include_candidates(&sysroot));
    }

    includes.extend([
        PathBuf::from("/usr/include"),
        PathBuf::from("/usr/local/include"),
        PathBuf::from("/usr/include/x86_64-linux-gnu"),
    ]);

    includes
        .into_iter()
        .filter(|path| path.exists())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn discover_linux_driver_include_dirs() -> Vec<PathBuf> {
    let candidates = [
        ["clang++", "-E", "-x", "c++", "-", "-v"],
        ["clang++-18", "-E", "-x", "c++", "-", "-v"],
        ["c++", "-E", "-x", "c++", "-", "-v"],
        ["g++", "-E", "-x", "c++", "-", "-v"],
    ];

    for command in candidates {
        let (program, args) = command.split_first().expect("driver candidate");
        let Ok(output) = Command::new(program).args(args).output() else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let parsed = parse_driver_include_search_list(&String::from_utf8_lossy(&output.stderr));
        if !parsed.is_empty() {
            return parsed;
        }
    }

    Vec::new()
}

fn parse_driver_include_search_list(stderr: &str) -> Vec<PathBuf> {
    let mut includes = Vec::new();
    let mut in_search_list = false;

    for line in stderr.lines() {
        let trimmed = line.trim();
        if trimmed == "#include <...> search starts here:" {
            in_search_list = true;
            continue;
        }
        if trimmed == "End of search list." {
            break;
        }
        if !in_search_list || trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("(framework directory)") {
            continue;
        }

        let candidate = PathBuf::from(trimmed);
        if candidate.exists() {
            includes.push(candidate);
        }
    }

    includes
}

fn linux_sysroot_include_candidates(sysroot: &Path) -> Vec<PathBuf> {
    if sysroot.as_os_str().is_empty() {
        return Vec::new();
    }

    vec![
        sysroot.join("usr/include"),
        sysroot.join("usr/local/include"),
        sysroot.join("include"),
        sysroot.join("include-fixed"),
    ]
}

fn discover_command_output_dir(command_with_args: &[&str]) -> Option<PathBuf> {
    let (program, args) = command_with_args.split_first()?;
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        return None;
    }

    let path = PathBuf::from(value);
    if path.exists() { Some(path) } else { None }
}

fn latest_versioned_include_dir(root: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(root).ok()?;
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join("include"))
        .filter(|path| path.join("mm_malloc.h").exists())
        .max()
}

fn read_compile_db_args(path: &Path, parse_entry: &Path) -> Result<Vec<String>> {
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
        .find(|command| resolve_command_file(db_dir, command) == parse_entry)
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

fn read_compile_db_translation_units(path: &Path, dir: &Path) -> Result<Vec<PathBuf>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read compile_commands.json: {}", path.display()))?;
    let commands: Vec<CompileCommand> = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse compile_commands.json: {}", path.display()))?;
    let db_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut units = BTreeSet::new();

    for command in &commands {
        let file = resolve_command_file(db_dir, command);
        if !is_source_translation_unit_file(&file) || !path_is_within(&file, dir) {
            continue;
        }
        units.insert(file);
    }

    Ok(units.into_iter().collect())
}

fn scan_dir_translation_units(dir: &Path) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(dir)
        .with_context(|| format!("failed to read input directory: {}", dir.display()))?;
    let mut source_units = BTreeSet::new();
    let mut header_units = BTreeSet::new();

    for entry in entries {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        if is_source_translation_unit_file(&path) {
            source_units.insert(path);
        } else if is_header_file(&path) {
            header_units.insert(path);
        }
    }

    if !source_units.is_empty() {
        Ok(source_units.into_iter().collect())
    } else {
        Ok(header_units.into_iter().collect())
    }
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
        normalize_clang_path(path)
    } else {
        normalize_clang_path(&base.join(path))
    }
}

fn normalize_clang_path(path: &Path) -> String {
    let value = path.display().to_string();
    if env::consts::OS == "windows" {
        value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
    } else {
        value
    }
}

fn path_is_within(path: &Path, dir: &Path) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    path.starts_with(dir)
}

fn is_source_translation_unit_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("c" | "cc" | "cpp" | "cxx")
    )
}

fn is_header_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("h" | "hh" | "hpp" | "hxx")
    )
}

fn split_command_line(command: &str) -> Vec<String> {
    command
        .split_whitespace()
        .skip(1)
        .map(ToString::to_string)
        .collect()
}

pub fn ensure_parse_entry_exists(parse_entry: &Path) -> Result<()> {
    if !parse_entry.exists() {
        anyhow::bail!("parse entry not found: {}", parse_entry.display());
    }
    Ok(())
}

pub fn ensure_header_exists(path: &Path) -> Result<()> {
    ensure_parse_entry_exists(path)
}

#[cfg(test)]
mod tests {
    use super::parse_driver_include_search_list;
    use std::path::PathBuf;

    #[test]
    fn parses_driver_include_search_list_from_verbose_output() {
        let stderr = r#"
#include "..." search starts here:
#include <...> search starts here:
 /usr/include/c++/13
 /usr/include/x86_64-linux-gnu/c++/13
 /usr/lib/llvm-18/lib/clang/18/include
 /usr/local/include
 /usr/include/x86_64-linux-gnu
 /usr/include
End of search list.
"#;

        let includes = parse_driver_include_search_list(stderr);
        assert_eq!(
            includes,
            vec![
                PathBuf::from("/usr/include/c++/13"),
                PathBuf::from("/usr/include/x86_64-linux-gnu/c++/13"),
                PathBuf::from("/usr/lib/llvm-18/lib/clang/18/include"),
                PathBuf::from("/usr/local/include"),
                PathBuf::from("/usr/include/x86_64-linux-gnu"),
                PathBuf::from("/usr/include"),
            ]
            .into_iter()
            .filter(|path| path.exists())
            .collect::<Vec<_>>()
        );
    }
}
