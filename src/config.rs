use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub version: Option<u32>,
    #[serde(default)]
    pub project_root: Option<PathBuf>,
    pub input: InputConfig,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub naming: NamingConfig,
    #[serde(default)]
    pub policies: PolicyConfig,
    #[serde(skip)]
    pub known_model_types: Vec<String>,
    #[serde(skip)]
    pub known_model_projections: Vec<KnownModelProjection>,
    #[serde(skip)]
    pub target_header: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct KnownModelProjection {
    pub cpp_type: String,
    pub handle_name: String,
    pub go_name: String,
    pub output_header: String,
    pub constructor_symbol: String,
    pub destructor_symbol: Option<String>,
    pub fields: Vec<KnownModelField>,
}

#[derive(Debug, Clone, Default)]
pub struct KnownModelField {
    pub go_name: String,
    pub go_type: String,
    pub getter_symbol: String,
    pub setter_symbol: String,
    pub return_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InputConfig {
    #[serde(default)]
    pub dir: Option<PathBuf>,
    #[serde(default)]
    pub headers: Vec<PathBuf>,
    #[serde(default)]
    pub dirs: Vec<PathBuf>,
    #[serde(default)]
    pub header_dirs: Vec<PathBuf>,
    #[serde(default)]
    pub translation_units: Vec<PathBuf>,
    #[serde(default)]
    pub compile_commands: Option<PathBuf>,
    #[serde(default)]
    pub include_dirs: Vec<PathBuf>,
    #[serde(default)]
    pub clang_args: Vec<String>,
    #[serde(default)]
    pub allow_diagnostics: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_output_dir")]
    pub dir: PathBuf,
    #[serde(default = "default_header_name")]
    pub header: String,
    #[serde(default = "default_source_name")]
    pub source: String,
    #[serde(default = "default_ir_name")]
    pub ir: String,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            dir: default_output_dir(),
            header: default_header_name(),
            source: default_source_name(),
            ir: default_ir_name(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamingConfig {
    #[serde(default = "default_prefix")]
    pub prefix: String,
    #[serde(default = "default_style")]
    pub style: String,
}

impl Default for NamingConfig {
    fn default() -> Self {
        Self {
            prefix: default_prefix(),
            style: default_style(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    #[serde(default = "default_string_mode")]
    pub string_mode: String,
    #[serde(default = "default_enum_mode")]
    pub enum_mode: String,
    #[serde(default)]
    pub unsupported: UnsupportedPolicy,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            string_mode: default_string_mode(),
            enum_mode: default_enum_mode(),
            unsupported: UnsupportedPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsupportedPolicy {
    #[serde(default = "default_error")]
    pub templates: String,
    #[serde(default = "default_skip")]
    pub stl_containers: String,
    #[serde(default = "default_error")]
    pub exceptions: String,
}

impl Default for UnsupportedPolicy {
    fn default() -> Self {
        Self {
            templates: default_error(),
            stl_containers: default_skip(),
            exceptions: default_error(),
        }
    }
}

fn default_output_dir() -> PathBuf {
    PathBuf::from("gen")
}
fn default_header_name() -> String {
    "wrapper.h".to_string()
}
fn default_source_name() -> String {
    "wrapper.cpp".to_string()
}
fn default_ir_name() -> String {
    "wrapper.ir.yaml".to_string()
}
fn default_prefix() -> String {
    "cgowrap".to_string()
}
fn default_style() -> String {
    "preserve".to_string()
}
fn default_string_mode() -> String {
    "c_str".to_string()
}
fn default_enum_mode() -> String {
    "c_enum".to_string()
}
fn default_error() -> String {
    "error".to_string()
}
fn default_skip() -> String {
    "skip".to_string()
}

fn resolve_relative_clang_args(args: &mut Vec<String>, base_dir: &Path) {
    let mut resolved = Vec::with_capacity(args.len());
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];

        if arg == "-I" || arg == "-isystem" {
            resolved.push(arg.clone());
            if let Some(value) = args.get(index + 1) {
                resolved.push(resolve_relative_clang_path_arg(value, base_dir));
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }

        if let Some(value) = arg.strip_prefix("-I") {
            resolved.push(format!(
                "-I{}",
                resolve_relative_clang_path_arg(value, base_dir)
            ));
            index += 1;
            continue;
        }

        if let Some(value) = arg.strip_prefix("-isystem") {
            resolved.push(format!(
                "-isystem{}",
                resolve_relative_clang_path_arg(value, base_dir)
            ));
            index += 1;
            continue;
        }

        resolved.push(arg.clone());
        index += 1;
    }

    *args = resolved;
}

fn resolve_relative_clang_path_arg(value: &str, base_dir: &Path) -> String {
    if value.is_empty() {
        return String::new();
    }

    let path = Path::new(value);
    if path.is_absolute() {
        return value.to_string();
    }

    let joined = base_dir.join(path);
    normalize_clang_config_path(&joined.canonicalize().unwrap_or(joined))
}

fn normalize_clang_config_path(path: &Path) -> String {
    let value = path.display().to_string();
    if cfg!(windows) {
        value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
    } else {
        value
    }
}

fn resolve_path(path: &mut PathBuf, base_dir: &Path) {
    if path.is_relative() {
        *path = base_dir.join(&*path);
    }
    if let Ok(canonical) = path.canonicalize() {
        *path = canonical;
    }
}

fn push_unique_path(paths: &mut Vec<PathBuf>, value: PathBuf) {
    if !paths.iter().any(|candidate| candidate == &value) {
        paths.push(value);
    }
}

fn is_supported_header_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("h" | "hh" | "hpp" | "hxx")
    )
}

fn is_supported_translation_unit_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("cc" | "cpp" | "cxx")
    )
}

fn collect_headers_from_dir(dir: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        bail!("header directory not found: {}", dir.display());
    }
    if !dir.is_dir() {
        bail!("header_dirs entry must be a directory: {}", dir.display());
    }

    let mut entries = fs::read_dir(dir)
        .with_context(|| format!("failed to read header directory: {}", dir.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("failed to list header directory: {}", dir.display()))?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_headers_from_dir(&path, output)?;
            continue;
        }
        if !is_supported_header_path(&path) {
            continue;
        }
        let canonical = path.canonicalize().unwrap_or(path);
        push_unique_path(output, canonical);
    }

