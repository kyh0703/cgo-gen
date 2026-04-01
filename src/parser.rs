#![allow(non_upper_case_globals)]
use std::{
    collections::BTreeSet,
    ffi::{CStr, CString},
    os::raw::{c_int, c_uint, c_void},
    path::{Path, PathBuf},
    ptr,
};

use anyhow::{Result, anyhow, bail};
use clang_sys::*;
use serde::Serialize;

use crate::{compiler, config::Config};

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
    pub return_canonical_type: String,
    pub return_is_function_pointer: bool,
    pub params: Vec<CppParam>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CppMethod {
    pub name: String,
    pub return_type: String,
    pub return_canonical_type: String,
    pub return_is_function_pointer: bool,
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
    pub canonical_ty: String,
    pub is_function_pointer: bool,
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
    let filter = ParseFilter::from_config(config);
    let translation_units = compiler::collect_translation_units(config)?;
    let mut discovered_headers = BTreeSet::new();
    unsafe {
        let index = clang_createIndex(0, 0);
        if index.is_null() {
            bail!("failed to create libclang index");
        }

        for translation_unit_path in &translation_units {
            compiler::ensure_header_exists(translation_unit_path)?;
            let args = compiler::collect_clang_args(config, translation_unit_path)?;
            let c_header = CString::new(translation_unit_path.to_string_lossy().to_string())?;
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
                    translation_unit_path.display(),
                    error
                );
            }

            let root = clang_getTranslationUnitCursor(translation_unit);
            for child in direct_children(root) {
                collect_entity(child, &[], &filter, &mut discovered_headers, &mut api)?;
            }

            let diagnostics = collect_diagnostics(translation_unit);
            if !diagnostics.is_empty() {
                if config.input.allow_diagnostics {
                    clang_disposeTranslationUnit(translation_unit);
                    continue;
                }
                clang_disposeTranslationUnit(translation_unit);
                bail!(
                    "libclang reported diagnostics while parsing {}:\n{}",
                    translation_unit_path.display(),
                    diagnostics.join("\n")
                );
            }

            clang_disposeTranslationUnit(translation_unit);
        }

        clang_disposeIndex(index);
    }

    api.headers = if !config.input.headers.is_empty() {
        config
            .input
            .headers
            .iter()
            .map(|path| path.display().to_string())
            .collect()
    } else {
        discovered_headers.into_iter().collect()
    };
    Ok(api)
}

#[derive(Debug, Clone)]
struct ParseFilter {
    main_file_only: bool,
    owned_dir: Option<PathBuf>,
}

impl ParseFilter {
    fn from_config(config: &Config) -> Self {
        Self {
            main_file_only: config.input.dir.is_none(),
            owned_dir: config.input.dir.clone(),
        }
    }
}

fn collect_entity(
    cursor: CXCursor,
    namespace: &[String],
    filter: &ParseFilter,
    discovered_headers: &mut BTreeSet<String>,
    api: &mut ParsedApi,
) -> Result<()> {
    if !should_collect_cursor(cursor, filter) {
        return Ok(());
    }
    record_header_path(cursor, discovered_headers);

    match unsafe { clang_getCursorKind(cursor) } {
        CXCursor_Namespace => {
            let Some(name) = cursor_spelling(cursor) else {
                return Ok(());
            };
            let mut next_namespace = namespace.to_vec();
            next_namespace.push(name);
            for child in direct_children(cursor) {
                collect_entity(child, &next_namespace, filter, discovered_headers, api)?;
            }
        }
        CXCursor_ClassDecl | CXCursor_StructDecl => {
            if unsafe { clang_isCursorDefinition(cursor) } == 0 {
                return Ok(());
            }
            if cursor_spelling(cursor).is_some() {
                let parsed = parse_class(cursor, namespace.to_vec(), filter, discovered_headers)?;
                if parsed.has_declared_constructor
                    || parsed.has_destructor
                    || !parsed.methods.is_empty()
                {
                    api.classes.push(parsed);
                }
            }
        }
        CXCursor_FunctionDecl => {
            if cursor_spelling(cursor).is_some() {
                api.functions
                    .push(parse_function(cursor, namespace.to_vec())?);
            }
        }
        CXCursor_TypedefDecl => {
            let Some(name) = cursor_spelling(cursor) else {
                return Ok(());
            };
            let Some(enum_cursor) = direct_children(cursor)
                .into_iter()
                .find(|child| unsafe { clang_getCursorKind(*child) } == CXCursor_EnumDecl)
            else {
                return Ok(());
            };
            if enum_decl_name(enum_cursor).is_none() {
                api.enums
                    .push(parse_enum_with_name(enum_cursor, namespace.to_vec(), name));
            }
        }
        CXCursor_EnumDecl => {
            if let Some(name) = enum_decl_name(cursor) {
                api.enums
                    .push(parse_enum_with_name(cursor, namespace.to_vec(), name));
            }
        }
        _ => {}
    }

    Ok(())
}

