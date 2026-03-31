use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{Result, anyhow, bail};

use crate::{
    config::{Config, HeaderRole, KnownModelField, KnownModelProjection},
    ir::{IrFunction, IrModule, IrType},
};

#[derive(Debug)]
pub struct GeneratedGoFile {
    pub filename: String,
    pub contents: String,
}

#[derive(Debug, Clone)]
struct ModelProjection {
    cpp_type: String,
    go_name: String,
    handle_name: String,
    constructor_symbol: String,
    destructor_symbol: String,
    fields: Vec<ModelProjectionField>,
}

#[derive(Debug, Clone)]
struct ModelProjectionField {
    go_name: String,
    go_type: String,
    getter_symbol: String,
    setter_symbol: String,
    return_kind: String,
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
        HeaderRole::Model => build_all_model_projections(ir)?,
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
    render_go_models(config, ir)
}

fn render_go_file(config: &Config, enums: &[GoEnum], projections: &[ModelProjection]) -> String {
    let package_name = go_package_name(&config.output.dir);
    let requires_unsafe = projections.iter().any(|projection| {
        projection
            .fields
            .iter()
            .any(|field| field.go_type == "string")
    });
    let mut out = String::new();
    out.push_str(&format!("package {}\n\n", package_name));

    if !projections.is_empty() {
        out.push_str("/*\n");
        out.push_str("#include <stdlib.h>\n");
        out.push_str(&format!(
            "#include \"{}\"\n",
            config.raw_include_for_go(&config.output.header)
        ));
        out.push_str("*/\n");
        out.push_str("import \"C\"\n\n");
        out.push_str("import \"errors\"\n\n");
        if requires_unsafe {
            out.push_str("import \"unsafe\"\n\n");
        }
    }

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
        out.push_str(&render_model_wrapper(config, projection));
        out.push('\n');
    }

    out
}

fn render_model_wrapper(config: &Config, projection: &ModelProjection) -> String {
    let mut out = String::new();
    let receiver = receiver_name(&projection.go_name);
    out.push_str(&format!(
        "type {} struct {{\n    ptr *C.{}\n}}\n\n",
        projection.go_name, projection.handle_name
    ));
    out.push_str(&format!(
        "func New{}() (*{}, error) {{\n    ptr := C.{}()\n    if ptr == nil {{\n        return nil, errors.New(\"wrapper returned nil model handle\")\n    }}\n    return &{}{{ptr: ptr}}, nil\n}}\n\n",
        projection.go_name,
        projection.go_name,
        projection.constructor_symbol,
        projection.go_name
    ));
    out.push_str(&format!(
        "func ({} *{}) Close() {{\n    if {} == nil || {}.ptr == nil {{\n        return\n    }}\n    C.{}({}.ptr)\n    {}.ptr = nil\n}}\n\n",
        receiver,
        projection.go_name,
        receiver,
        receiver,
        projection.destructor_symbol,
        receiver,
        receiver
    ));
    out.push_str(&format!(
        "func require{}Handle(value *{}) *C.{} {{\n    if value == nil || value.ptr == nil {{\n        panic(\"{} handle is nil\")\n    }}\n    return value.ptr\n}}\n\n",
        projection.go_name,
        projection.go_name,
        projection.handle_name,
        projection.go_name
    ));
    out.push_str(&format!(
        "func optional{}Handle(value *{}) *C.{} {{\n    if value == nil {{\n        return nil\n    }}\n    return require{}Handle(value)\n}}\n\n",
        projection.go_name,
        projection.go_name,
        projection.handle_name,
        projection.go_name
    ));
    for field in &projection.fields {
        out.push_str(&render_model_getter(config, projection, field));
        out.push('\n');
        out.push_str(&render_model_setter(projection, field));
        out.push('\n');
    }
    out
}

fn render_model_getter(
    config: &Config,
    projection: &ModelProjection,
    field: &ModelProjectionField,
) -> String {
    let receiver = receiver_name(&projection.go_name);
    let mut out = String::new();
    out.push_str(&format!(
        "func ({} *{}) Get{}() {} {{\n",
        receiver, projection.go_name, field.go_name, field.go_type
    ));
    out.push_str(&format!(
        "    handle := require{}Handle({})\n",
        projection.go_name, receiver
    ));
    match field.return_kind.as_str() {
        "string" => {
            out.push_str(&format!("    raw := C.{}(handle)\n", field.getter_symbol));
            out.push_str("    if raw == nil {\n        return \"\"\n    }\n");
            out.push_str(&format!(
                "    defer C.{}_string_free(raw)\n",
                config.naming.prefix
            ));
            out.push_str("    return C.GoString(raw)\n");
        }
        "c_string" => {
            out.push_str(&format!("    raw := C.{}(handle)\n", field.getter_symbol));
            out.push_str("    if raw == nil {\n        return \"\"\n    }\n");
            out.push_str("    return C.GoString(raw)\n");
        }
        _ => out.push_str(&format!(
            "    return {}(C.{}(handle))\n",
            field.go_type, field.getter_symbol
        )),
    }
    out.push_str("}\n");
    out
}

