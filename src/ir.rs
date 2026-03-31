use std::collections::{BTreeMap, BTreeSet};

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped_declarations: Vec<SkippedDeclaration>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkippedDeclaration {
    pub cpp_name: String,
    pub reason: String,
}

pub fn normalize(config: &Config, api: &ParsedApi) -> Result<IrModule> {
    let module = config.naming.prefix.clone();
    let mut opaque_types = Vec::new();
    let mut functions = Vec::new();
    let mut enums = Vec::new();
    let mut skipped_declarations = Vec::new();

    for class in &api.classes {
        let handle_name = format!("{}Handle", flatten_cpp_name(&class.namespace, &class.name));
        opaque_types.push(OpaqueType {
            name: handle_name.clone(),
            cpp_type: cpp_qualified(&class.namespace, &class.name),
        });
        functions.extend(normalize_class(
            config,
            class,
            &handle_name,
            &mut skipped_declarations,
        )?);
    }

    for function in &api.functions {
        if let Some(function) = normalize_function(config, function, &mut skipped_declarations)? {
            functions.push(function);
        }
    }

    for item in &api.enums {
        enums.push(normalize_enum(item));
    }

    collect_referenced_opaque_types(&mut opaque_types, &functions);

    assign_unique_function_symbols(&mut functions);
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
            notes: {
                let mut notes = vec![
                    "Parsed with clang AST and normalized into a conservative C ABI IR."
                        .to_string(),
                    "v1 intentionally rejects unsupported C++ constructs during type normalization."
                        .to_string(),
                ];
                if !skipped_declarations.is_empty() {
                    notes.push(
                        "Skipped declarations are recorded in support.skipped_declarations when v1 cannot safely express them in raw output.".to_string(),
                    );
                }
                notes
            },
            skipped_declarations,
        },
    })
}

fn collect_referenced_opaque_types(opaque_types: &mut Vec<OpaqueType>, functions: &[IrFunction]) {
    let mut known = opaque_types
        .iter()
        .map(|item| item.name.clone())
        .collect::<BTreeSet<_>>();

    for function in functions {
        for ty in
            std::iter::once(&function.returns).chain(function.params.iter().map(|param| &param.ty))
        {
            let Some(handle) = &ty.handle else {
                continue;
            };
            if known.contains(handle) {
                continue;
            }
            if !matches!(ty.kind.as_str(), "model_reference" | "model_pointer") {
                continue;
            }

            opaque_types.push(OpaqueType {
                name: handle.clone(),
                cpp_type: base_model_cpp_type(&ty.cpp_type),
            });
            known.insert(handle.clone());
        }
    }
}

fn assign_unique_function_symbols(functions: &mut [IrFunction]) {
    let mut by_symbol: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, function) in functions.iter().enumerate() {
        by_symbol
            .entry(function.name.clone())
            .or_default()
            .push(index);
    }

    for (base_name, indexes) in by_symbol {
        if indexes.len() < 2 {
            continue;
        }

        let mut assigned: BTreeMap<String, usize> = BTreeMap::new();
        for index in indexes {
            let suffix = overload_suffix(&functions[index]);
            let candidate = format!("{base_name}__{suffix}");
            let occurrence = assigned.entry(candidate.clone()).or_insert(0);
            *occurrence += 1;
            if *occurrence == 1 {
                functions[index].name = candidate;
            } else {
                functions[index].name = format!("{candidate}_{}", occurrence);
            }
        }
    }
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

    bail!("overload collision detected after suffix assignment: {message}")
}

