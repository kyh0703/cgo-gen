use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{Result, anyhow};

use crate::{
    config::{Config, HeaderRole},
    ir::{IrFunction, IrModule, IrType},
};

#[derive(Debug)]
pub struct GeneratedGoFile {
    pub filename: String,
    pub contents: String,
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
struct ModelProjection {
    go_name: String,
    fields: Vec<ModelProjectionField>,
}

#[derive(Debug)]
struct ModelProjectionField {
    go_name: String,
    go_type: String,
}

#[derive(Debug)]
struct GoEnum {
    name: String,
    variants: Vec<GoEnumVariant>,
}

#[derive(Debug)]
struct GoEnumVariant {
    name: String,
    value: String,
}

pub fn render_go_models(config: &Config, ir: &IrModule) -> Result<Vec<GeneratedGoFile>> {
    let role = config
        .input
        .headers
        .first()
        .map(|header| config.header_role(header))
        .unwrap_or(HeaderRole::Unclassified);

    let projections = match role {
        HeaderRole::Facade | HeaderRole::Unclassified => Vec::new(),
        HeaderRole::Model => build_all_model_structs(ir)?,
    };
    let enums = if role == HeaderRole::Model {
        build_go_enums(ir)
    } else {
        Vec::new()
    };

    if projections.is_empty() && enums.is_empty() {
        return Ok(Vec::new());
    }

    Ok(vec![GeneratedGoFile {
        filename: config.go_filename(""),
        contents: render_go_file(config, &enums, &projections),
    }])
}

pub fn render_go_structs(config: &Config, ir: &IrModule) -> Result<Vec<GeneratedGoFile>> {
    if config
        .input
        .headers
        .first()
        .map(|header| config.header_role(header) != HeaderRole::Model)
        .unwrap_or(true)
    {
        return Ok(Vec::new());
    }

    let projections = build_all_model_structs(ir)?;
    Ok(vec![GeneratedGoFile {
        filename: config.go_filename(""),
        contents: render_go_file(config, &[], &projections),
    }])
}

fn render_go_file(config: &Config, enums: &[GoEnum], projections: &[GoStruct]) -> String {
    let package_name = go_package_name(&config.output.dir);
    let mut out = String::new();
    out.push_str(&format!("package {}\n\n", package_name));

    for item in enums {
        out.push_str(&format!("type {} int64\n\n", item.name));
        out.push_str("const (\n");
        for variant in &item.variants {
            out.push_str(&format!(
                "    {} {} = {}\n",
                variant.name, item.name, variant.value
            ));
        }
        out.push_str(")\n\n");
    }

    for projection in projections {
        out.push_str(&format!("type {} struct {{\n", projection.name));
        for field in &projection.fields {
            out.push_str(&format!("    {} {}\n", field.name, field.ty));
        }
        out.push_str("}\n\n");
    }

    out
}

fn build_go_enums(ir: &IrModule) -> Vec<GoEnum> {
    ir.enums
        .iter()
        .map(|item| GoEnum {
            name: leaf_cpp_name(&item.cpp_name).to_string(),
            variants: item
                .variants
                .iter()
                .enumerate()
                .map(|(index, variant)| GoEnumVariant {
                    name: variant.name.clone(),
                    value: variant.value.clone().unwrap_or_else(|| index.to_string()),
                })
                .collect(),
        })
        .collect()
}

fn build_all_model_structs(ir: &IrModule) -> Result<Vec<GoStruct>> {
    Ok(build_all_model_projections(ir)?
        .into_iter()
        .map(|projection| GoStruct {
            name: projection.go_name,
            fields: projection
                .fields
                .into_iter()
                .map(|field| GoField {
                    name: field.go_name,
                    ty: field.go_type,
                })
                .collect(),
        })
        .collect())
}

fn build_all_model_projections(ir: &IrModule) -> Result<Vec<ModelProjection>> {
    let mut methods_by_owner = BTreeMap::<String, Vec<&IrFunction>>::new();
    for function in ir
        .functions
        .iter()
        .filter(|function| function.kind == "method")
    {
        let Some(owner) = &function.owner_cpp_type else {
            continue;
        };
        methods_by_owner
            .entry(owner.clone())
            .or_default()
            .push(function);
    }

    let mut projections = Vec::new();
    for (owner, class_methods) in methods_by_owner {
        if let Some(projection) = build_model_projection(&owner, &class_methods)? {
            projections.push(projection);
        }
    }
    Ok(projections)
}

fn build_model_projection(
    owner: &str,
    class_methods: &[&IrFunction],
) -> Result<Option<ModelProjection>> {
    let setters = class_methods
        .iter()
        .filter_map(|function| {
            setter_suffix(function).map(|suffix| (suffix.to_string(), *function))
        })
        .collect::<BTreeMap<_, _>>();

    let mut fields = Vec::new();
    let mut seen = BTreeSet::new();
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

        let Some(getter_ty) = go_type_for_ir(&function.returns) else {
            continue;
        };
        let setter_param = setter.params.get(1).ok_or_else(|| {
            anyhow!(
                "setter `{}` on `{owner}` is missing its value parameter",
                setter.cpp_name
            )
        })?;
        let Some(setter_ty) = go_type_for_ir(&setter_param.ty) else {
            continue;
        };

        if getter_ty != setter_ty {
            continue;
        }

        fields.push(ModelProjectionField {
            go_name: go_field_name(suffix),
            go_type: getter_ty,
        });
    }

    if fields.is_empty() {
        return Ok(None);
    }

    Ok(Some(ModelProjection {
        go_name: leaf_cpp_name(owner).to_string(),
        fields,
    }))
}

fn getter_suffix(function: &IrFunction) -> Option<&str> {
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

fn setter_suffix(function: &IrFunction) -> Option<&str> {
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