    Ok(())
}

fn collect_translation_units_from_dir(dir: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        bail!("input.dirs entry not found: {}", dir.display());
    }
    if !dir.is_dir() {
        bail!("input.dirs entry must be a directory: {}", dir.display());
    }

    let mut entries = fs::read_dir(dir)
        .with_context(|| format!("failed to read input directory: {}", dir.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("failed to list input directory: {}", dir.display()))?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_translation_units_from_dir(&path, output)?;
            continue;
        }
        if !is_supported_translation_unit_path(&path) {
            continue;
        }
        let canonical = path.canonicalize().unwrap_or(path);
        push_unique_path(output, canonical);
    }

    Ok(())
}
impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let mut config: Self = serde_yaml::from_str(&raw)
            .with_context(|| format!("failed to parse YAML config: {}", path.display()))?;
        config.resolve_relative_paths(path)?;
        config.validate()?;
        Ok(config)
    }

    pub fn compile_commands_path(&self) -> Option<PathBuf> {
        self.input.compile_commands.clone()
    }

    pub fn parse_entries(&self) -> Vec<PathBuf> {
        if self.input.translation_units.is_empty() {
            self.input.headers.clone()
        } else {
            self.input.translation_units.clone()
        }
    }

    pub fn uses_default_output_names(&self) -> bool {
        self.output.header == default_header_name()
            && self.output.source == default_source_name()
            && self.output.ir == default_ir_name()
    }

    pub fn scoped_to_header(&self, header: PathBuf) -> Self {
        let mut scoped = self.clone();
        scoped.target_header = Some(header.clone());
        if scoped.input.dir.is_none() {
            scoped.input.headers = vec![header];
        } else {
            scoped.input.headers.clear();
        }
        scoped.apply_output_defaults();
        scoped
    }

    fn resolve_relative_paths(&mut self, config_path: &Path) -> Result<()> {
        let base_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
        if let Some(dir) = &mut self.input.dir {
            if dir.is_relative() {
                *dir = base_dir.join(&*dir);
            }
            if let Ok(canonical) = dir.canonicalize() {
                *dir = canonical;
            }
        }
        for header in &mut self.input.headers {
            resolve_path(header, base_dir);
        }
        for header_dir in &mut self.input.header_dirs {
            resolve_path(header_dir, base_dir);
        }
        for dir in &mut self.input.dirs {
            resolve_path(dir, base_dir);
        }
        for header_dir in &mut self.input.header_dirs {
            resolve_path(header_dir, base_dir);
        }
        for entry in &mut self.input.translation_units {
            resolve_path(entry, base_dir);
        }
        let mut expanded_headers = Vec::new();
        for header in &self.input.headers {
            push_unique_path(&mut expanded_headers, header.clone());
        }
        let mut expanded_translation_units = Vec::new();
        for entry in &self.input.translation_units {
            push_unique_path(&mut expanded_translation_units, entry.clone());
        }
        for dir in &self.input.dirs {
            collect_headers_from_dir(dir, &mut expanded_headers)?;
            collect_translation_units_from_dir(dir, &mut expanded_translation_units)?;
        }
        for header_dir in &self.input.header_dirs {
            collect_headers_from_dir(header_dir, &mut expanded_headers)?;
        }
        self.input.headers = expanded_headers;
        self.input.translation_units = expanded_translation_units;
        if let Some(compdb) = &mut self.input.compile_commands {
            resolve_path(compdb, base_dir);
        }
        for include_dir in &mut self.input.include_dirs {
            resolve_path(include_dir, base_dir);
        }
        resolve_relative_clang_args(&mut self.input.clang_args, base_dir);
        if !self.input.include_dirs.is_empty() {
            let mut include_args = self
                .input
                .include_dirs
                .iter()
                .map(|path| format!("-I{}", normalize_clang_config_path(path)))
                .collect::<Vec<_>>();
            include_args.extend(self.input.clang_args.clone());
            self.input.clang_args = include_args;
        }
        if self.output.dir.is_relative() {
            self.output.dir = base_dir.join(&self.output.dir);
        }
        self.apply_output_defaults();
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        if self.input.dir.is_none() && self.input.headers.is_empty() {
            bail!("config.input.dir or config.input.headers must be set");
        }
        if let Some(dir) = &self.input.dir {
            if dir.exists() && !dir.is_dir() {
                bail!("config.input.dir must point to a directory: {}", dir.display());
            }
        }
        Ok(())
    }

    pub fn go_filename(&self, _value: &str) -> String {
        let stem = Path::new(&self.output.header)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.is_empty())
            .unwrap_or("wrapper");
        format!("{stem}.go")
    }

    pub fn raw_output_dir(&self) -> PathBuf {
        self.output.dir.clone()
    }

    pub fn go_output_dir(&self) -> PathBuf {
        self.output.dir.clone()
    }

    pub fn raw_include_for_go(&self, header: &str) -> String {
        header.to_string()
    }

    fn apply_output_defaults(&mut self) {
        if !self.uses_default_output_names() {
            return;
        }

        let Some(basename) = self.infer_output_basename() else {
            return;
        };
        self.output.header = format!("{basename}.h");
        self.output.source = format!("{basename}.cpp");
        self.output.ir = format!("{basename}.ir.yaml");
    }

    fn infer_output_basename(&self) -> Option<String> {
        let header = if let Some(header) = self.target_header.as_ref() {
            header
        } else {
            if self.input.headers.len() != 1 {
                return None;
            }
            self.input.headers.first()?
        };
        let stem = header.file_stem()?.to_str()?;
        Some(format!("{}_wrapper", to_snake_case(stem)))
    }

    pub fn with_known_model_types(mut self, known_model_types: Vec<String>) -> Self {
        self.known_model_types = known_model_types;
        self
    }

    pub fn with_known_model_projections(
        mut self,
        known_model_projections: Vec<KnownModelProjection>,
    ) -> Self {
        self.known_model_projections = known_model_projections;
        self
    }

    pub fn known_model_projection(&self, cpp_type: &str) -> Option<&KnownModelProjection> {
        let base = base_cpp_type_name(cpp_type);
        self.known_model_projections.iter().find(|projection| {
            let normalized = base_cpp_type_name(&projection.cpp_type);
            normalized == base
                || normalized.rsplit("::").next().unwrap_or(&normalized) == base
                || base.rsplit("::").next().unwrap_or(&base) == normalized
        })
    }

    pub fn is_known_model_type(&self, cpp_type: &str) -> bool {
        let base = base_cpp_type_name(cpp_type);
        self.known_model_types.iter().any(|candidate| {
            let normalized = base_cpp_type_name(candidate);
            normalized == base
                || normalized.rsplit("::").next().unwrap_or(&normalized) == base
                || base.rsplit("::").next().unwrap_or(&base) == normalized
        })
    }
}

fn base_cpp_type_name(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("const ")
        .trim_end_matches('&')
        .trim_end_matches('*')
        .trim()
        .to_string()
}

fn to_snake_case(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return String::new();
    }

    let mut tokens = Vec::new();
    let mut start = 0;
    for index in 1..chars.len() {
        let prev = chars[index - 1];
        let current = chars[index];
        let next = chars.get(index + 1).copied();

        let boundary = (prev.is_lowercase() && current.is_uppercase())
            || (prev.is_ascii_digit() && !current.is_ascii_digit())
            || (!prev.is_ascii_digit() && current.is_ascii_digit())
            || (prev.is_uppercase()
                && current.is_uppercase()
                && next.map(|ch| ch.is_lowercase()).unwrap_or(false));

        if boundary {
            tokens.push(chars[start..index].iter().collect::<String>());
            start = index;
        }
    }
    tokens.push(chars[start..].iter().collect::<String>());

    tokens
        .into_iter()
        .map(|token| token.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("_")
}
