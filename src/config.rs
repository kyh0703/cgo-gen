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
    pub files: FileRoleConfig,
    #[serde(default)]
    pub naming: NamingConfig,
    #[serde(default)]
    pub policies: PolicyConfig,
    #[serde(skip)]
    pub known_model_types: Vec<String>,
    #[serde(skip)]
    pub known_model_projections: Vec<KnownModelProjection>,
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
    pub return_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InputConfig {
    #[serde(default)]
    pub dir: Option<PathBuf>,
    #[serde(default)]
    pub headers: Vec<PathBuf>,
    #[serde(default)]
    pub compile_commands: Option<PathBuf>,
    #[serde(default)]
    pub clang_args: Vec<String>,
    #[serde(default)]
    pub allow_diagnostics: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileRoleConfig {
    #[serde(default)]
    pub model: Vec<PathBuf>,
    #[serde(default)]
    pub facade: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderRole {
    Model,
    Facade,
    Unclassified,
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
        scoped.input.dir = None;
        scoped.input.headers = vec![header];
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
        for header in &mut self.files.model {
            if header.is_relative() {
                *header = base_dir.join(&*header);
            }
            if let Ok(canonical) = header.canonicalize() {
                *header = canonical;
            }
        }
        for header in &mut self.files.facade {
            if header.is_relative() {
                *header = base_dir.join(&*header);
            }
            if let Ok(canonical) = header.canonicalize() {
                *header = canonical;
            }
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
        let enforces_header_membership = !self.input.headers.is_empty();
        for header in &self.files.model {
            if enforces_header_membership
                && !self
                .input
                .headers
                .iter()
                .any(|candidate| candidate == header)
            {
                bail!(
                    "files.model entry must also appear in input.headers: {}",
                    header.display()
                );
            }
        }
        for header in &self.files.facade {
            if enforces_header_membership
                && !self
                .input
                .headers
                .iter()
                .any(|candidate| candidate == header)
            {
                bail!(
                    "files.facade entry must also appear in input.headers: {}",
                    header.display()
                );
            }
            if self.files.model.iter().any(|candidate| candidate == header) {
                bail!(
                    "header cannot be classified as both model and facade: {}",
                    header.display()
                );
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
        self.output.dir.join("raw")
    }

    pub fn model_output_dir(&self) -> PathBuf {
        self.output.dir.join("model")
    }

    pub fn facade_output_dir(&self) -> PathBuf {
        self.output.dir.join("facade")
    }

    pub fn raw_include_for_go(&self, header: &str) -> String {
        format!("../raw/{header}")
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

    pub fn header_role(&self, header: &Path) -> HeaderRole {
        if self.files.model.iter().any(|candidate| candidate == header) {
            HeaderRole::Model
        } else if self
            .files
            .facade
            .iter()
            .any(|candidate| candidate == header)
        {
            HeaderRole::Facade
        } else {
            HeaderRole::Unclassified
        }
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

    pub fn is_known_model_type(&self, cpp_type: &str) -> bool {
        let base = base_cpp_type_name(cpp_type);
        self.known_model_types.iter().any(|candidate| {
            let normalized = base_cpp_type_name(candidate);
            normalized == base
                || normalized.rsplit("::").next().unwrap_or(&normalized) == base
                || base.rsplit("::").next().unwrap_or(&base) == normalized
        })
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