fn render_model_setter(projection: &ModelProjection, field: &ModelProjectionField) -> String {
    let receiver = receiver_name(&projection.go_name);
    let mut out = String::new();
    out.push_str(&format!(
        "func ({} *{}) Set{}(value {}) {{\n",
        receiver, projection.go_name, field.go_name, field.go_type
    ));
    out.push_str(&format!(
        "    handle := require{}Handle({})\n",
        projection.go_name, receiver
    ));
    if field.go_type == "string" {
        out.push_str("    cValue := C.CString(value)\n");
        out.push_str("    defer C.free(unsafe.Pointer(cValue))\n");
        out.push_str(&format!("    C.{}(handle, cValue)\n", field.setter_symbol));
    } else {
        out.push_str(&format!(
            "    C.{}(handle, {})\n",
            field.setter_symbol,
            render_c_arg(field, "value")
        ));
    }
    out.push_str("}\n");
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

pub fn collect_known_model_projections(
    config: &Config,
    ir: &IrModule,
) -> Result<Vec<KnownModelProjection>> {
    Ok(build_all_model_projections(ir)?
        .into_iter()
        .map(|projection| KnownModelProjection {
            cpp_type: projection.cpp_type,
            handle_name: projection.handle_name,
            go_name: projection.go_name,
            output_header: config.raw_include_for_go(&config.output.header),
            constructor_symbol: projection.constructor_symbol,
            destructor_symbol: Some(projection.destructor_symbol),
            fields: projection
                .fields
                .into_iter()
                .map(|field| KnownModelField {
                    go_name: field.go_name,
                    go_type: field.go_type,
                    getter_symbol: field.getter_symbol,
                    setter_symbol: field.setter_symbol,
                    return_kind: field.return_kind,
                })
                .collect(),
        })
        .collect())
}

fn build_all_model_projections(ir: &IrModule) -> Result<Vec<ModelProjection>> {
    let constructors = ir
        .functions
        .iter()
        .filter(|function| function.kind == "constructor")
        .filter_map(|function| {
            function
                .owner_cpp_type
                .as_deref()
                .map(|owner| (owner.to_string(), function.name.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let destructors = ir
        .functions
        .iter()
        .filter(|function| function.kind == "destructor")
        .filter_map(|function| {
            function
                .owner_cpp_type
                .as_deref()
                .map(|owner| (owner.to_string(), function.name.clone()))
        })
        .collect::<BTreeMap<_, _>>();

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
        if let Some(projection) = build_model_projection(
            &owner,
            &class_methods,
            constructors.get(&owner),
            destructors.get(&owner),
        )? {
            projections.push(projection);
        }
    }
    Ok(projections)
}

fn build_model_projection(
    owner: &str,
    class_methods: &[&IrFunction],
    constructor_symbol: Option<&String>,
    destructor_symbol: Option<&String>,
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

        let getter_ty = go_type_for_ir(&function.returns).ok_or_else(|| {
            anyhow!(
                "getter `{}` on `{owner}` has unsupported Go projection type `{}`",
                function.cpp_name,
                function.returns.cpp_type
            )
        })?;
        let setter_param = setter.params.get(1).ok_or_else(|| {
            anyhow!(
                "setter `{}` on `{owner}` is missing its value parameter",
                setter.cpp_name
            )
        })?;
        let setter_ty = go_type_for_ir(&setter_param.ty).ok_or_else(|| {
            anyhow!(
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

        fields.push(ModelProjectionField {
            go_name: go_field_name(suffix),
            go_type: getter_ty,
            getter_symbol: function.name.clone(),
            setter_symbol: setter.name.clone(),
            return_kind: function.returns.kind.clone(),
        });
    }

    if fields.is_empty() {
        return Ok(None);
    }

    let constructor_symbol = constructor_symbol
        .ok_or_else(|| anyhow!("model projection `{owner}` is missing a constructor wrapper"))?;
    let destructor_symbol = destructor_symbol
        .ok_or_else(|| anyhow!("model projection `{owner}` is missing a destructor wrapper"))?;

    Ok(Some(ModelProjection {
        cpp_type: owner.to_string(),
        go_name: leaf_cpp_name(owner).to_string(),
        handle_name: format!("{}Handle", flatten_qualified_cpp_name(owner)),
        constructor_symbol: constructor_symbol.clone(),
        destructor_symbol: destructor_symbol.clone(),
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

fn render_c_arg(field: &ModelProjectionField, name: &str) -> String {
    match field.go_type.as_str() {
        "bool" => format!("C.bool({name})"),
        "float32" => format!("C.float({name})"),
        "float64" => format!("C.double({name})"),
        "int8" => format!("C.int8_t({name})"),
        "int16" => format!("C.int16_t({name})"),
        "int32" => format!("C.int32_t({name})"),
        "int64" => format!("C.int64_t({name})"),
        "uint8" => format!("C.uint8_t({name})"),
        "uint16" => format!("C.uint16_t({name})"),
        "uint32" => format!("C.uint32_t({name})"),
        "uint64" => format!("C.uint64_t({name})"),
        "uintptr" => format!("C.size_t({name})"),
        _ => format!("C.int({name})"),
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

fn receiver_name(value: &str) -> String {
    value
        .chars()
        .next()
        .map(|ch| ch.to_ascii_lowercase().to_string())
        .unwrap_or_else(|| "v".to_string())
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

fn flatten_qualified_cpp_name(value: &str) -> String {
    value.split("::").collect::<Vec<_>>().join("")
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
