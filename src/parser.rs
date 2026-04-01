#![allow(non_upper_case_globals)]
use std::{
    collections::HashSet,
    ffi::{CStr, CString},
    os::raw::{c_int, c_uint, c_void},
    path::Path,
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
    let target_headers = config
        .input
        .headers
        .iter()
        .map(|path| normalize_source_path(path))
        .collect::<HashSet<_>>();
    let parse_entries = config.parse_entries();
    unsafe {
        let index = clang_createIndex(0, 0);
        if index.is_null() {
            bail!("failed to create libclang index");
        }

        for parse_entry in &parse_entries {
            compiler::ensure_parse_entry_exists(parse_entry)?;
            let args = compiler::collect_clang_args(config, parse_entry)?;
            let c_header = CString::new(parse_entry.to_string_lossy().to_string())?;
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
                    parse_entry.display(),
                    error
                );
            }

            let root = clang_getTranslationUnitCursor(translation_unit);
            for child in direct_children(root) {
                collect_entity(child, &[], &mut api, &target_headers)?;
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
                    parse_entry.display(),
                    diagnostics.join("\n")
                );
            }

            clang_disposeTranslationUnit(translation_unit);
        }

        clang_disposeIndex(index);
    }

    dedupe_parsed_api(&mut api);
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
    api: &mut ParsedApi,
    target_headers: &HashSet<String>,
) -> Result<()> {
    if !belongs_to_target_header(cursor, target_headers) || is_system_header(cursor) {
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
                collect_entity(child, &next_namespace, api, target_headers)?;
            }
        }
        CXCursor_ClassDecl | CXCursor_StructDecl => {
            if unsafe { clang_isCursorDefinition(cursor) } == 0 {
                return Ok(());
            }
            if cursor_spelling(cursor).is_some() {
                let parsed = parse_class(cursor, namespace.to_vec(), target_headers)?;
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
        CXCursor_EnumDecl => {
            if cursor_spelling(cursor).is_some() {
                api.enums.push(parse_enum(cursor, namespace.to_vec()));
            }
        }
        _ => {}
    }

    Ok(())
}

fn parse_class(
    cursor: CXCursor,
    namespace: Vec<String>,
    target_headers: &HashSet<String>,
) -> Result<CppClass> {
    let name = cursor_spelling(cursor)
        .ok_or_else(|| anyhow!("anonymous classes are unsupported in v1"))?;
    let is_struct = unsafe { clang_getCursorKind(cursor) == CXCursor_StructDecl };
    let mut methods = Vec::new();
    let mut constructors = Vec::new();
    let mut has_destructor = false;
    let mut has_declared_constructor = false;

    for child in direct_children(cursor) {
        if !belongs_to_target_header(child, target_headers) {
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

fn belongs_to_target_header(cursor: CXCursor, target_headers: &HashSet<String>) -> bool {
    cursor_source_path(cursor)
        .map(|path| target_headers.contains(&path))
        .unwrap_or(false)
}

fn is_system_header(cursor: CXCursor) -> bool {
    unsafe { clang_Location_isInSystemHeader(clang_getCursorLocation(cursor)) != 0 }
}

fn cursor_source_path(cursor: CXCursor) -> Option<String> {
    unsafe {
        let location = clang_getCursorLocation(cursor);
        let mut file = ptr::null_mut();
        clang_getExpansionLocation(
            location,
            &mut file,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        );
        if file.is_null() {
            return None;
        }
        let spelling = cxstring_to_string(clang_getFileName(file));
        if spelling.is_empty() {
            return None;
        }
        Some(normalize_source_path(Path::new(&spelling)))
    }
}

fn normalize_source_path(path: &Path) -> String {
    let normalized = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let value = normalized.display().to_string();
    if cfg!(windows) {
        value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
    } else {
        value
    }
}

fn dedupe_parsed_api(api: &mut ParsedApi) {
    let mut class_keys = HashSet::new();
    api.classes.retain(|class| {
        let key = format!("{}::{}", class.namespace.join("::"), class.name);
        class_keys.insert(key)
    });

    let mut function_keys = HashSet::new();
    api.functions.retain(|function| {
        let key = format!(
            "{}::{}({})->{}",
            function.namespace.join("::"),
            function.name,
            function
                .params
                .iter()
                .map(|param| param.canonical_ty.clone())
                .collect::<Vec<_>>()
                .join(","),
            function.return_canonical_type
        );
        function_keys.insert(key)
    });

    let mut enum_keys = HashSet::new();
    api.enums.retain(|cpp_enum| {
        let key = format!("{}::{}", cpp_enum.namespace.join("::"), cpp_enum.name);
        enum_keys.insert(key)
    });
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
