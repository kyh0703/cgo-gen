use std::{collections::BTreeSet, fs, path::Path};

use anyhow::{Context, Result, bail};

use crate::{
    config::Config,
    ir::{IrEnum, IrFunction, IrModule, IrType},
    parser,
};

pub fn generate_all(config: &Config, write_ir: bool) -> Result<()> {
    if config.input.headers.len() > 1 && !config.uses_default_output_names() {
        bail!(
            "multi-header generation does not support explicit output.header/source/ir overrides; leave them as defaults to emit one wrapper set per header"
        );
    }

    if config.input.headers.len() <= 1 {
        let scoped = config
            .input
            .headers
            .first()
            .cloned()
            .map(|header| config.scoped_to_header(header))
            .unwrap_or_else(|| config.clone());
        let parsed = parser::parse(&scoped)?;
        let ir = crate::ir::normalize(&scoped, &parsed)?;
        return generate(&scoped, &ir, write_ir);
    }

    let mut matched_go_targets = BTreeSet::new();
    for header in &config.input.headers {
        let scoped = config.scoped_to_header(header.clone());
        let parsed = parser::parse(&scoped)?;
        let scoped = scoped_config_for_parsed_targets(&scoped, &parsed, Some(&mut matched_go_targets));
        let ir = crate::ir::normalize(&scoped, &parsed)?;
        generate(&scoped, &ir, write_ir)?;
    }

    let unmatched_targets = config
        .go_structs
        .iter()
        .filter(|target| !matched_go_targets.contains(target.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !unmatched_targets.is_empty() {
        bail!(
            "go_structs targets did not match any parsed header output: {}",
            unmatched_targets.join(", ")
        );
    }

    Ok(())
}

fn scoped_config_for_parsed_targets(
    config: &Config,
    parsed: &parser::ParsedApi,
    mut matched_go_targets: Option<&mut BTreeSet<String>>,
) -> Config {
    let mut scoped = config.clone();
    let matched_targets = config
        .go_structs
        .iter()
        .filter(|target| {
            parsed.classes.iter().any(|class| {
                let owner = if class.namespace.is_empty() {
                    class.name.as_str().to_string()
                } else {
                    format!("{}::{}", class.namespace.join("::"), class.name)
                };
                go_struct_target_matches(target, &owner)
            })
        })
        .cloned()
        .collect::<Vec<_>>();

    if let Some(seen) = &mut matched_go_targets {
        for target in &matched_targets {
            seen.insert(target.clone());
        }
    }

    scoped.go_structs = matched_targets;
    scoped
}

pub fn generate(config: &Config, ir: &IrModule, write_ir: bool) -> Result<()> {
    fs::create_dir_all(&config.output.dir).with_context(|| {
        format!(
            "failed to create output dir: {}",
            config.output.dir.display()
        )
    })?;

    let header_path = config.output.dir.join(&config.output.header);
    let source_path = config.output.dir.join(&config.output.source);
    let ir_path = config.output.dir.join(&config.output.ir);
    fs::write(&header_path, render_header(config, ir))
        .with_context(|| format!("failed to write header: {}", header_path.display()))?;
    fs::write(&source_path, render_source(config, ir))
        .with_context(|| format!("failed to write source: {}", source_path.display()))?;
    for go_struct in render_go_structs(config, ir)? {
        let go_path = config.output.dir.join(&go_struct.filename);
        fs::write(&go_path, go_struct.contents)
            .with_context(|| format!("failed to write Go structs: {}", go_path.display()))?;
    }
    if write_ir {
        let serialized = serde_yaml::to_string(ir)?;
        fs::write(&ir_path, serialized)
            .with_context(|| format!("failed to write ir dump: {}", ir_path.display()))?;
    }
    Ok(())
}

pub fn write_ir(path: &Path, ir: &IrModule) -> Result<()> {
    let serialized = serde_yaml::to_string(ir)?;
    fs::write(path, serialized)
        .with_context(|| format!("failed to write ir dump: {}", path.display()))?;
    Ok(())
}

pub fn render_header(config: &Config, ir: &IrModule) -> String {
    let guard = format!(
        "{}_{}",
        config.naming.prefix.to_uppercase(),
        config.output.header.replace('.', "_").to_uppercase()
    );
    let mut out = String::new();
    out.push_str(&format!("#ifndef {guard}\n#define {guard}\n\n"));
    out.push_str("#include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n\n");
    out.push_str("#ifdef __cplusplus\nextern \"C\" {\n#endif\n\n");

    for opaque in &ir.opaque_types {
        out.push_str(&format!(
            "typedef struct {} {};\n",
            opaque.name, opaque.name
        ));
    }
    if !ir.opaque_types.is_empty() {
        out.push('\n');
    }

    for item in &ir.enums {
        render_enum_decl(&mut out, item);
    }

    for function in &ir.functions {
        out.push_str(&render_function_decl(function));
        out.push('\n');
    }

    if ir
        .functions
        .iter()
        .any(|function| function.returns.kind == "string")
    {
        out.push_str(&format!(
            "void {}_string_free(char* value);\n\n",
            config.naming.prefix
        ));
    }

    out.push_str("#ifdef __cplusplus\n}\n#endif\n\n");
    out.push_str(&format!("#endif /* {guard} */\n"));
    out
}

pub fn render_source(config: &Config, ir: &IrModule) -> String {
    let mut out = String::new();
    out.push_str(&format!("#include \"{}\"\n", config.output.header));
    out.push_str("#include <cstdlib>\n#include <cstring>\n#include <new>\n#include <string>\n\n");
    for header in &ir.source_headers {
        let include_name = Path::new(header)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(header.as_str());
        out.push_str(&format!("#include \"{}\"\n", include_name));
    }
    out.push('\n');

    for function in &ir.functions {
        out.push_str(&render_function_def(function));
        out.push('\n');
    }

    if ir
        .functions
        .iter()
        .any(|function| function.returns.kind == "string")
    {
        out.push_str(&render_string_free(config));
    }

    out
}

pub fn render_go_structs(config: &Config, ir: &IrModule) -> Result<Vec<GeneratedGoFile>> {
    if config.go_structs.is_empty() {
        return Ok(Vec::new());
    }

    let projections = build_go_structs(config, ir)?;
    let package_name = go_package_name(&config.output.dir);
    let mut out = String::new();
    out.push_str(&format!("package {}\n\n", package_name));

    for projection in projections {
        out.push_str(&format!("type {} struct {{\n", projection.name));
        for field in projection.fields {
            out.push_str(&format!("    {} {}\n", field.name, field.ty));
        }
        out.push_str("}\n\n");
    }

    Ok(vec![GeneratedGoFile {
        filename: config.go_filename(""),
        contents: out,
    }])
}

fn render_enum_decl(out: &mut String, item: &IrEnum) {
    out.push_str(&format!("typedef enum {} {{\n", item.name));
    for variant in &item.variants {
        match &variant.value {
            Some(value) => out.push_str(&format!("    {} = {},\n", variant.name, value)),
            None => out.push_str(&format!("    {},\n", variant.name)),
        }
    }
    out.push_str(&format!("}} {};\n\n", item.name));
}

#[derive(Debug)]
struct GoStruct {
    name: String,
    fields: Vec<GoField>,
}

#[derive(Debug)]
struct GoField {
    name: String,
    ty: String,
}

#[derive(Debug)]
pub struct GeneratedGoFile {
    pub filename: String,
    pub contents: String,
}

fn build_go_structs(config: &Config, ir: &IrModule) -> Result<Vec<GoStruct>> {
    let mut projections = Vec::new();

    for target in &config.go_structs {
        let class_methods = ir
            .functions
            .iter()
            .filter(|function| function.kind == "method")
            .filter(|function| {
                function
                    .owner_cpp_type
                    .as_deref()
                    .map(|owner| go_struct_target_matches(target, owner))
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();

        if class_methods.is_empty() {
            bail!("go_structs target `{target}` did not match any parsed class methods");
        }

        let owner = class_methods[0]
            .owner_cpp_type
            .as_deref()
            .unwrap_or(target.as_str());
        let setters = class_methods
            .iter()
            .filter_map(|function| {
                setter_suffix(function).map(|suffix| (suffix.to_string(), *function))
            })
            .collect::<std::collections::BTreeMap<_, _>>();

        let mut fields = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for function in class_methods {
            let Some(suffix) = getter_suffix(function) else {
                continue;
            };
            let Some(setter) = setters.get(suffix) else {
                continue;
            };
            if !seen.insert(suffix.to_string()) {
                continue;
            }

            let getter_ty = go_type_for_ir(&function.returns).ok_or_else(|| {
                anyhow::anyhow!(
                    "getter `{}` on `{owner}` has unsupported Go projection type `{}`",
                    function.cpp_name,
                    function.returns.cpp_type
                )
            })?;
            let setter_param = setter.params.get(1).ok_or_else(|| {
                anyhow::anyhow!("setter `{}` on `{owner}` is missing its value parameter", setter.cpp_name)
            })?;
            let setter_ty = go_type_for_ir(&setter_param.ty).ok_or_else(|| {
                anyhow::anyhow!(
                    "setter `{}` on `{owner}` has unsupported Go projection type `{}`",
                    setter.cpp_name,
                    setter_param.ty.cpp_type
                )
            })?;

            if getter_ty != setter_ty {
                bail!(
                    "getter/setter type mismatch for `{owner}` field `{suffix}`: getter -> {getter_ty}, setter -> {setter_ty}"
                );
            }

            fields.push(GoField {
                name: go_field_name(suffix),
                ty: getter_ty,
            });
        }

        if fields.is_empty() {
            bail!("go_structs target `{target}` did not yield any getter/setter field pairs");
        }

        projections.push(GoStruct {
            name: leaf_cpp_name(owner).to_string(),
            fields,
        });
    }

    Ok(projections)
}

fn go_struct_target_matches(target: &str, owner: &str) -> bool {
    target == owner || target == leaf_cpp_name(owner)
}

fn getter_suffix<'a>(function: &'a IrFunction) -> Option<&'a str> {
    if function.kind != "method" || function.params.len() != 1 || function.returns.kind == "void" {
        return None;
    }
    function
        .cpp_name
        .rsplit("::")
        .next()
        .and_then(|name| name.strip_prefix("Get"))
        .filter(|suffix| !suffix.is_empty())
}

fn setter_suffix<'a>(function: &'a IrFunction) -> Option<&'a str> {
    if function.kind != "method" || function.params.len() != 2 || function.returns.kind != "void" {
        return None;
    }
    function
        .cpp_name
        .rsplit("::")
        .next()
        .and_then(|name| name.strip_prefix("Set"))
        .filter(|suffix| !suffix.is_empty())
}

