use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use serde::Serialize;

pub use crate::domain::kind::{FieldAccessKind, IrFunctionKind, IrTypeKind};

use crate::{
    config::Config,
    parser::{
        CppCallbackTypedef, CppClass, CppConstructor, CppEnum, CppField, CppFunction, CppMethod,
        CppParam, ParsedApi,
    },
    pipeline::context::PipelineContext,
};

#[derive(Debug, Clone, Serialize)]
pub struct IrModule {
    pub version: u32,
    pub module: String,
    pub source_headers: Vec<String>,
    pub opaque_types: Vec<OpaqueType>,
    pub functions: Vec<IrFunction>,
    pub enums: Vec<IrEnum>,
    pub callbacks: Vec<IrCallback>,
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
    pub kind: IrFunctionKind,
    pub cpp_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_cpp_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_const: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_accessor: Option<IrFieldAccessor>,
    pub returns: IrType,
    pub params: Vec<IrParam>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrFieldAccessor {
    pub field_name: String,
    pub access: FieldAccessKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrParam {
    pub name: String,
    pub ty: IrType,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrType {
    pub kind: IrTypeKind,
    pub cpp_type: String,
    pub c_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrCallback {
    pub name: String,
    pub cpp_name: String,
    pub returns: IrType,
    pub params: Vec<IrParam>,
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

pub fn normalize(ctx: &PipelineContext, api: &ParsedApi) -> Result<IrModule> {
    let config = &ctx.config;
    let module = config.naming.prefix.clone();
    let mut opaque_types = Vec::new();
    let mut functions = Vec::new();
    let mut enums = Vec::new();
    let mut callbacks = Vec::new();
    let mut skipped_declarations = Vec::new();
    let callback_names = callback_name_set(api);
    let abstract_types = api
        .classes
        .iter()
        .filter(|class| class.is_abstract)
        .map(|class| cpp_qualified(&class.namespace, &class.name))
        .collect::<BTreeSet<_>>();

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
            &abstract_types,
            &callback_names,
            &mut skipped_declarations,
        )?);
    }

    for function in &api.functions {
        if let Some(function) = normalize_function(
            config,
            function,
            &abstract_types,
            &callback_names,
            &mut skipped_declarations,
        )?
        {
            functions.push(function);
        }
    }

    for item in &api.enums {
        enums.push(normalize_enum(item));
    }
    for callback in &api.callbacks {
        callbacks.push(normalize_callback(config, callback, &callback_names)?);
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
        callbacks,
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
            if !matches!(
                ty.kind,
                IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue
            ) {
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
    abstract_types: &BTreeSet<String>,
    callback_names: &BTreeSet<String>,
    skipped_declarations: &mut Vec<SkippedDeclaration>,
) -> Result<Vec<IrFunction>> {
    let mut functions = Vec::new();
    let qualified = cpp_qualified(&class.namespace, &class.name);

    if class.is_abstract {
        skipped_declarations.push(SkippedDeclaration {
            cpp_name: qualified.clone(),
            reason: "abstract class: has pure virtual methods; constructor wrapper omitted"
                .to_string(),
        });
    } else if class.constructors.is_empty() {
        functions.push(IrFunction {
            name: symbol_name(config, &class.namespace, &class.name, "new"),
            kind: IrFunctionKind::Constructor,
            cpp_name: qualified.clone(),
            method_of: Some(handle_name.to_string()),
            owner_cpp_type: Some(qualified.clone()),
            is_const: None,
            field_accessor: None,
            returns: IrType {
                kind: IrTypeKind::Opaque,
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
                callback_names,
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
        kind: IrFunctionKind::Destructor,
        cpp_name: if class.has_destructor {
            format!("~{}", qualified)
        } else {
            qualified.clone()
        },
        method_of: Some(handle_name.to_string()),
        owner_cpp_type: Some(qualified.clone()),
        is_const: None,
        field_accessor: None,
        returns: primitive_type("void"),
        params: vec![IrParam {
            name: "self".to_string(),
            ty: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: format!("{}*", qualified),
                c_type: format!("{handle_name}*"),
                handle: Some(handle_name.to_string()),
            },
        }],
    });

    for method in &class.methods {
            if let Some(function) = normalize_method(
                config,
                class,
                handle_name,
                method,
                abstract_types,
                callback_names,
                skipped_declarations,
            )? {
                functions.push(function);
        }
    }

    if class.is_struct {
        functions.extend(normalize_struct_fields(
            config,
            class,
            handle_name,
            callback_names,
        )?);
    }

    Ok(functions)
}

fn normalize_struct_fields(
    config: &Config,
    class: &CppClass,
    handle_name: &str,
    callback_names: &BTreeSet<String>,
) -> Result<Vec<IrFunction>> {
    let qualified = cpp_qualified(&class.namespace, &class.name);
    let existing_methods = class
        .methods
        .iter()
        .map(|method| method.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut functions = Vec::new();

    for field in &class.fields {
        if field.is_function_pointer {
            continue;
        }

        let suffix = struct_field_accessor_suffix(&field.name);
        let getter_name = format!("Get{suffix}");
        if existing_methods.contains(getter_name.as_str()) {
            continue;
        }

        let Ok(field_ty) =
            normalize_type_with_canonical(config, &field.ty, &field.canonical_ty, callback_names)
        else {
            continue;
        };
        if field_ty.kind != IrTypeKind::Primitive
            && field_ty.kind != IrTypeKind::ModelValue
            && field_ty.kind != IrTypeKind::CString
            && field_ty.kind != IrTypeKind::FixedByteArray
            && field_ty.kind != IrTypeKind::FixedArray
            && field_ty.kind != IrTypeKind::FixedModelArray
        {
            continue;
        }

        functions.push(make_struct_field_getter(
            config,
            &class.namespace,
            &class.name,
            &qualified,
            handle_name,
            field,
            field_ty.clone(),
        ));

        let setter_name = format!("Set{suffix}");
        if existing_methods.contains(setter_name.as_str()) || field_is_read_only(field) {
            continue;
        }

        functions.push(make_struct_field_setter(
            config,
            &class.namespace,
            &class.name,
            &qualified,
            handle_name,
            field,
            field_ty,
        ));
    }

    Ok(functions)
}

fn make_struct_field_getter(
    config: &Config,
    namespace: &[String],
    owner_name: &str,
    qualified: &str,
    handle_name: &str,
    field: &CppField,
    returns: IrType,
) -> IrFunction {
    let method_name = format!("Get{}", struct_field_accessor_suffix(&field.name));
    IrFunction {
        name: symbol_name(config, namespace, owner_name, &method_name),
        kind: IrFunctionKind::Method,
        cpp_name: format!("{qualified}::{method_name}"),
        method_of: Some(handle_name.to_string()),
        owner_cpp_type: Some(qualified.to_string()),
        is_const: Some(true),
        field_accessor: Some(IrFieldAccessor {
            field_name: field.name.clone(),
            access: FieldAccessKind::Get,
        }),
        returns,
        params: vec![IrParam {
            name: "self".to_string(),
            ty: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: format!("const {}*", qualified),
                c_type: format!("const {handle_name}*"),
                handle: Some(handle_name.to_string()),
            },
        }],
    }
}

fn make_struct_field_setter(
    config: &Config,
    namespace: &[String],
    owner_name: &str,
    qualified: &str,
    handle_name: &str,
    field: &CppField,
    field_ty: IrType,
) -> IrFunction {
    let method_name = format!("Set{}", struct_field_accessor_suffix(&field.name));
    IrFunction {
        name: symbol_name(config, namespace, owner_name, &method_name),
        kind: IrFunctionKind::Method,
        cpp_name: format!("{qualified}::{method_name}"),
        method_of: Some(handle_name.to_string()),
        owner_cpp_type: Some(qualified.to_string()),
        is_const: Some(false),
        field_accessor: Some(IrFieldAccessor {
            field_name: field.name.clone(),
            access: FieldAccessKind::Set,
        }),
        returns: primitive_type("void"),
        params: vec![
            IrParam {
                name: "self".to_string(),
                ty: IrType {
                    kind: IrTypeKind::Opaque,
                    cpp_type: format!("{}*", qualified),
                    c_type: format!("{handle_name}*"),
                    handle: Some(handle_name.to_string()),
                },
            },
            IrParam {
                name: "value".to_string(),
                ty: field_ty,
            },
        ],
    }
}

fn field_is_read_only(field: &CppField) -> bool {
    let ty = field.ty.trim();
    let canonical = field.canonical_ty.trim();
    ty.starts_with("const ") || canonical.starts_with("const ")
}

fn struct_field_accessor_suffix(field_name: &str) -> String {
    field_name
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut out = first.to_uppercase().collect::<String>();
                    out.push_str(chars.as_str());
                    out
                }
                None => String::new(),
            }
        })
        .collect::<String>()
}

fn normalize_constructor(
    config: &Config,
    class: &CppClass,
    handle_name: &str,
    constructor: &CppConstructor,
    callback_names: &BTreeSet<String>,
    skipped_declarations: &mut Vec<SkippedDeclaration>,
) -> Result<Option<IrFunction>> {
    let qualified = cpp_qualified(&class.namespace, &class.name);
    if let Some(reason) = function_pointer_reason(None, &constructor.params, callback_names) {
        skipped_declarations.push(SkippedDeclaration {
            cpp_name: qualified.clone(),
            reason,
        });
        return Ok(None);
    }
    Ok(Some(IrFunction {
        name: symbol_name(config, &class.namespace, &class.name, "new"),
        kind: IrFunctionKind::Constructor,
        cpp_name: qualified.clone(),
        method_of: Some(handle_name.to_string()),
        owner_cpp_type: Some(qualified.clone()),
        is_const: None,
        field_accessor: None,
        returns: IrType {
            kind: IrTypeKind::Opaque,
            cpp_type: qualified.clone(),
            c_type: format!("{handle_name}*"),
            handle: Some(handle_name.to_string()),
        },
        params: constructor
            .params
            .iter()
            .map(|param| normalize_param(config, param, callback_names))
            .collect::<Result<Vec<_>>>()?,
    }))
}

fn normalize_method(
    config: &Config,
    class: &CppClass,
    handle_name: &str,
    method: &CppMethod,
    abstract_types: &BTreeSet<String>,
    callback_names: &BTreeSet<String>,
    skipped_declarations: &mut Vec<SkippedDeclaration>,
) -> Result<Option<IrFunction>> {
    let qualified = cpp_qualified(&class.namespace, &class.name);
    let cpp_name = format!("{}::{}", qualified, method.name);
    if is_operator_name(&method.name) {
        skipped_declarations.push(SkippedDeclaration {
            cpp_name,
            reason: "operator declarations are unsupported in v1".to_string(),
        });
        return Ok(None);
    }
    if let Some(reason) = function_pointer_reason(
        Some((
            &method.return_type,
            &method.return_canonical_type,
            method.return_is_function_pointer,
        )),
        &method.params,
        callback_names,
    ) {
        skipped_declarations.push(SkippedDeclaration { cpp_name, reason });
        return Ok(None);
    }
    if let Some(reason) = raw_unsafe_by_value_reason(
        Some((&method.return_type, &method.return_canonical_type)),
        &method.params,
        callback_names,
    ) {
        skipped_declarations.push(SkippedDeclaration { cpp_name, reason });
        return Ok(None);
    }
    let mut params = Vec::new();
    params.push(IrParam {
        name: "self".to_string(),
        ty: IrType {
            kind: IrTypeKind::Opaque,
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
            .map(|param| normalize_param(config, param, callback_names))
            .collect::<Result<Vec<_>>>()?,
    );
    Ok(Some(IrFunction {
        name: symbol_name(config, &class.namespace, &class.name, &method.name),
        kind: IrFunctionKind::Method,
        cpp_name,
        method_of: Some(handle_name.to_string()),
        owner_cpp_type: Some(qualified),
        is_const: Some(method.is_const),
        field_accessor: None,
        returns: normalize_return_type_with_canonical(
            config,
            &method.return_type,
            &method.return_canonical_type,
            abstract_types,
            callback_names,
        )?,
        params,
    }))
}

fn normalize_function(
    config: &Config,
    function: &CppFunction,
    abstract_types: &BTreeSet<String>,
    callback_names: &BTreeSet<String>,
    skipped_declarations: &mut Vec<SkippedDeclaration>,
) -> Result<Option<IrFunction>> {
    let cpp_name = cpp_qualified(&function.namespace, &function.name);
    if is_operator_name(&function.name) {
        skipped_declarations.push(SkippedDeclaration {
            cpp_name,
            reason: "operator declarations are unsupported in v1".to_string(),
        });
        return Ok(None);
    }
    if let Some(reason) = function_pointer_reason(
        Some((
            &function.return_type,
            &function.return_canonical_type,
            function.return_is_function_pointer,
        )),
        &function.params,
        callback_names,
    ) {
        skipped_declarations.push(SkippedDeclaration { cpp_name, reason });
        return Ok(None);
    }
    if let Some(reason) = raw_unsafe_by_value_reason(
        Some((&function.return_type, &function.return_canonical_type)),
        &function.params,
        callback_names,
    ) {
        skipped_declarations.push(SkippedDeclaration { cpp_name, reason });
        return Ok(None);
    }
    Ok(Some(IrFunction {
        name: symbol_name(config, &function.namespace, "", &function.name),
        kind: IrFunctionKind::Function,
        cpp_name,
        method_of: None,
        owner_cpp_type: None,
        is_const: None,
        field_accessor: None,
        returns: normalize_return_type_with_canonical(
            config,
            &function.return_type,
            &function.return_canonical_type,
            abstract_types,
            callback_names,
        )?,
        params: function
            .params
            .iter()
            .map(|param| normalize_param(config, param, callback_names))
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

fn normalize_callback(
    config: &Config,
    callback: &CppCallbackTypedef,
    callback_names: &BTreeSet<String>,
) -> Result<IrCallback> {
    Ok(IrCallback {
        name: callback.name.clone(),
        cpp_name: cpp_qualified(&callback.namespace, &callback.name),
        returns: normalize_type_with_canonical(
            config,
            &callback.return_type,
            &callback.return_canonical_type,
            callback_names,
        )?,
        params: callback
            .params
            .iter()
            .map(|param| normalize_param(config, param, callback_names))
            .collect::<Result<Vec<_>>>()?,
    })
}

fn sanitize_go_param_name(name: &str) -> String {
    const GO_KEYWORDS: &[&str] = &[
        "break", "case", "chan", "const", "continue", "default", "defer", "else", "fallthrough",
        "for", "func", "go", "goto", "if", "import", "interface", "map", "package", "range",
        "return", "select", "struct", "switch", "type", "var",
    ];
    if GO_KEYWORDS.contains(&name) {
        format!("{name}_")
    } else {
        name.to_string()
    }
}

fn normalize_param(
    config: &Config,
    param: &CppParam,
    callback_names: &BTreeSet<String>,
) -> Result<IrParam> {
    Ok(IrParam {
        name: sanitize_go_param_name(&param.name),
        ty: normalize_type_with_canonical(config, &param.ty, &param.canonical_ty, callback_names)?,
    })
}

fn normalize_return_type_with_canonical(
    config: &Config,
    cpp_type: &str,
    canonical_type: &str,
    abstract_types: &BTreeSet<String>,
    callback_names: &BTreeSet<String>,
) -> Result<IrType> {
    let mut ty = normalize_type_with_canonical(config, cpp_type, canonical_type, callback_names)?;
    if matches!(
        ty.kind,
        IrTypeKind::ModelReference | IrTypeKind::ModelPointer
    ) && !is_abstract_model_type(&ty.cpp_type, abstract_types)
        && base_model_cpp_type(&ty.cpp_type) != "void"
    {
        ty.kind = IrTypeKind::ModelView;
    }
    Ok(ty)
}

fn is_abstract_model_type(cpp_type: &str, abstract_types: &BTreeSet<String>) -> bool {
    let base = base_model_cpp_type(cpp_type);
    !base.is_empty() && abstract_types.contains(&base)
}

fn function_pointer_reason(
    return_type: Option<(&str, &str, bool)>,
    params: &[CppParam],
    callback_names: &BTreeSet<String>,
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
        if param.is_function_pointer && !is_named_callback_param(param, callback_names) {
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
    callback_names: &BTreeSet<String>,
) -> Option<String> {
    let mut issues = Vec::new();

    if let Some((display, canonical)) = return_type
        && is_raw_unsafe_by_value_return_type(display, canonical, callback_names)
    {
        issues.push(format!(
            "return type `{}` uses a raw-unsafe by-value object type",
            format_type_for_reason(display, canonical)
        ));
    }

    for param in params {
        if is_raw_unsafe_by_value_param_type(&param.ty, &param.canonical_ty, callback_names) {
            issues.push(format!(
                "parameter `{}` type `{}` uses a raw-unsafe by-value object type",
                param.name,
                format_type_for_reason(&param.ty, &param.canonical_ty)
            ));
        }
    }

    (!issues.is_empty()).then(|| issues.join("; "))
}

fn is_raw_unsafe_by_value_return_type(
    display: &str,
    canonical: &str,
    callback_names: &BTreeSet<String>,
) -> bool {
    if normalize_type_with_canonical(&Config::default(), display, canonical, callback_names).is_ok()
    {
        return false;
    }
    if normalize_type(display, callback_names).is_ok() {
        return false;
    }
    if !canonical.trim().is_empty()
        && canonical.trim() != display.trim()
        && normalize_type(canonical, callback_names).is_ok()
    {
        return false;
    }

    is_raw_unsafe_by_value_object_candidate(display)
        || (!canonical.trim().is_empty()
            && canonical.trim() != display.trim()
            && is_raw_unsafe_by_value_object_candidate(canonical))
}

fn is_raw_unsafe_by_value_param_type(
    display: &str,
    canonical: &str,
    callback_names: &BTreeSet<String>,
) -> bool {
    let display = display.trim();
    let canonical = canonical.trim();

    if let Ok(ty) =
        normalize_type_with_canonical(&Config::default(), display, canonical, callback_names)
    {
        let _ = ty;
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
    callback_names: &BTreeSet<String>,
) -> Result<IrType> {
    let trimmed = cpp_type.trim();
    let canonical_trimmed = canonical_type.trim();
    if let Ok(ty) = normalize_type(trimmed, callback_names) {
        if matches!(
            ty.kind,
            IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue
        ) && canonical_trimmed != trimmed
        {
            if let Ok(mut canonical_ty) = normalize_type(canonical_trimmed, callback_names) {
                if matches!(
                    canonical_ty.kind,
                    IrTypeKind::ExternStructReference
                        | IrTypeKind::ExternStructPointer
                        | IrTypeKind::Primitive
                        | IrTypeKind::Reference
                        | IrTypeKind::Pointer
                        | IrTypeKind::String
                        | IrTypeKind::CString
                        | IrTypeKind::FixedByteArray
                ) {
                    canonical_ty.cpp_type = trimmed.to_string();
                    return Ok(canonical_ty);
                }
                // When both original and canonical resolve to a Model kind, use the
                // canonical type name for C++ code generation (e.g. `iKey_t` instead
                // of `iKey`) while keeping the original handle/c_type for the C API.
                if matches!(
                    canonical_ty.kind,
                    IrTypeKind::ModelValue
                        | IrTypeKind::ModelReference
                        | IrTypeKind::ModelPointer
                ) {
                    return Ok(IrType {
                        cpp_type: canonical_trimmed.to_string(),
                        ..ty
                    });
                }
            }
        }
        return Ok(ty);
    }

    if canonical_trimmed != trimmed {
        if raw_type_shape(trimmed) == raw_type_shape(canonical_trimmed)
            && let Ok(mut ty) = normalize_type(canonical_trimmed, callback_names)
        {
            ty.cpp_type = trimmed.to_string();
            return Ok(ty);
        }
        bail!("unsupported C++ type in v1: {trimmed} (canonical: {canonical_trimmed})");
    }

    bail!("unsupported C++ type in v1: {trimmed}");
}

fn raw_type_shape(cpp_type: &str) -> &'static str {
    let trimmed = cpp_type.trim().trim_start_matches("const ").trim();
    if trimmed.ends_with('*') {
        "pointer"
    } else if trimmed.ends_with('&') {
        "reference"
    } else {
        "value"
    }
}

fn normalize_type(cpp_type: &str, callback_names: &BTreeSet<String>) -> Result<IrType> {
    let trimmed = cpp_type.trim();
    if callback_names.contains(trimmed) {
        return Ok(IrType {
            kind: IrTypeKind::Callback,
            cpp_type: trimmed.to_string(),
            c_type: trimmed.to_string(),
            handle: None,
        });
    }

    // Strip leading "const " for value types and retry.
    // Only applies to non-pointer/reference types to preserve const semantics
    // on pointer targets (e.g. "const char*" is handled separately above).
    if let Some(stripped) = trimmed.strip_prefix("const ") {
        let stripped = stripped.trim();
        if !stripped.ends_with('*')
            && !stripped.ends_with('&')
            && let Ok(ty) = normalize_type(stripped, callback_names)
        {
            return Ok(IrType {
                cpp_type: trimmed.to_string(),
                c_type: ty.c_type.clone(),
                ..ty
            });
        }
    }

    if is_char_array_type(trimmed) {
        return Ok(IrType {
            kind: IrTypeKind::CString,
            cpp_type: trimmed.to_string(),
            c_type: "const char*".to_string(),
            handle: None,
        });
    }

    if is_unsigned_char_array_type(trimmed) {
        return Ok(IrType {
            kind: IrTypeKind::FixedByteArray,
            cpp_type: trimmed.to_string(),
            c_type: "uint8_t*".to_string(),
            handle: None,
        });
    }

    if let Some((elem, _)) = parse_array_type(trimmed) {
        if is_supported_primitive(elem) {
            let c_elem = canonical_primitive_c_type(elem);
            return Ok(IrType {
                kind: IrTypeKind::FixedArray,
                cpp_type: trimmed.to_string(),
                c_type: format!("{c_elem}*"),
                handle: None,
            });
        }
        if let Some(handle_name) = raw_safe_model_handle_name(elem) {
            return Ok(IrType {
                kind: IrTypeKind::FixedModelArray,
                cpp_type: trimmed.to_string(),
                c_type: format!("{handle_name}**"),
                handle: Some(handle_name),
            });
        }
    }

    match trimmed {
        "void" => Ok(primitive_type(trimmed)),
        "bool" | "int" | "short" | "long" | "long long" | "float" | "double" | "size_t"
        | "char" | "const char" | "unsigned" | "unsigned int" | "unsigned short"
        | "unsigned long" | "unsigned long long" | "signed char" | "unsigned char" => {
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
        "const char *" | "const char*" => Ok(IrType {
            kind: IrTypeKind::CString,
            cpp_type: trimmed.to_string(),
            c_type: "const char*".to_string(),
            handle: None,
        }),
        "char *" | "char*" => Ok(IrType {
            kind: IrTypeKind::CString,
            cpp_type: trimmed.to_string(),
            c_type: "char*".to_string(),
            handle: None,
        }),
        "NPCSTR" | "NPSTRC" | "NPCSTRC" => Ok(IrType {
            kind: IrTypeKind::CString,
            cpp_type: trimmed.to_string(),
            c_type: "const char*".to_string(),
            handle: None,
        }),
        "NPSTR" => Ok(IrType {
            kind: IrTypeKind::CString,
            cpp_type: trimmed.to_string(),
            c_type: "char*".to_string(),
            handle: None,
        }),
        "NPVOID" | "void *" | "void*" => Ok(IrType {
            kind: IrTypeKind::ModelPointer,
            cpp_type: "void".to_string(),
            c_type: "NPVOIDHandle*".to_string(),
            handle: Some("NPVOIDHandle".to_string()),
        }),
        "std::string" | "const std::string &" | "const std::string&" | "std::string_view" => {
            Ok(IrType {
                kind: IrTypeKind::String,
                cpp_type: trimmed.to_string(),
                c_type: "char*".to_string(),
                handle: None,
            })
        }
        _ if trimmed.ends_with('*')
            && is_supported_primitive(trimmed.trim_end_matches('*').trim()) =>
        {
            let base = trimmed.trim_end_matches('*').trim();
            Ok(IrType {
                kind: IrTypeKind::Pointer,
                cpp_type: trimmed.to_string(),
                c_type: format!("{}*", canonical_primitive_c_type(base)),
                handle: None,
            })
        }
        _ if trimmed.ends_with('&')
            && is_supported_primitive(trimmed.trim_end_matches('&').trim()) =>
        {
            let base = trimmed.trim_end_matches('&').trim();
            Ok(IrType {
                kind: IrTypeKind::Reference,
                cpp_type: trimmed.to_string(),
                c_type: format!("{}*", canonical_primitive_c_type(base)),
                handle: None,
            })
        }
        _ if trimmed.ends_with('&') && extern_c_struct_base_type(trimmed).is_some() => {
            let base = extern_c_struct_base_type(trimmed).unwrap();
            Ok(IrType {
                kind: IrTypeKind::ExternStructReference,
                cpp_type: trimmed.to_string(),
                c_type: format!("{base}*"),
                handle: None,
            })
        }
        _ if trimmed.ends_with('*') && extern_c_struct_base_type(trimmed).is_some() => {
            let base = extern_c_struct_base_type(trimmed).unwrap();
            Ok(IrType {
                kind: IrTypeKind::ExternStructPointer,
                cpp_type: trimmed.to_string(),
                c_type: format!("{base}*"),
                handle: None,
            })
        }
        _ if trimmed.ends_with('&') && raw_safe_model_handle_name(trimmed).is_some() => {
            let handle_name = raw_safe_model_handle_name(trimmed).unwrap();
            Ok(IrType {
                kind: IrTypeKind::ModelReference,
                cpp_type: trimmed.to_string(),
                c_type: format!("{handle_name}*"),
                handle: Some(handle_name),
            })
        }
        _ if trimmed.ends_with('*') && raw_safe_model_handle_name(trimmed).is_some() => {
            let handle_name = raw_safe_model_handle_name(trimmed).unwrap();
            Ok(IrType {
                kind: IrTypeKind::ModelPointer,
                cpp_type: trimmed.to_string(),
                c_type: format!("{handle_name}*"),
                handle: Some(handle_name),
            })
        }
        _ if raw_safe_model_handle_name(trimmed).is_some() => {
            let handle_name = raw_safe_model_handle_name(trimmed).unwrap();
            Ok(IrType {
                kind: IrTypeKind::ModelValue,
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
        kind: if name == "void" {
            IrTypeKind::Void
        } else {
            IrTypeKind::Primitive
        },
        cpp_type: name.to_string(),
        c_type: name.to_string(),
        handle: None,
    }
}

fn alias_primitive_type(cpp_name: &str, c_name: &str) -> IrType {
    IrType {
        kind: IrTypeKind::Primitive,
        cpp_type: cpp_name.to_string(),
        c_type: c_name.to_string(),
        handle: None,
    }
}

fn canonical_primitive_c_type(name: &str) -> &str {
    match name {
        "uint8" => "uint8_t",
        "uint16" => "uint16_t",
        "uint32" => "uint32_t",
        "uint64" => "uint64_t",
        "int8" => "int8_t",
        "int16" => "int16_t",
        "int32" => "int32_t",
        "int64" => "int64_t",
        other => other,
    }
}

fn callback_name_set(api: &ParsedApi) -> BTreeSet<String> {
    api.callbacks
        .iter()
        .flat_map(|callback| {
            let qualified = cpp_qualified(&callback.namespace, &callback.name);
            [callback.name.clone(), qualified]
        })
        .collect()
}

fn is_named_callback_param(param: &CppParam, callback_names: &BTreeSet<String>) -> bool {
    param
        .callback_typedef
        .as_deref()
        .is_some_and(|name| callback_names.contains(name))
}

fn is_operator_name(name: &str) -> bool {
    name.trim().starts_with("operator")
}

fn is_supported_primitive(name: &str) -> bool {
    matches!(
        name,
        "bool"
            | "int"
            | "int8_t"
            | "int8"
            | "int16_t"
            | "int16"
            | "int32_t"
            | "int32"
            | "int64_t"
            | "int64"
            | "short"
            | "long"
            | "long long"
            | "float"
            | "double"
            | "size_t"
            | "uint8_t"
            | "uint8"
            | "uint16_t"
            | "uint16"
            | "uint32_t"
            | "uint32"
            | "uint64_t"
            | "uint64"
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
    let params = if function.method_of.is_some()
        && matches!(
            function.kind,
            IrFunctionKind::Method | IrFunctionKind::Destructor
        ) {
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

    if function.kind == IrFunctionKind::Method {
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
    match ty.kind {
        IrTypeKind::Primitive | IrTypeKind::Void => sanitize_symbol_token(&ty.cpp_type),
        IrTypeKind::CString => {
            if ty.cpp_type.contains("const")
                || matches!(ty.cpp_type.as_str(), "NPCSTR" | "NPSTRC" | "NPCSTRC")
            {
                "c_str".to_string()
            } else {
                "mut_c_str".to_string()
            }
        }
        IrTypeKind::FixedByteArray => {
            let n = byte_array_length(&ty.cpp_type).unwrap_or(0);
            format!("byte_array_{n}")
        }
        IrTypeKind::String => "string".to_string(),
        IrTypeKind::Pointer => format!(
            "ptr_{}",
            sanitize_symbol_token(ty.cpp_type.trim_end_matches('*'))
        ),
        IrTypeKind::Reference => format!(
            "ref_{}",
            sanitize_symbol_token(ty.cpp_type.trim_end_matches('&'))
        ),
        IrTypeKind::ExternStructPointer => format!(
            "extern_ptr_{}",
            sanitize_symbol_token(&base_model_cpp_type(&ty.c_type))
        ),
        IrTypeKind::ExternStructReference => format!(
            "extern_ref_{}",
            sanitize_symbol_token(&base_model_cpp_type(&ty.c_type))
        ),
        IrTypeKind::Opaque => format!(
            "opaque_{}",
            sanitize_symbol_token(&base_model_cpp_type(&ty.cpp_type))
        ),
        IrTypeKind::ModelReference => format!(
            "model_ref_{}",
            sanitize_symbol_token(&base_model_cpp_type(&ty.cpp_type))
        ),
        IrTypeKind::ModelPointer => format!(
            "model_ptr_{}",
            sanitize_symbol_token(&base_model_cpp_type(&ty.cpp_type))
        ),
        IrTypeKind::ModelView => format!(
            "model_view_{}",
            sanitize_symbol_token(&base_model_cpp_type(&ty.cpp_type))
        ),
        IrTypeKind::ModelValue => format!(
            "model_value_{}",
            sanitize_symbol_token(&base_model_cpp_type(&ty.cpp_type))
        ),
        IrTypeKind::Callback => format!("callback_{}", sanitize_symbol_token(&ty.cpp_type)),
        IrTypeKind::FixedArray => {
            let n = fixed_array_length(&ty.cpp_type).unwrap_or(0);
            let elem = fixed_array_elem_type(&ty.cpp_type).unwrap_or("unknown");
            format!("array_{n}_{}", sanitize_symbol_token(elem))
        }
        IrTypeKind::FixedModelArray => {
            let n = fixed_array_length(&ty.cpp_type).unwrap_or(0);
            let handle = ty.handle.as_deref().unwrap_or("unknown");
            format!("model_array_{n}_{}", sanitize_symbol_token(handle))
        }
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

pub fn flatten_cpp_name(namespace: &[String], leaf: &str) -> String {
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

fn extern_c_struct_base_type(cpp_type: &str) -> Option<String> {
    let base = base_model_cpp_type(cpp_type);
    if let Some(tag) = base.strip_prefix("struct ") {
        return (!tag.trim().is_empty()).then(|| format!("struct {}", tag.trim()));
    }
    match base.as_str() {
        "timeval" => Some("struct timeval".to_string()),
        _ => None,
    }
}

fn raw_safe_model_handle_name(cpp_type: &str) -> Option<String> {
    let base = base_model_cpp_type(cpp_type);
    if base.is_empty()
        || base == "void"
        || base.contains('<')
        || base.starts_with("std::")
        || base.starts_with("struct ")
        || base.contains('[')
        || base.contains(']')
        || base.contains('(')
        || base.contains(')')
        || is_supported_primitive(&base)
    {
        return None;
    }

    Some(format!("{}Handle", flatten_qualified_cpp_name(&base)))
}

/// "T[N]" 패턴에서 (elem_type_str, size) 추출. const 접두사 제거 후 처리.
fn parse_array_type(value: &str) -> Option<(&str, usize)> {
    let trimmed = value.trim().trim_start_matches("const ").trim();
    let bracket = trimmed.rfind('[')?;
    let elem = trimmed[..bracket].trim();
    let rest = trimmed[bracket + 1..].strip_suffix(']')?;
    let n: usize = rest.trim().parse().ok()?;
    if elem.is_empty() || n == 0 {
        return None;
    }
    Some((elem, n))
}

/// cpp_type에서 배열 크기 추출 (FixedArray / FixedModelArray용)
pub fn fixed_array_length(cpp_type: &str) -> Option<usize> {
    parse_array_type(cpp_type).map(|(_, n)| n)
}

/// cpp_type에서 원소 타입 문자열 추출 (FixedArray용)
pub fn fixed_array_elem_type(cpp_type: &str) -> Option<&str> {
    parse_array_type(cpp_type).map(|(t, _)| t)
}

fn is_char_array_type(value: &str) -> bool {
    let trimmed = value.trim();
    if let Some(inner) = trimmed.strip_prefix("const ") {
        return is_char_array_type(inner);
    }

    let Some(prefix) = trimmed.strip_prefix("char[") else {
        return false;
    };
    let Some(length) = prefix.strip_suffix(']') else {
        return false;
    };
    !length.is_empty() && length.chars().all(|ch| ch.is_ascii_digit())
}

fn is_unsigned_char_array_type(value: &str) -> bool {
    let trimmed = value.trim();
    if let Some(inner) = trimmed.strip_prefix("const ") {
        return is_unsigned_char_array_type(inner);
    }
    let Some(prefix) = trimmed.strip_prefix("unsigned char[") else {
        return false;
    };
    let Some(length) = prefix.strip_suffix(']') else {
        return false;
    };
    !length.is_empty() && length.chars().all(|ch| ch.is_ascii_digit())
}

pub fn byte_array_length(cpp_type: &str) -> Option<usize> {
    let trimmed = cpp_type.trim().trim_start_matches("const ").trim();
    let prefix = trimmed.strip_prefix("unsigned char[")?;
    let len = prefix.strip_suffix(']')?;
    len.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalizes_struct_timeval_pointer_and_reference_as_external_structs() {
        let callback_names = BTreeSet::new();

        let pointer = normalize_type("struct timeval*", &callback_names).unwrap();
        assert_eq!(pointer.kind, IrTypeKind::ExternStructPointer);
        assert_eq!(pointer.c_type, "struct timeval*");
        assert_eq!(pointer.handle, None);

        let reference = normalize_type("struct timeval&", &callback_names).unwrap();
        assert_eq!(reference.kind, IrTypeKind::ExternStructReference);
        assert_eq!(reference.c_type, "struct timeval*");
        assert_eq!(reference.handle, None);
    }

    #[test]
    fn normalizes_timeval_alias_from_canonical_type() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type_with_canonical(
            &Config::default(),
            "timeval*",
            "struct timeval*",
            &callback_names,
        )
        .unwrap();

        assert_eq!(ty.kind, IrTypeKind::ExternStructPointer);
        assert_eq!(ty.cpp_type, "timeval*");
        assert_eq!(ty.c_type, "struct timeval*");
        assert_eq!(ty.handle, None);
    }

    #[test]
    fn rejects_by_value_type_when_only_canonical_form_is_pointer() {
        let callback_names = BTreeSet::new();
        let result =
            normalize_type_with_canonical(&Config::default(), "MTime", "MTime*", &callback_names)
                .unwrap();

        assert_eq!(result.kind, IrTypeKind::ModelValue);
        assert_eq!(result.c_type, "MTimeHandle*");
    }

    #[test]
    fn rejects_by_value_type_when_only_canonical_form_is_reference() {
        let callback_names = BTreeSet::new();
        let result = normalize_type_with_canonical(
            &Config::default(),
            "TD_IE_CALL",
            "TD_IE_CALL&",
            &callback_names,
        )
        .unwrap();

        assert_eq!(result.kind, IrTypeKind::ModelValue);
        assert_eq!(result.c_type, "TD_IE_CALLHandle*");
    }

    #[test]
    fn by_value_model_params_are_supported() {
        let callback_names = BTreeSet::new();
        assert!(!is_raw_unsafe_by_value_param_type(
            "MTime",
            "MTime",
            &callback_names
        ));
    }

    #[test]
    fn normalizes_model_pointer_returns_as_model_view() {
        let callback_names = BTreeSet::new();
        let ty = normalize_return_type_with_canonical(
            &Config::default(),
            "ThingModel*",
            "ThingModel*",
            &BTreeSet::new(),
            &callback_names,
        )
        .unwrap();

        assert_eq!(ty.kind, IrTypeKind::ModelView);
        assert_eq!(ty.c_type, "ThingModelHandle*");
    }

    #[test]
    fn keeps_abstract_model_pointer_returns_as_model_pointer() {
        let callback_names = BTreeSet::new();
        let abstract_types = BTreeSet::from([String::from("DBHandler")]);
        let ty = normalize_return_type_with_canonical(
            &Config::default(),
            "DBHandler*",
            "DBHandler*",
            &abstract_types,
            &callback_names,
        )
        .unwrap();

        assert_eq!(ty.kind, IrTypeKind::ModelPointer);
        assert_eq!(ty.c_type, "DBHandlerHandle*");
    }

    #[test]
    fn normalizes_char_array_as_c_string() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type("char[33]", &callback_names).unwrap();

        assert_eq!(ty.kind, IrTypeKind::CString);
        assert_eq!(ty.cpp_type, "char[33]");
        assert_eq!(ty.c_type, "const char*");
        assert_eq!(ty.handle, None);
    }

    #[test]
    fn array_types_are_not_promoted_to_model_handles() {
        assert_eq!(raw_safe_model_handle_name("char[33]"), None);
        assert_eq!(raw_safe_model_handle_name("uint32[8]"), None);
    }

    #[test]
    fn normalizes_unsigned_char_array_as_fixed_byte_array() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type("unsigned char[16]", &callback_names).unwrap();
        assert_eq!(ty.kind, IrTypeKind::FixedByteArray);
        assert_eq!(ty.cpp_type, "unsigned char[16]");
        assert_eq!(ty.c_type, "uint8_t*");
        assert_eq!(ty.handle, None);
    }

    #[test]
    fn normalizes_const_unsigned_char_array_as_fixed_byte_array() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type("const unsigned char[32]", &callback_names).unwrap();
        assert_eq!(ty.kind, IrTypeKind::FixedByteArray);
        assert_eq!(ty.cpp_type, "const unsigned char[32]");
        assert_eq!(ty.c_type, "uint8_t*");
        assert_eq!(ty.handle, None);
    }

    #[test]
    fn normalizes_uuid_t_alias_via_canonical_as_fixed_byte_array() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type_with_canonical(
            &Config::default(),
            "uuid_t",
            "unsigned char[16]",
            &callback_names,
        )
        .unwrap();
        assert_eq!(ty.kind, IrTypeKind::FixedByteArray);
        assert_eq!(ty.cpp_type, "uuid_t");
        assert_eq!(ty.c_type, "uint8_t*");
    }

    #[test]
    fn byte_array_length_extracts_size() {
        assert_eq!(byte_array_length("unsigned char[16]"), Some(16));
        assert_eq!(byte_array_length("const unsigned char[32]"), Some(32));
        assert_eq!(byte_array_length("unsigned char[1]"), Some(1));
        assert_eq!(byte_array_length("char[16]"), None);
        assert_eq!(byte_array_length("unsigned char*"), None);
        assert_eq!(byte_array_length("unsigned char"), None);
    }

    #[test]
    fn serializes_kind_enums_with_legacy_string_values() {
        let ty = IrType {
            kind: IrTypeKind::ModelValue,
            cpp_type: "ThingModel".to_string(),
            c_type: "ThingModelHandle*".to_string(),
            handle: Some("ThingModelHandle".to_string()),
        };
        let function = IrFunction {
            name: "cgowrap_ThingModel_new".to_string(),
            kind: IrFunctionKind::Constructor,
            cpp_name: "ThingModel".to_string(),
            method_of: None,
            owner_cpp_type: Some("ThingModel".to_string()),
            is_const: None,
            field_accessor: None,
            returns: ty.clone(),
            params: vec![],
        };

        let serialized_ty = serde_yaml::to_string(&ty).unwrap();
        let serialized_function = serde_yaml::to_string(&function).unwrap();

        assert!(serialized_ty.contains("kind: model_value"));
        assert!(serialized_function.contains("kind: constructor"));
    }

    #[test]
    fn normalizes_int_array_as_fixed_array() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type("int[4]", &callback_names).unwrap();
        assert_eq!(ty.kind, IrTypeKind::FixedArray);
        assert_eq!(ty.cpp_type, "int[4]");
        assert_eq!(ty.c_type, "int*");
        assert_eq!(ty.handle, None);
    }

    #[test]
    fn normalizes_bool_array_as_fixed_array() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type("bool[8]", &callback_names).unwrap();
        assert_eq!(ty.kind, IrTypeKind::FixedArray);
        assert_eq!(ty.cpp_type, "bool[8]");
        assert_eq!(ty.c_type, "bool*");
        assert_eq!(ty.handle, None);
    }

    #[test]
    fn normalizes_float_array_as_fixed_array() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type("float[3]", &callback_names).unwrap();
        assert_eq!(ty.kind, IrTypeKind::FixedArray);
        assert_eq!(ty.cpp_type, "float[3]");
        assert_eq!(ty.c_type, "float*");
        assert_eq!(ty.handle, None);
    }

    #[test]
    fn normalizes_uint32_t_array_as_fixed_array() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type("uint32_t[2]", &callback_names).unwrap();
        assert_eq!(ty.kind, IrTypeKind::FixedArray);
        assert_eq!(ty.cpp_type, "uint32_t[2]");
        assert_eq!(ty.c_type, "uint32_t*");
        assert_eq!(ty.handle, None);
    }

    #[test]
    fn normalizes_model_array_as_fixed_model_array() {
        let callback_names = BTreeSet::new();
        let ty = normalize_type("FooModel[3]", &callback_names).unwrap();
        assert_eq!(ty.kind, IrTypeKind::FixedModelArray);
        assert_eq!(ty.cpp_type, "FooModel[3]");
        assert_eq!(ty.c_type, "FooModelHandle**");
        assert_eq!(ty.handle, Some("FooModelHandle".to_string()));
    }

    #[test]
    fn fixed_array_length_extracts_size() {
        assert_eq!(fixed_array_length("int[4]"), Some(4));
        assert_eq!(fixed_array_length("bool[8]"), Some(8));
        assert_eq!(fixed_array_length("float[3]"), Some(3));
        assert_eq!(fixed_array_length("FooModel[3]"), Some(3));
        assert_eq!(fixed_array_length("int"), None);
        assert_eq!(fixed_array_length("int*"), None);
    }

    #[test]
    fn fixed_array_elem_type_extracts_elem() {
        assert_eq!(fixed_array_elem_type("int[4]"), Some("int"));
        assert_eq!(fixed_array_elem_type("bool[8]"), Some("bool"));
        assert_eq!(fixed_array_elem_type("float[3]"), Some("float"));
        assert_eq!(fixed_array_elem_type("int"), None);
    }
}