fn parse_class(
    cursor: CXCursor,
    namespace: Vec<String>,
    filter: &ParseFilter,
    discovered_headers: &mut BTreeSet<String>,
) -> Result<CppClass> {
    let name = cursor_spelling(cursor)
        .ok_or_else(|| anyhow!("anonymous classes are unsupported in v1"))?;
    let is_struct = unsafe { clang_getCursorKind(cursor) == CXCursor_StructDecl };
    let mut methods = Vec::new();
    let mut constructors = Vec::new();
    let mut has_destructor = false;
    let mut has_declared_constructor = false;

    for child in direct_children(cursor) {
        if !should_collect_cursor(child, filter) {
            continue;
        }
        record_header_path(child, discovered_headers);
        let accessible = matches!(unsafe { clang_getCXXAccessSpecifier(child) }, CX_CXXPublic)
            || (is_struct
                && unsafe { clang_getCXXAccessSpecifier(child) } == CX_CXXInvalidAccessSpecifier);
        if !accessible {
            continue;
        }

        match unsafe { clang_getCursorKind(child) } {
            CXCursor_Constructor => {
                has_declared_constructor = true;
                constructors.push(CppConstructor {
                    params: parse_params(child),
                });
            }
            CXCursor_Destructor => has_destructor = true,
            CXCursor_CXXMethod => {
                methods.push(CppMethod {
                    name: cursor_spelling(child).unwrap_or_default(),
                    return_type: result_type_name(child),
                    return_canonical_type: result_canonical_type_name(child),
                    return_is_function_pointer: result_is_function_pointer(child),
                    params: parse_params(child),
                    is_const: unsafe { clang_CXXMethod_isConst(child) != 0 },
                });
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
        return_canonical_type: result_canonical_type_name(cursor),
        return_is_function_pointer: result_is_function_pointer(cursor),
        params: parse_params(cursor),
    })
}

fn parse_enum_with_name(cursor: CXCursor, namespace: Vec<String>, name: String) -> CppEnum {
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
        name,
        variants,
    }
}

fn enum_decl_name(cursor: CXCursor) -> Option<String> {
    cursor_spelling(cursor).filter(|name| !is_unnamed_enum_spelling(name))
}

fn is_unnamed_enum_spelling(name: &str) -> bool {
    name.starts_with("(unnamed enum at ")
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
            canonical_ty: canonicalize_type_name(&cursor_canonical_type_spelling(arg)),
            is_function_pointer: cursor_is_function_pointer(arg),
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

fn result_canonical_type_name(cursor: CXCursor) -> String {
    canonicalize_type_name(&unsafe {
        type_spelling(clang_getCanonicalType(clang_getCursorResultType(cursor)))
    })
}

fn result_is_function_pointer(cursor: CXCursor) -> bool {
    is_function_pointer_type(unsafe { clang_getCursorResultType(cursor) })
}

fn cursor_type_spelling(cursor: CXCursor) -> String {
    unsafe { type_spelling(clang_getCursorType(cursor)) }
}

fn cursor_canonical_type_spelling(cursor: CXCursor) -> String {
    unsafe { type_spelling(clang_getCanonicalType(clang_getCursorType(cursor))) }
}

fn cursor_is_function_pointer(cursor: CXCursor) -> bool {
    is_function_pointer_type(unsafe { clang_getCursorType(cursor) })
}

unsafe fn type_spelling(ty: CXType) -> String {
    unsafe { cxstring_to_string(clang_getTypeSpelling(ty)) }
}

fn is_function_pointer_type(ty: CXType) -> bool {
    let canonical = unsafe { clang_getCanonicalType(ty) };
    match canonical.kind {
        CXType_FunctionProto | CXType_FunctionNoProto => true,
        CXType_Pointer => {
            let pointee = unsafe { clang_getPointeeType(canonical) };
            matches!(pointee.kind, CXType_FunctionProto | CXType_FunctionNoProto)
        }
        _ => false,
    }
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

fn should_collect_cursor(cursor: CXCursor, filter: &ParseFilter) -> bool {
    if is_system_header(cursor) {
        return false;
    }
    if filter.main_file_only {
        return is_main_file(cursor);
    }
    let Some(path) = cursor_file_path(cursor) else {
        return false;
    };
    let Some(dir) = &filter.owned_dir else {
        return false;
    };
    path_is_within(&path, dir) && is_header_path(&path)
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

fn record_header_path(cursor: CXCursor, discovered_headers: &mut BTreeSet<String>) {
    let Some(path) = cursor_file_path(cursor) else {
        return;
    };
    if !is_header_path(&path) {
        return;
    }
    discovered_headers.insert(path.display().to_string());
}

fn cursor_file_path(cursor: CXCursor) -> Option<PathBuf> {
    unsafe {
        let location = clang_getCursorLocation(cursor);
        if clang_equalLocations(location, clang_getNullLocation()) != 0 {
            return None;
        }

        let mut file = ptr::null_mut();
        let mut line = 0;
        let mut column = 0;
        let mut offset = 0;
        clang_getExpansionLocation(location, &mut file, &mut line, &mut column, &mut offset);
        if file.is_null() {
            return None;
        }
        let raw = cxstring_to_string(clang_getFileName(file));
        if raw.is_empty() {
            None
        } else {
            Some(PathBuf::from(raw))
        }
    }
}

fn path_is_within(path: &Path, dir: &Path) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    path.starts_with(dir)
}

fn is_header_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("h" | "hh" | "hpp" | "hxx")
    )
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