fn go_type_for_ir(ty: &IrType) -> Option<String> {
    match ty.kind.as_str() {
        "string" | "c_string" => Some("string".to_string()),
        "primitive" => match normalize_type_key(&ty.cpp_type).as_str() {
            "bool" => Some("bool".to_string()),
            "float" => Some("float32".to_string()),
            "double" => Some("float64".to_string()),
            "int8" | "int8_t" => Some("int8".to_string()),
            "int16" | "int16_t" => Some("int16".to_string()),
            "int32" | "int32_t" => Some("int32".to_string()),
            "int64" | "int64_t" => Some("int64".to_string()),
            "uint8" | "uint8_t" => Some("uint8".to_string()),
            "uint16" | "uint16_t" => Some("uint16".to_string()),
            "uint32" | "uint32_t" => Some("uint32".to_string()),
            "uint64" | "uint64_t" => Some("uint64".to_string()),
            "int" => Some("int".to_string()),
            "short" => Some("int16".to_string()),
            "long" => Some("int64".to_string()),
            "size_t" => Some("uintptr".to_string()),
            _ => None,
        },
        _ => None,
    }
}

fn normalize_type_key(value: &str) -> String {
    value
        .replace(' ', "")
        .trim_start_matches("const")
        .trim_end_matches('&')
        .trim_end_matches('*')
        .to_string()
}

