use std::collections::BTreeMap;

use anyhow::{Result, bail};
use serde::Serialize;

use crate::{
    config::Config,
    parser::{CppClass, CppConstructor, CppEnum, CppFunction, CppMethod, CppParam, ParsedApi},
};

#[derive(Debug, Clone, Serialize)]
pub struct IrModule {
    pub version: u32,
    pub module: String,
    pub source_headers: Vec<String>,
    pub opaque_types: Vec<OpaqueType>,
    pub functions: Vec<IrFunction>,
    pub enums: Vec<IrEnum>,
    pub support: SupportMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpaqueType {
    pub name: String,
    pub cpp_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrFunction {
    pub name: String,
    pub kind: String,
    pub cpp_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_cpp_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_const: Option<bool>,
    pub returns: IrType,
    pub params: Vec<IrParam>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrParam {
    pub name: String,
    pub ty: IrType,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrType {
    pub kind: String,
    pub cpp_type: String,
    pub c_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrEnum {
    pub name: String,
    pub cpp_name: String,
    pub variants: Vec<IrEnumVariant>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrEnumVariant {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SupportMetadata {
    pub parser_backend: String,
    pub notes: Vec<String>,
}

pub fn normalize(config: &Config, api: &ParsedApi) -> Result<IrModule> {
    let module = config.naming.prefix.clone();
    let mut opaque_types = Vec::new();
    let mut functions = Vec::new();
    let mut enums = Vec::new();

    for class in &api.classes {
        let handle_name = format!("{}Handle", flatten_cpp_name(&class.namespace, &class.name));
        opaque_types.push(OpaqueType {
            name: handle_name.clone(),
            cpp_type: cpp_qualified(&class.namespace, &class.name),
        });
        functions.extend(normalize_class(config, class, &handle_name)?);
    }

    for function in &api.functions {
        functions.push(normalize_function(config, function)?);
    }

    for item in &api.enums {
        enums.push(normalize_enum(item));
    }

    ensure_unique_function_symbols(&functions)?;

    Ok(IrModule {
        version: config.version.unwrap_or(1),
        module,
        source_headers: api.headers.clone(),
        opaque_types,
        functions,
        enums,
        support: SupportMetadata {
            parser_backend: "libclang".to_string(),
            notes: vec![
                "Parsed with clang AST and normalized into a conservative C ABI IR.".to_string(),
                "v1 intentionally rejects unsupported C++ constructs during type normalization."
                    .to_string(),
            ],
        },
    })
}

fn ensure_unique_function_symbols(functions: &[IrFunction]) -> Result<()> {
    let mut by_symbol: BTreeMap<&str, Vec<&IrFunction>> = BTreeMap::new();
    for function in functions {
        by_symbol
            .entry(function.name.as_str())
            .or_default()
            .push(function);
    }

    let duplicates = by_symbol
        .into_iter()
        .filter(|(_, items)| items.len() > 1)
        .collect::<Vec<_>>();

    if duplicates.is_empty() {
        return Ok(());
    }

    let message = duplicates
        .into_iter()
        .map(|(symbol, items)| {
            let origins = items
                .into_iter()
                .map(|item| item.cpp_name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            format!("wrapper symbol `{symbol}` collides for C++ declarations: {origins}")
        })
        .collect::<Vec<_>>()
        .join("; ");

    bail!(
        "overload collision detected; deterministic overload-safe naming is not implemented yet: {message}"
    )
}

fn normalize_class(
    config: &Config,
    class: &CppClass,
    handle_name: &str,
) -> Result<Vec<IrFunction>> {
    let mut functions = Vec::new();
    let qualified = cpp_qualified(&class.namespace, &class.name);

    if class.has_declared_constructor && class.constructors.is_empty() {
        bail!(
            "class {qualified} declares constructors, but none survived filtering/type normalization; refusing to synthesize a default constructor"
        );
    }

    if class.constructors.is_empty() {
        functions.push(IrFunction {
            name: symbol_name(config, &class.namespace, &class.name, "new"),
            kind: "constructor".to_string(),
            cpp_name: qualified.clone(),
            method_of: Some(handle_name.to_string()),
            owner_cpp_type: Some(qualified.clone()),
            is_const: None,
            returns: IrType {
                kind: "opaque".to_string(),
                cpp_type: qualified.clone(),
                c_type: format!("{handle_name}*"),
                handle: Some(handle_name.to_string()),
            },
            params: Vec::new(),
        });
    } else {
        for constructor in &class.constructors {
            functions.push(normalize_constructor(
                config,
                class,
                handle_name,
                constructor,
            )?);
        }
    }

    if class.has_destructor {
        functions.push(IrFunction {
            name: symbol_name(config, &class.namespace, &class.name, "delete"),
            kind: "destructor".to_string(),
            cpp_name: format!("~{}", qualified),
            method_of: Some(handle_name.to_string()),
            owner_cpp_type: Some(qualified.clone()),
            is_const: None,
            returns: primitive_type("void"),
            params: vec![IrParam {
                name: "self".to_string(),
                ty: IrType {
                    kind: "opaque".to_string(),
                    cpp_type: format!("{}*", qualified),
                    c_type: format!("{handle_name}*"),
                    handle: Some(handle_name.to_string()),
                },
            }],
        });
    }

    for method in &class.methods {
        functions.push(normalize_method(config, class, handle_name, method)?);
    }

    Ok(functions)
}

fn normalize_constructor(
    config: &Config,
    class: &CppClass,
    handle_name: &str,
    constructor: &CppConstructor,
) -> Result<IrFunction> {
    let qualified = cpp_qualified(&class.namespace, &class.name);
    Ok(IrFunction {
        name: symbol_name(config, &class.namespace, &class.name, "new"),
        kind: "constructor".to_string(),
        cpp_name: qualified.clone(),
        method_of: Some(handle_name.to_string()),
        owner_cpp_type: Some(qualified.clone()),
        is_const: None,
        returns: IrType {
            kind: "opaque".to_string(),
            cpp_type: qualified.clone(),
            c_type: format!("{handle_name}*"),
            handle: Some(handle_name.to_string()),
        },
        params: constructor
            .params
            .iter()
            .map(normalize_param)
            .collect::<Result<Vec<_>>>()?,
    })
}

fn normalize_method(
    config: &Config,
    class: &CppClass,
    handle_name: &str,
    method: &CppMethod,
) -> Result<IrFunction> {
    let qualified = cpp_qualified(&class.namespace, &class.name);
    let mut params = Vec::new();
    params.push(IrParam {
        name: "self".to_string(),
        ty: IrType {
            kind: "opaque".to_string(),
            cpp_type: if method.is_const {
                format!("const {}*", qualified)
            } else {
                format!("{}*", qualified)
            },
            c_type: if method.is_const {
                format!("const {handle_name}*")
            } else {
                format!("{handle_name}*")
            },
            handle: Some(handle_name.to_string()),
        },
    });
    params.extend(
        method
            .params
            .iter()
            .map(normalize_param)
            .collect::<Result<Vec<_>>>()?,
    );
    Ok(IrFunction {
        name: symbol_name(config, &class.namespace, &class.name, &method.name),
        kind: "method".to_string(),
        cpp_name: format!("{}::{}", qualified, method.name),
        method_of: Some(handle_name.to_string()),
        owner_cpp_type: Some(qualified),
        is_const: Some(method.is_const),
        returns: normalize_type(&method.return_type)?,
        params,
    })
}

fn normalize_function(config: &Config, function: &CppFunction) -> Result<IrFunction> {
    Ok(IrFunction {
        name: symbol_name(config, &function.namespace, "", &function.name),
        kind: "function".to_string(),
        cpp_name: cpp_qualified(&function.namespace, &function.name),
        method_of: None,
        owner_cpp_type: None,
        is_const: None,
        returns: normalize_type(&function.return_type)?,
        params: function
            .params
            .iter()
            .map(normalize_param)
            .collect::<Result<Vec<_>>>()?,
    })
}

fn normalize_enum(item: &CppEnum) -> IrEnum {
    IrEnum {
        name: flatten_cpp_name(&item.namespace, &item.name),
        cpp_name: cpp_qualified(&item.namespace, &item.name),
        variants: item
            .variants
            .iter()
            .map(|variant| IrEnumVariant {
                name: variant.name.clone(),
                value: variant.value.clone(),
            })
            .collect(),
    }
}

fn normalize_param(param: &CppParam) -> Result<IrParam> {
    Ok(IrParam {
        name: param.name.clone(),
        ty: normalize_type(&param.ty)?,
    })
}

fn normalize_type(cpp_type: &str) -> Result<IrType> {
    let trimmed = cpp_type.trim();
    match trimmed {
        "void" => Ok(primitive_type(trimmed)),
        "bool" | "int" | "short" | "long" | "float" | "double" | "size_t" | "char" => {
            Ok(primitive_type(trimmed))
        }
        "uint8" => Ok(alias_primitive_type(trimmed, "uint8_t")),
        "uint16" => Ok(alias_primitive_type(trimmed, "uint16_t")),
        "uint32" => Ok(alias_primitive_type(trimmed, "uint32_t")),
        "uint64" => Ok(alias_primitive_type(trimmed, "uint64_t")),
        "int8" => Ok(alias_primitive_type(trimmed, "int8_t")),
        "int16" => Ok(alias_primitive_type(trimmed, "int16_t")),
        "int32" => Ok(alias_primitive_type(trimmed, "int32_t")),
        "int64" => Ok(alias_primitive_type(trimmed, "int64_t")),
        "const char *" | "char *" | "const char*" | "char*" => Ok(IrType {
            kind: "c_string".to_string(),
            cpp_type: trimmed.to_string(),
            c_type: trimmed.replace(' ', ""),
            handle: None,
        }),
        "NPCSTR" | "NPSTRC" | "NPCSTRC" => Ok(IrType {
            kind: "c_string".to_string(),
            cpp_type: trimmed.to_string(),
            c_type: "const char*".to_string(),
            handle: None,
        }),
        "NPSTR" => Ok(IrType {
            kind: "c_string".to_string(),
            cpp_type: trimmed.to_string(),
            c_type: "char*".to_string(),
            handle: None,
        }),
        "std::string" | "const std::string &" | "const std::string&" | "std::string_view" => {
            Ok(IrType {
                kind: "string".to_string(),
                cpp_type: trimmed.to_string(),
                c_type: "char*".to_string(),
                handle: None,
            })
        }
        _ if trimmed.ends_with('*')
            && is_supported_primitive(trimmed.trim_end_matches('*').trim()) =>
        {
            Ok(IrType {
                kind: "pointer".to_string(),
                cpp_type: trimmed.to_string(),
                c_type: trimmed.to_string(),
                handle: None,
            })
        }
        _ if trimmed.ends_with('&')
            && is_supported_primitive(trimmed.trim_end_matches('&').trim()) =>
        {
            Ok(IrType {
                kind: "reference".to_string(),
                cpp_type: trimmed.to_string(),
                c_type: format!("{}*", trimmed.trim_end_matches('&').trim()),
                handle: None,
            })
        }
        _ => bail!("unsupported C++ type in v1: {trimmed}"),
    }
}

fn primitive_type(name: &str) -> IrType {
    IrType {
        kind: if name == "void" { "void" } else { "primitive" }.to_string(),
        cpp_type: name.to_string(),
        c_type: name.to_string(),
        handle: None,
    }
}

fn alias_primitive_type(cpp_name: &str, c_name: &str) -> IrType {
    IrType {
        kind: "primitive".to_string(),
        cpp_type: cpp_name.to_string(),
        c_type: c_name.to_string(),
        handle: None,
    }
}

fn is_supported_primitive(name: &str) -> bool {
    matches!(
        name,
        "bool" | "int" | "short" | "long" | "float" | "double" | "size_t" | "char" | "const char"
    )
}

fn symbol_name(config: &Config, namespace: &[String], owner: &str, tail: &str) -> String {
    let mut parts = vec![config.naming.prefix.clone()];
    parts.extend(namespace.iter().map(|item| format_symbol_part(config, item)));
    if !owner.is_empty() {
        parts.push(format_symbol_part(config, owner));
    }
    parts.push(format_symbol_part(config, tail));
    parts.join("_")
}

fn format_symbol_part(config: &Config, value: &str) -> String {
    match config.naming.style.as_str() {
        "preserve" => value.to_string(),
        _ => value.to_lowercase(),
    }
}

fn cpp_qualified(namespace: &[String], leaf: &str) -> String {
    if namespace.is_empty() {
        leaf.to_string()
    } else {
        format!("{}::{}", namespace.join("::"), leaf)
    }
}

fn flatten_cpp_name(namespace: &[String], leaf: &str) -> String {
    if namespace.is_empty() {
        leaf.to_string()
    } else {
        format!("{}{}", namespace.join(""), leaf)
    }
}
