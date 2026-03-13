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
    pub input: InputConfig,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub filter: FilterConfig,
    #[serde(default)]
    pub go_structs: Vec<String>,
    #[serde(default)]
    pub naming: NamingConfig,
    #[serde(default)]
    pub policies: PolicyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InputConfig {
    pub headers: Vec<PathBuf>,
    #[serde(default)]
    pub compile_commands: Option<PathBuf>,
    #[serde(default)]
    pub clang_args: Vec<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterConfig {
    #[serde(default)]
    pub namespaces: Vec<String>,
    #[serde(default)]
    pub exclude_namespaces: Vec<String>,
    #[serde(default)]
    pub classes: Vec<String>,
    #[serde(default)]
    pub exclude_classes: Vec<String>,
    #[serde(default)]
    pub functions: Vec<String>,
    #[serde(default)]
    pub exclude_functions: Vec<String>,
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default)]
    pub exclude_methods: Vec<String>,
    #[serde(default)]
    pub enums: Vec<String>,
    #[serde(default)]
    pub exclude_enums: Vec<String>,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub exclude_types: Vec<String>,
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

    pub fn uses_default_output_names(&self) -> bool {
        self.output.header == default_header_name()
            && self.output.source == default_source_name()
            && self.output.ir == default_ir_name()
    }

    pub fn scoped_to_header(&self, header: PathBuf) -> Self {
        let mut scoped = self.clone();
        scoped.input.headers = vec![header];
        scoped.apply_output_defaults();
        scoped
    }

    fn resolve_relative_paths(&mut self, config_path: &Path) -> Result<()> {
        let base_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
        for header in &mut self.input.headers {
            if header.is_relative() {
                *header = base_dir.join(&*header);
            }
            if let Ok(canonical) = header.canonicalize() {
                *header = canonical;
            }
        }
        if let Some(compdb) = &mut self.input.compile_commands {
            if compdb.is_relative() {
                *compdb = base_dir.join(&*compdb);
            }
            if let Ok(canonical) = compdb.canonicalize() {
                *compdb = canonical;
            }
        }
        if self.output.dir.is_relative() {
            self.output.dir = base_dir.join(&self.output.dir);
        }
        self.apply_output_defaults();
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        if self.input.headers.is_empty() {
            bail!("config.input.headers must not be empty");
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
        if self.input.headers.len() != 1 {
            return None;
        }
        let header = self.input.headers.first()?;
        let stem = header.file_stem()?.to_str()?;
        Some(format!("{}_wrapper", to_snake_case(stem)))
    }
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
