#![allow(non_upper_case_globals)]
use std::{
    ffi::{CStr, CString},
    os::raw::{c_int, c_uint, c_void},
    ptr,
};

use anyhow::{Result, anyhow, bail};
use clang_sys::*;
use serde::Serialize;

use crate::{
    compiler,
    config::{Config, FilterConfig},
};

#[derive(Debug, Clone, Serialize, Default)]
pub struct ParsedApi {
    pub headers: Vec<String>,
    pub functions: Vec<CppFunction>,
    pub classes: Vec<CppClass>,
    pub enums: Vec<CppEnum>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CppClass {
    pub namespace: Vec<String>,
    pub name: String,
    pub methods: Vec<CppMethod>,
    pub constructors: Vec<CppConstructor>,
    pub has_destructor: bool,
    pub has_declared_constructor: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CppFunction {
    pub namespace: Vec<String>,
    pub name: String,
    pub return_type: String,
    pub params: Vec<CppParam>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CppMethod {
    pub name: String,
    pub return_type: String,
    pub params: Vec<CppParam>,
    pub is_const: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CppConstructor {
    pub params: Vec<CppParam>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CppParam {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CppEnum {
    pub namespace: Vec<String>,
    pub name: String,
    pub variants: Vec<CppEnumVariant>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CppEnumVariant {
    pub name: String,
    pub value: Option<String>,
}

pub fn parse(config: &Config) -> Result<ParsedApi> {
    let mut api = ParsedApi::default();
    unsafe {
        let index = clang_createIndex(0, 0);
        if index.is_null() {
            bail!("failed to create libclang index");
        }

        for header in &config.input.headers {
            compiler::ensure_header_exists(header)?;
            let args = compiler::collect_clang_args(config, header)?;
            let c_header = CString::new(header.to_string_lossy().to_string())?;
            let c_args = args
                .iter()
                .map(|arg| CString::new(arg.as_str()))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let mut arg_ptrs = c_args.iter().map(|arg| arg.as_ptr()).collect::<Vec<_>>();

            let flags = (CXTranslationUnit_DetailedPreprocessingRecord
                | CXTranslationUnit_SkipFunctionBodies) as c_int;
            let mut translation_unit = ptr::null_mut();
            let error = clang_parseTranslationUnit2(
                index,
                c_header.as_ptr(),
                arg_ptrs.as_mut_ptr(),
                arg_ptrs.len() as c_int,
                ptr::null_mut(),
                0,
                flags,
                &mut translation_unit,
            );

            if error != CXError_Success || translation_unit.is_null() {
                bail!(
                    "failed to parse {} with libclang (error code {})",
                    header.display(),
                    error
                );
            }

            let root = clang_getTranslationUnitCursor(translation_unit);
            for child in direct_children(root) {
                collect_entity(child, &[], config, &mut api)?;
            }

            let diagnostics = collect_diagnostics(translation_unit);
            if !diagnostics.is_empty() {
                clang_disposeTranslationUnit(translation_unit);
                bail!(
                    "libclang reported diagnostics while parsing {}:\n{}",
                    header.display(),
                    diagnostics.join("\n")
                );
            }

            clang_disposeTranslationUnit(translation_unit);
        }

        clang_disposeIndex(index);
    }

    api.headers = config
        .input
        .headers
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    Ok(api)
}

fn collect_entity(
    cursor: CXCursor,
    namespace: &[String],
    config: &Config,
    api: &mut ParsedApi,
) -> Result<()> {
    if !is_main_file(cursor) || is_system_header(cursor) {
        return Ok(());
    }

    match unsafe { clang_getCursorKind(cursor) } {
        CXCursor_Namespace => {
            let Some(name) = cursor_spelling(cursor) else {
                return Ok(());
            };
            let mut next_namespace = namespace.to_vec();
            next_namespace.push(name);
            for child in direct_children(cursor) {
                collect_entity(child, &next_namespace, config, api)?;
            }
        }
        CXCursor_ClassDecl | CXCursor_StructDecl => {
            if unsafe { clang_isCursorDefinition(cursor) } == 0 {
                return Ok(());
            }
            if let Some(name) = cursor_spelling(cursor) {
                let qualified = qualified_name(namespace, &name);
                if namespace_allowed(&config.filter, namespace)
                    && named_allowed(
                        &config.filter.classes,
                        &config.filter.exclude_classes,
                        &name,
                        &qualified,
                    )
                {
                    let parsed = parse_class(cursor, namespace.to_vec(), &config.filter)?;
                    if parsed.has_declared_constructor
                        || parsed.has_destructor
                        || !parsed.methods.is_empty()
                    {
                        api.classes.push(parsed);
                    }
                }
            }
        }
        CXCursor_FunctionDecl => {
            if let Some(name) = cursor_spelling(cursor) {
                let qualified = qualified_name(namespace, &name);
                if namespace_allowed(&config.filter, namespace)
                    && named_allowed(
                        &config.filter.functions,
                        &config.filter.exclude_functions,
                        &name,
                        &qualified,
                    )
                {
                    let parsed = parse_function(cursor, namespace.to_vec())?;
                    if signature_types_allowed(&config.filter, &parsed.return_type, &parsed.params)
                    {
                        api.functions.push(parsed);
                    }
                }
            }
        }
        CXCursor_EnumDecl => {
            if let Some(name) = cursor_spelling(cursor) {
                let qualified = qualified_name(namespace, &name);
                if namespace_allowed(&config.filter, namespace)
                    && named_allowed(
                        &config.filter.enums,
                        &config.filter.exclude_enums,
                        &name,
                        &qualified,
                    )
                {
                    api.enums.push(parse_enum(cursor, namespace.to_vec()));
                }
            }
        }
        _ => {}
    }

    Ok(())
}

fn parse_class(
    cursor: CXCursor,
    namespace: Vec<String>,
    filter: &FilterConfig,
) -> Result<CppClass> {
    let name = cursor_spelling(cursor)
        .ok_or_else(|| anyhow!("anonymous classes are unsupported in v1"))?;
    let qualified_class = qualified_name(&namespace, &name);
    let is_struct = unsafe { clang_getCursorKind(cursor) == CXCursor_StructDecl };
    let mut methods = Vec::new();
    let mut constructors = Vec::new();
    let mut has_destructor = false;
    let mut has_declared_constructor = false;

    for child in direct_children(cursor) {
        if !is_main_file(child) {
            continue;
        }
        let accessible = matches!(unsafe { clang_getCXXAccessSpecifier(child) }, CX_CXXPublic)
            || (is_struct
                && unsafe { clang_getCXXAccessSpecifier(child) } == CX_CXXInvalidAccessSpecifier);
        if !accessible {
            continue;
        }

        match unsafe { clang_getCursorKind(child) } {
            CXCursor_Constructor => {
                has_declared_constructor = true;
                let constructor = CppConstructor {
                    params: parse_params(child),
                };
                if signature_types_allowed(filter, "void", &constructor.params) {
                    constructors.push(constructor);
                }
            }
            CXCursor_Destructor => has_destructor = true,
            CXCursor_CXXMethod => {
                let method_name = cursor_spelling(child).unwrap_or_default();
                let qualified_method = format!("{qualified_class}::{method_name}");
                if !named_allowed(
                    &filter.methods,
                    &filter.exclude_methods,
                    &method_name,
                    &qualified_method,
                ) {
                    continue;
                }
                let method = CppMethod {
                    name: method_name,
                    return_type: result_type_name(child),
                    params: parse_params(child),
                    is_const: unsafe { clang_CXXMethod_isConst(child) != 0 },
                };
                if signature_types_allowed(filter, &method.return_type, &method.params) {
                    methods.push(method);
                }
            }
            _ => {}
        }
    }

    Ok(CppClass {
        namespace,
        name,
        methods,
        constructors,
        has_destructor,
        has_declared_constructor,
    })
}

fn parse_function(cursor: CXCursor, namespace: Vec<String>) -> Result<CppFunction> {
    Ok(CppFunction {
        namespace,
        name: cursor_spelling(cursor)
            .ok_or_else(|| anyhow!("encountered unnamed function declaration"))?,
        return_type: result_type_name(cursor),
        params: parse_params(cursor),
    })
}

fn parse_enum(cursor: CXCursor, namespace: Vec<String>) -> CppEnum {
    let variants = direct_children(cursor)
        .into_iter()
        .filter(|child| unsafe { clang_getCursorKind(*child) } == CXCursor_EnumConstantDecl)
        .map(|child| CppEnumVariant {
            name: cursor_spelling(child).unwrap_or_default(),
            value: Some(unsafe { clang_getEnumConstantDeclValue(child) }.to_string()),
        })
        .collect();

    CppEnum {
        namespace,
        name: cursor_spelling(cursor).unwrap_or_default(),
        variants,
    }
}

fn parse_params(cursor: CXCursor) -> Vec<CppParam> {
    let count = unsafe { clang_Cursor_getNumArguments(cursor) };
    if count < 0 {
        return Vec::new();
    }

    (0..count)
        .map(|index| unsafe { clang_Cursor_getArgument(cursor, index as c_uint) })
        .map(|arg| CppParam {
            name: cursor_spelling(arg).unwrap_or_else(|| "arg".to_string()),
            ty: canonicalize_type_name(&cursor_type_spelling(arg)),
        })
        .enumerate()
        .map(|(index, mut param)| {
            if param.name.is_empty() || param.name == "arg" {
                param.name = format!("arg{index}");
            }
            param
        })
        .collect()
}

fn result_type_name(cursor: CXCursor) -> String {
    canonicalize_type_name(&unsafe { type_spelling(clang_getCursorResultType(cursor)) })
}

fn cursor_type_spelling(cursor: CXCursor) -> String {
    unsafe { type_spelling(clang_getCursorType(cursor)) }
}

unsafe fn type_spelling(ty: CXType) -> String {
    unsafe { cxstring_to_string(clang_getTypeSpelling(ty)) }
}

fn direct_children(cursor: CXCursor) -> Vec<CXCursor> {
    let mut children = Vec::new();
    unsafe {
        clang_visitChildren(
            cursor,
            collect_child,
            &mut children as *mut Vec<CXCursor> as *mut c_void,
        );
    }
    children
}

extern "C" fn collect_child(
    cursor: CXCursor,
    _parent: CXCursor,
    data: CXClientData,
) -> CXChildVisitResult {
    let children = unsafe { &mut *(data as *mut Vec<CXCursor>) };
    children.push(cursor);
    CXChildVisit_Continue
}

fn collect_diagnostics(translation_unit: CXTranslationUnit) -> Vec<String> {
    let count = unsafe { clang_getNumDiagnostics(translation_unit) };
    let mut diagnostics = Vec::new();
    for index in 0..count {
        unsafe {
            let diagnostic = clang_getDiagnostic(translation_unit, index);
            let severity = clang_getDiagnosticSeverity(diagnostic);
            if severity >= CXDiagnostic_Error {
                let message = cxstring_to_string(clang_formatDiagnostic(
                    diagnostic,
                    clang_defaultDiagnosticDisplayOptions(),
                ));
                diagnostics.push(message);
            }
            clang_disposeDiagnostic(diagnostic);
        }
    }
    diagnostics
}

fn is_main_file(cursor: CXCursor) -> bool {
    unsafe { clang_Location_isFromMainFile(clang_getCursorLocation(cursor)) != 0 }
}

fn is_system_header(cursor: CXCursor) -> bool {
    unsafe { clang_Location_isInSystemHeader(clang_getCursorLocation(cursor)) != 0 }
}

fn cursor_spelling(cursor: CXCursor) -> Option<String> {
    let spelling = unsafe { cxstring_to_string(clang_getCursorSpelling(cursor)) };
    if spelling.is_empty() {
        None
    } else {
        Some(spelling)
    }
}

unsafe fn cxstring_to_string(raw: CXString) -> String {
    let value = unsafe { clang_getCString(raw) };
    let owned = if value.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned()
    };
    unsafe { clang_disposeString(raw) };
    owned
}

fn qualified_name(namespace: &[String], name: &str) -> String {
    if namespace.is_empty() {
        name.to_string()
    } else {
        format!("{}::{name}", namespace.join("::"))
    }
}

fn namespace_allowed(filter: &FilterConfig, namespace: &[String]) -> bool {
    let qualified = namespace.join("::");
    let simple = namespace.last().cloned().unwrap_or_default();
    named_allowed(
        &filter.namespaces,
        &filter.exclude_namespaces,
        &simple,
        &qualified,
    )
}

fn named_allowed(includes: &[String], excludes: &[String], simple: &str, qualified: &str) -> bool {
    let included = includes.is_empty()
        || includes
            .iter()
            .any(|item| selector_matches(item, simple, qualified));
    let excluded = excludes
        .iter()
        .any(|item| selector_matches(item, simple, qualified));
    included && !excluded
}

fn signature_types_allowed(filter: &FilterConfig, return_type: &str, params: &[CppParam]) -> bool {
    let mut referenced_types = Vec::new();
    if return_type != "void" {
        referenced_types.push(canonicalize_type_name(return_type));
    }
    referenced_types.extend(params.iter().map(|param| canonicalize_type_name(&param.ty)));

    if !filter.exclude_types.is_empty()
        && referenced_types.iter().any(|ty| {
            filter
                .exclude_types
                .iter()
                .any(|selector| type_selector_matches(selector, ty))
        })
    {
        return false;
    }

    filter.types.is_empty()
        || referenced_types.iter().any(|ty| {
            filter
                .types
                .iter()
                .any(|selector| type_selector_matches(selector, ty))
        })
}

fn selector_matches(selector: &str, simple: &str, qualified: &str) -> bool {
    let selector = selector.trim();
    if selector.is_empty() {
        return false;
    }
    if let Some(prefix) = selector.strip_suffix("::*") {
        return qualified == prefix || qualified.starts_with(&format!("{prefix}::"));
    }
    selector == simple || selector == qualified
}

fn type_selector_matches(selector: &str, ty: &str) -> bool {
    let selector = canonicalize_type_name(selector);
    let ty = canonicalize_type_name(ty);
    if let Some(prefix) = selector.strip_suffix("::*") {
        return ty == prefix || ty.starts_with(&format!("{prefix}::"));
    }

    selector == ty || base_type_name(&selector) == base_type_name(&ty)
}

fn base_type_name(value: &str) -> String {
    value
        .trim_start_matches("const ")
        .trim_end_matches('&')
        .trim_end_matches('*')
        .trim()
        .to_string()
}

fn canonicalize_type_name(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" &", "&")
        .replace("* ", "*")
        .replace(" *", "*")
        .replace("< ", "<")
        .replace(" >", ">")
        .trim()
        .to_string()
}