fn go_field_name(value: &str) -> String {
    value
        .split('_')
        .flat_map(split_pascal_tokens)
        .map(|token| match token.to_ascii_lowercase().as_str() {
            "id" => "ID".to_string(),
            "url" => "URL".to_string(),
            "db" => "DB".to_string(),
            "api" => "API".to_string(),
            "http" => "HTTP".to_string(),
            "https" => "HTTPS".to_string(),
            "json" => "JSON".to_string(),
            "xml" => "XML".to_string(),
            other if token.chars().all(|ch| ch.is_uppercase()) => other.to_ascii_uppercase(),
            _ => token,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn split_pascal_tokens(value: &str) -> Vec<String> {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Vec::new();
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
}

fn leaf_cpp_name(value: &str) -> &str {
    value.rsplit("::").next().unwrap_or(value)
}

fn go_package_name(path: &Path) -> String {
    let source = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("bindings");
    let sanitized = source
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "bindings".to_string()
    } else {
        sanitized
    }
}

fn render_function_decl(function: &IrFunction) -> String {
    let params = render_param_list(function);
    format!("{} {}({});", function.returns.c_type, function.name, params)
}

fn render_param_list(function: &IrFunction) -> String {
    if function.params.is_empty() {
        return "void".to_string();
    }
    function
        .params
        .iter()
        .map(|param| format!("{} {}", param.ty.c_type, param.name))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_function_def(function: &IrFunction) -> String {
    let signature = format!(
        "{} {}({})",
        function.returns.c_type,
        function.name,
        render_param_list(function)
    );
    let body = match function.kind.as_str() {
        "constructor" => render_constructor_body(function),
        "destructor" => render_destructor_body(function),
        "method" => render_method_body(function),
        _ => render_free_function_body(function),
    };
    format!("{signature} {{\n{body}}}\n")
}

fn render_constructor_body(function: &IrFunction) -> String {
    let owner = function.owner_cpp_type.as_deref().unwrap_or("void");
    let call_args = call_args(function, 0);
    format!(
        "    return reinterpret_cast<{}>(new {}({}));\n",
        function.returns.c_type, owner, call_args
    )
}

fn render_destructor_body(function: &IrFunction) -> String {
    let owner = function.owner_cpp_type.as_deref().unwrap_or("void");
    format!("    delete reinterpret_cast<{}*>(self);\n", owner)
}

fn render_method_body(function: &IrFunction) -> String {
    let owner = function.owner_cpp_type.as_deref().unwrap_or("void");
    let receiver = if function.is_const.unwrap_or(false) {
        format!("reinterpret_cast<const {}*>(self)", owner)
    } else {
        format!("reinterpret_cast<{}*>(self)", owner)
    };
    let method_name = function
        .cpp_name
        .rsplit("::")
        .next()
        .unwrap_or(&function.cpp_name);
    render_callable_body(function, &format!("{receiver}->{method_name}"), 1)
}

fn render_free_function_body(function: &IrFunction) -> String {
    render_callable_body(function, &function.cpp_name, 0)
}

fn render_callable_body(function: &IrFunction, target: &str, arg_start: usize) -> String {
    let args = call_args(function, arg_start);
    match function.returns.kind.as_str() {
        "void" => format!("    {}({});\n", target, args),
        "string" => format!(
            "    std::string result = {}({});\n    char* buffer = static_cast<char*>(std::malloc(result.size() + 1));\n    if (buffer == nullptr) {{\n        return nullptr;\n    }}\n    std::memcpy(buffer, result.c_str(), result.size() + 1);\n    return buffer;\n",
            target, args
        ),
        _ => format!("    return {}({});\n", target, args),
    }
}

fn call_args(function: &IrFunction, start: usize) -> String {
    function
        .params
        .iter()
        .skip(start)
        .map(|param| render_cpp_arg(param.ty.clone(), &param.name))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_cpp_arg(ty: IrType, name: &str) -> String {
    match ty.kind.as_str() {
        "string" => format!("std::string({name} != nullptr ? {name} : \"\")"),
        "reference" => format!("*{name}"),
        _ => name.to_string(),
    }
}

fn render_string_free(config: &Config) -> String {
    format!(
        "void {}_string_free(char* value) {{\n    std::free(value);\n}}\n",
        config.naming.prefix
    )
}