fn normalize_class(
    config: &Config,
    class: &CppClass,
    handle_name: &str,
    skipped_declarations: &mut Vec<SkippedDeclaration>,
) -> Result<Vec<IrFunction>> {
    let mut functions = Vec::new();
    let qualified = cpp_qualified(&class.namespace, &class.name);

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
        let initial_len = functions.len();
        for constructor in &class.constructors {
            if let Some(function) = normalize_constructor(
                config,
                class,
                handle_name,
                constructor,
                skipped_declarations,
            )? {
                functions.push(function);
            }
        }
        if functions.len() == initial_len {
            bail!(
                "class {qualified} declares constructors, but none were eligible for wrapper generation; refusing to synthesize a default constructor"
            );
        }
    }

    functions.push(IrFunction {
        name: symbol_name(config, &class.namespace, &class.name, "delete"),
        kind: "destructor".to_string(),
        cpp_name: if class.has_destructor {
            format!("~{}", qualified)
        } else {
            qualified.clone()
        },
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

    for method in &class.methods {
        if let Some(function) =
            normalize_method(config, class, handle_name, method, skipped_declarations)?
        {
            functions.push(function);
        }
    }

    Ok(functions)
}

fn normalize_constructor(
    config: &Config,
    class: &CppClass,
    handle_name: &str,
    constructor: &CppConstructor,
    skipped_declarations: &mut Vec<SkippedDeclaration>,
) -> Result<Option<IrFunction>> {
    let qualified = cpp_qualified(&class.namespace, &class.name);
    if let Some(reason) = function_pointer_reason(None, &constructor.params) {
        skipped_declarations.push(SkippedDeclaration {
            cpp_name: qualified.clone(),
            reason,
        });
        return Ok(None);
    }
    Ok(Some(IrFunction {
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
            .map(|param| normalize_param(config, param))
            .collect::<Result<Vec<_>>>()?,
    }))
}

fn normalize_method(
    config: &Config,
    class: &CppClass,
    handle_name: &str,
    method: &CppMethod,
    skipped_declarations: &mut Vec<SkippedDeclaration>,
) -> Result<Option<IrFunction>> {
    let qualified = cpp_qualified(&class.namespace, &class.name);
    let cpp_name = format!("{}::{}", qualified, method.name);
    if let Some(reason) = function_pointer_reason(
        Some((
            &method.return_type,
            &method.return_canonical_type,
            method.return_is_function_pointer,
        )),
        &method.params,
    ) {
        skipped_declarations.push(SkippedDeclaration { cpp_name, reason });
        return Ok(None);
    }
    if let Some(reason) = raw_unsafe_by_value_reason(
        Some((&method.return_type, &method.return_canonical_type)),
        &method.params,
    ) {
        skipped_declarations.push(SkippedDeclaration { cpp_name, reason });
        return Ok(None);
    }
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
            .map(|param| normalize_param(config, param))
            .collect::<Result<Vec<_>>>()?,
    );
    Ok(Some(IrFunction {
        name: symbol_name(config, &class.namespace, &class.name, &method.name),
        kind: "method".to_string(),
        cpp_name,
        method_of: Some(handle_name.to_string()),
        owner_cpp_type: Some(qualified),
        is_const: Some(method.is_const),
        returns: normalize_type_with_canonical(
            config,
            &method.return_type,
            &method.return_canonical_type,
        )?,
        params,
    }))
}

fn normalize_function(
    config: &Config,
    function: &CppFunction,
    skipped_declarations: &mut Vec<SkippedDeclaration>,
) -> Result<Option<IrFunction>> {
    let cpp_name = cpp_qualified(&function.namespace, &function.name);
    if let Some(reason) = function_pointer_reason(
        Some((
            &function.return_type,
            &function.return_canonical_type,
            function.return_is_function_pointer,
        )),
        &function.params,
    ) {
        skipped_declarations.push(SkippedDeclaration { cpp_name, reason });
        return Ok(None);
    }
    if let Some(reason) = raw_unsafe_by_value_reason(
        Some((&function.return_type, &function.return_canonical_type)),
        &function.params,
    ) {
        skipped_declarations.push(SkippedDeclaration { cpp_name, reason });
        return Ok(None);
    }
    Ok(Some(IrFunction {
        name: symbol_name(config, &function.namespace, "", &function.name),
        kind: "function".to_string(),
        cpp_name,
        method_of: None,
        owner_cpp_type: None,
        is_const: None,
        returns: normalize_type_with_canonical(
            config,
            &function.return_type,
            &function.return_canonical_type,
        )?,
        params: function
            .params
            .iter()
            .map(|param| normalize_param(config, param))
            .collect::<Result<Vec<_>>>()?,
    }))
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

fn normalize_param(config: &Config, param: &CppParam) -> Result<IrParam> {
    Ok(IrParam {
        name: param.name.clone(),
        ty: normalize_type_with_canonical(config, &param.ty, &param.canonical_ty)?,
    })
}

fn function_pointer_reason(
    return_type: Option<(&str, &str, bool)>,
    params: &[CppParam],
) -> Option<String> {
    let mut issues = Vec::new();

    if let Some((display, canonical, is_function_pointer)) = return_type
        && is_function_pointer
    {
        issues.push(format!(
            "return type `{}` uses a function pointer",
            format_type_for_reason(display, canonical)
        ));
    }

    for param in params {
        if param.is_function_pointer {
            issues.push(format!(
                "parameter `{}` type `{}` uses a function pointer",
                param.name,
                format_type_for_reason(&param.ty, &param.canonical_ty)
            ));
        }
    }

    (!issues.is_empty()).then(|| issues.join("; "))
}

fn raw_unsafe_by_value_reason(
    return_type: Option<(&str, &str)>,
    params: &[CppParam],
) -> Option<String> {
    let mut issues = Vec::new();

    if let Some((display, canonical)) = return_type
        && is_raw_unsafe_by_value_type(display, canonical)
    {
        issues.push(format!(
            "return type `{}` uses a raw-unsafe by-value object type",
            format_type_for_reason(display, canonical)
        ));
    }

    for param in params {
        if is_raw_unsafe_by_value_type(&param.ty, &param.canonical_ty) {
            issues.push(format!(
                "parameter `{}` type `{}` uses a raw-unsafe by-value object type",
                param.name,
                format_type_for_reason(&param.ty, &param.canonical_ty)
            ));
        }
    }

    (!issues.is_empty()).then(|| issues.join("; "))
}

fn is_raw_unsafe_by_value_type(display: &str, canonical: &str) -> bool {
    let display = display.trim();
    let canonical = canonical.trim();

    if normalize_type(display).is_ok() {
        return false;
    }
    if !canonical.is_empty() && canonical != display && normalize_type(canonical).is_ok() {
        return false;
    }

    [display, canonical]
        .into_iter()
        .filter(|candidate| !candidate.is_empty())
        .any(is_raw_unsafe_by_value_object_candidate)
}

fn is_raw_unsafe_by_value_object_candidate(cpp_type: &str) -> bool {
    let trimmed = cpp_type.trim();
    if trimmed.is_empty() || trimmed == "void" || trimmed.ends_with('&') || trimmed.ends_with('*') {
        return false;
    }

    let base = base_model_cpp_type(trimmed);
    !base.is_empty()
        && !base.contains('<')
        && !base.starts_with("std::")
        && !is_supported_primitive(&base)
}

fn format_type_for_reason(display: &str, canonical: &str) -> String {
    if canonical.is_empty() || canonical == display {
        display.to_string()
    } else {
        format!("{display} ({canonical})")
    }
}

fn normalize_type_with_canonical(
    _config: &Config,
    cpp_type: &str,
    canonical_type: &str,
) -> Result<IrType> {
    let trimmed = cpp_type.trim();
    if let Ok(ty) = normalize_type(trimmed) {
        return Ok(ty);
    }

    let canonical_trimmed = canonical_type.trim();
    if canonical_trimmed != trimmed {
        if let Ok(mut ty) = normalize_type(canonical_trimmed) {
            ty.cpp_type = trimmed.to_string();
            return Ok(ty);
        }
        bail!("unsupported C++ type in v1: {trimmed} (canonical: {canonical_trimmed})");
    }

    bail!("unsupported C++ type in v1: {trimmed}");
}

fn normalize_type(cpp_type: &str) -> Result<IrType> {
    let trimmed = cpp_type.trim();
    match trimmed {
        "void" => Ok(primitive_type(trimmed)),
        "bool" | "int" | "short" | "long" | "long long" | "float" | "double" | "size_t"
        | "char" | "unsigned" | "unsigned int" | "unsigned short" | "unsigned long"
        | "unsigned long long" | "signed char" | "unsigned char" => Ok(primitive_type(trimmed)),
        "uint8" => Ok(alias_primitive_type(trimmed, "uint8_t")),
        "uint16" => Ok(alias_primitive_type(trimmed, "uint16_t")),
        "uint32" => Ok(alias_primitive_type(trimmed, "uint32_t")),
        "uint64" => Ok(alias_primitive_type(trimmed, "uint64_t")),
        "int8" => Ok(alias_primitive_type(trimmed, "int8_t")),
        "int16" => Ok(alias_primitive_type(trimmed, "int16_t")),
        "int32" => Ok(alias_primitive_type(trimmed, "int32_t")),
        "int64" => Ok(alias_primitive_type(trimmed, "int64_t")),
        "const char *" | "const char*" => Ok(IrType {
            kind: "c_string".to_string(),
            cpp_type: trimmed.to_string(),
            c_type: "const char*".to_string(),
            handle: None,
        }),
        "char *" | "char*" => Ok(IrType {
            kind: "c_string".to_string(),
            cpp_type: trimmed.to_string(),
            c_type: "char*".to_string(),
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
        _ if trimmed.ends_with('&') && raw_safe_model_handle_name(trimmed).is_some() => {
            let handle_name = raw_safe_model_handle_name(trimmed).unwrap();
            Ok(IrType {
                kind: "model_reference".to_string(),
                cpp_type: trimmed.to_string(),
                c_type: format!("{handle_name}*"),
                handle: Some(handle_name),
            })
        }
        _ if trimmed.ends_with('*') && raw_safe_model_handle_name(trimmed).is_some() => {
            let handle_name = raw_safe_model_handle_name(trimmed).unwrap();
            Ok(IrType {
                kind: "model_pointer".to_string(),
                cpp_type: trimmed.to_string(),
                c_type: format!("{handle_name}*"),
                handle: Some(handle_name),
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
        "bool"
            | "int"
            | "int8_t"
            | "int16_t"
            | "int32_t"
            | "int64_t"
            | "short"
            | "long"
            | "long long"
            | "float"
            | "double"
            | "size_t"
            | "uint8_t"
            | "uint16_t"
            | "uint32_t"
            | "uint64_t"
            | "char"
            | "const char"
            | "unsigned"
            | "unsigned int"
            | "unsigned short"
            | "unsigned long"
            | "unsigned long long"
            | "signed char"
            | "unsigned char"
    )
}

fn symbol_name(config: &Config, namespace: &[String], owner: &str, tail: &str) -> String {
    let mut parts = vec![config.naming.prefix.clone()];
    parts.extend(
        namespace
            .iter()
            .map(|item| format_symbol_part(config, item)),
    );
    if !owner.is_empty() {
        parts.push(format_symbol_part(config, owner));
    }
    parts.push(format_symbol_part(config, tail));
    parts.join("_")
}

fn overload_suffix(function: &IrFunction) -> String {
    let params = if function.method_of.is_some() {
        &function.params[1..]
    } else {
        &function.params[..]
    };

    let mut parts = if params.is_empty() {
        vec!["void".to_string()]
    } else {
        params
            .iter()
            .map(|param| type_signature_token(&param.ty))
            .collect::<Vec<_>>()
    };

    if function.kind == "method" {
        parts.push(
            if function.is_const == Some(true) {
                "const"
            } else {
                "mut"
            }
            .to_string(),
        );
    }

    parts.join("_")
}

fn type_signature_token(ty: &IrType) -> String {
    match ty.kind.as_str() {
        "primitive" | "void" => sanitize_symbol_token(&ty.cpp_type),
        "c_string" => {
            if ty.cpp_type.contains("const")
                || matches!(ty.cpp_type.as_str(), "NPCSTR" | "NPSTRC" | "NPCSTRC")
            {
                "c_str".to_string()
            } else {
                "mut_c_str".to_string()
            }
        }
        "string" => "string".to_string(),
        "pointer" => format!(
            "ptr_{}",
            sanitize_symbol_token(ty.cpp_type.trim_end_matches('*'))
        ),
        "reference" => format!(
            "ref_{}",
            sanitize_symbol_token(ty.cpp_type.trim_end_matches('&'))
        ),
        "opaque" => format!(
            "opaque_{}",
            sanitize_symbol_token(&base_model_cpp_type(&ty.cpp_type))
        ),
        "model_reference" => format!(
            "model_ref_{}",
            sanitize_symbol_token(&base_model_cpp_type(&ty.cpp_type))
        ),
        "model_pointer" => format!(
            "model_ptr_{}",
            sanitize_symbol_token(&base_model_cpp_type(&ty.cpp_type))
        ),
        _ => sanitize_symbol_token(&ty.cpp_type),
    }
}

fn sanitize_symbol_token(value: &str) -> String {
    let mut out = String::new();
    let mut last_was_underscore = false;

    for ch in value.chars() {
        let normalized = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else {
            None
        };

        match normalized {
            Some(ch) => {
                out.push(ch);
                last_was_underscore = false;
            }
            None if !last_was_underscore => {
                out.push('_');
                last_was_underscore = true;
            }
            None => {}
        }
    }

    out.trim_matches('_').to_string()
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

fn flatten_qualified_cpp_name(value: &str) -> String {
    value.split("::").collect::<Vec<_>>().join("")
}

fn base_model_cpp_type(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("const ")
        .trim_end_matches('&')
        .trim_end_matches('*')
        .trim()
        .to_string()
}

fn raw_safe_model_handle_name(cpp_type: &str) -> Option<String> {
    let base = base_model_cpp_type(cpp_type);
    if base.is_empty()
        || base.contains('<')
        || base.starts_with("std::")
        || is_supported_primitive(&base)
    {
        return None;
    }

    Some(format!("{}Handle", flatten_qualified_cpp_name(&base)))
}
