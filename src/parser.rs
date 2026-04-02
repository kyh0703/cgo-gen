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
    pub callbacks: Vec<CppCallbackTypedef>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppClass {
    pub source_header: PathBuf,
    pub namespace: Vec<String>,
    pub name: String,
    pub methods: Vec<CppMethod>,
    pub constructors: Vec<CppConstructor>,
    pub has_destructor: bool,
    pub has_declared_constructor: bool,
    pub is_abstract: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppFunction {
    pub source_header: PathBuf,
    pub namespace: Vec<String>,
    pub name: String,
    pub return_type: String,
    pub return_canonical_type: String,
    pub return_is_function_pointer: bool,
    pub params: Vec<CppParam>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppMethod {
    pub name: String,
    pub return_type: String,
    pub return_canonical_type: String,
    pub return_is_function_pointer: bool,
    pub params: Vec<CppParam>,
    pub is_const: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppConstructor {
    pub params: Vec<CppParam>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppParam {
    pub name: String,
    pub ty: String,
    pub canonical_ty: String,
    pub is_function_pointer: bool,
    pub callback_typedef: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppCallbackTypedef {
    pub source_header: PathBuf,
    pub namespace: Vec<String>,
    pub name: String,
    pub return_type: String,
    pub return_canonical_type: String,
    pub params: Vec<CppParam>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppEnum {
    pub source_header: PathBuf,
    pub namespace: Vec<String>,
    pub name: String,
    pub variants: Vec<CppEnumVariant>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppEnumVariant {
    pub name: String,
    pub value: Option<String>,
}

impl ParsedApi {
    pub fn filter_to_header(&self, header: &Path) -> Self {
        let mut filtered = Self::default();
        filtered.headers = vec![header.display().to_string()];
        filtered.functions = self
            .functions
            .iter()
            .filter(|function| same_path(&function.source_header, header))
            .cloned()
            .collect();
        filtered.classes = self
            .classes
            .iter()
            .filter(|class| same_path(&class.source_header, header))
            .cloned()
            .collect();
        filtered.enums = self
            .enums
            .iter()
            .filter(|item| same_path(&item.source_header, header))
            .cloned()
            .collect();
        filtered.callbacks = self
            .callbacks
            .iter()
            .filter(|item| same_path(&item.source_header, header))
            .cloned()
            .collect();
        filtered
    }
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
    dedupe_api(&mut api);
    Ok(api)
}

fn dedupe_api(api: &mut ParsedApi) {
    api.functions = api
        .functions
        .clone()
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    api.classes = api
        .classes
        .clone()
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    api.enums = api
        .enums
        .clone()
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    api.callbacks = api
        .callbacks
        .clone()
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
}

#[derive(Debug, Clone)]
struct ParseFilter {
    main_file_only: bool,
    owned_dir: Option<PathBuf>,
    target_header: Option<PathBuf>,
}

impl ParseFilter {
    fn from_config(config: &Config) -> Self {
        Self {
            main_file_only: config.input.dir.is_none(),
            owned_dir: config.input.dir.clone(),
            target_header: config.target_header.clone(),
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
            if let Some(callback) = parse_callback_typedef(cursor, namespace.to_vec(), name.clone())?
            {
                api.callbacks.push(callback);
                return Ok(());
            }
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
    let source_header = normalized_cursor_file_path(cursor)
        .ok_or_else(|| anyhow!("failed to determine source header for class `{name}`"))?;
    let is_struct = unsafe { clang_getCursorKind(cursor) == CXCursor_StructDecl };
    let mut methods = Vec::new();
    let mut constructors = Vec::new();
    let mut has_destructor = false;
    let mut has_declared_constructor = false;
    let mut is_abstract = false;

    for child in direct_children(cursor) {
        if !should_collect_cursor(child, filter) {
            continue;
        }
        record_header_path(child, discovered_headers);
        let accessible = matches!(unsafe { clang_getCXXAccessSpecifier(child) }, CX_CXXPublic)
            || (is_struct
                && unsafe { clang_getCXXAccessSpecifier(child) } == CX_CXXInvalidAccessSpecifier);

        match unsafe { clang_getCursorKind(child) } {
            CXCursor_CXXMethod if unsafe { clang_CXXMethod_isPureVirtual(child) != 0 } => {
                is_abstract = true;
            }
            _ => {}
        }

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
        source_header,
        namespace,
        name,
        methods,
        constructors,
        has_destructor,
        has_declared_constructor,
        is_abstract,
    })
}

fn parse_function(cursor: CXCursor, namespace: Vec<String>) -> Result<CppFunction> {
    let name = cursor_spelling(cursor)
        .ok_or_else(|| anyhow!("encountered unnamed function declaration"))?;
    let source_header = normalized_cursor_file_path(cursor)
        .ok_or_else(|| anyhow!("failed to determine source header for function `{name}`"))?;
    Ok(CppFunction {
        source_header,
        namespace,
        name,
        return_type: result_type_name(cursor),
        return_canonical_type: result_canonical_type_name(cursor),
        return_is_function_pointer: result_is_function_pointer(cursor),
        params: parse_params(cursor),
    })
}

fn parse_enum_with_name(cursor: CXCursor, namespace: Vec<String>, name: String) -> CppEnum {
    let source_header = normalized_cursor_file_path(cursor).unwrap_or_default();
    let variants = direct_children(cursor)
        .into_iter()
        .filter(|child| unsafe { clang_getCursorKind(*child) } == CXCursor_EnumConstantDecl)
        .map(|child| CppEnumVariant {
            name: cursor_spelling(child).unwrap_or_default(),
            value: Some(unsafe { clang_getEnumConstantDeclValue(child) }.to_string()),
        })
        .collect();

    CppEnum {
        source_header,
        namespace,
        name,
        variants,
    }
}

fn parse_callback_typedef(
    cursor: CXCursor,
    namespace: Vec<String>,
    name: String,
) -> Result<Option<CppCallbackTypedef>> {
    let underlying = unsafe { clang_getTypedefDeclUnderlyingType(cursor) };
    let function_type = callback_function_type(underlying);
    if function_type.kind == CXType_Invalid {
        return Ok(None);
    }

    let source_header = normalized_cursor_file_path(cursor)
        .ok_or_else(|| anyhow!("failed to determine source header for callback typedef `{name}`"))?;
    let return_type = canonicalize_type_name(&unsafe { type_spelling(clang_getResultType(function_type)) });
    let return_canonical_type = canonicalize_type_name(&unsafe {
        type_spelling(clang_getCanonicalType(clang_getResultType(function_type)))
    });

    let child_params = direct_children(cursor)
        .into_iter()
        .filter(|child| unsafe { clang_getCursorKind(*child) } == CXCursor_ParmDecl)
        .map(|arg| CppParam {
            name: cursor_spelling(arg).unwrap_or_else(|| "arg".to_string()),
            ty: canonicalize_type_name(&cursor_type_spelling(arg)),
            canonical_ty: canonicalize_type_name(&cursor_canonical_type_spelling(arg)),
            is_function_pointer: cursor_is_function_pointer(arg),
            callback_typedef: callback_typedef_name_from_type(unsafe { clang_getCursorType(arg) }),
        })
        .enumerate()
        .map(|(index, mut param)| {
            if param.name.is_empty() || param.name == "arg" {
                param.name = format!("arg{index}");
            }
            param
        })
        .collect::<Vec<_>>();

    let params = if !child_params.is_empty() {
        child_params
    } else {
        parse_callback_params_from_type(function_type)
    };

    Ok(Some(CppCallbackTypedef {
        source_header,
        namespace,
        name,
        return_type,
        return_canonical_type,
        params,
    }))
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
            callback_typedef: callback_typedef_name_from_type(unsafe { clang_getCursorType(arg) }),
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

fn parse_callback_params_from_type(function_type: CXType) -> Vec<CppParam> {
    let count = unsafe { clang_getNumArgTypes(function_type) };
    if count < 0 {
        return Vec::new();
    }

    (0..count)
        .map(|index| unsafe { clang_getArgType(function_type, index as c_uint) })
        .enumerate()
        .map(|(index, ty)| CppParam {
            name: format!("arg{index}"),
            ty: canonicalize_type_name(&unsafe { type_spelling(ty) }),
            canonical_ty: canonicalize_type_name(&unsafe {
                type_spelling(clang_getCanonicalType(ty))
            }),
            is_function_pointer: is_function_pointer_type(ty),
            callback_typedef: callback_typedef_name_from_type(ty),
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

fn callback_function_type(ty: CXType) -> CXType {
    let canonical = unsafe { clang_getCanonicalType(ty) };
    match canonical.kind {
        CXType_FunctionProto | CXType_FunctionNoProto => canonical,
        CXType_Pointer => {
            let pointee = unsafe { clang_getPointeeType(canonical) };
            if matches!(pointee.kind, CXType_FunctionProto | CXType_FunctionNoProto) {
                pointee
            } else {
                invalid_type()
            }
        }
        _ => invalid_type(),
    }
}

fn invalid_type() -> CXType {
    CXType {
        kind: CXType_Invalid,
        data: [ptr::null_mut(); 2],
    }
}

fn callback_typedef_name_from_type(ty: CXType) -> Option<String> {
    if !is_function_pointer_type(ty) {
        return None;
    }

    let declaration = unsafe { clang_getTypeDeclaration(ty) };
    if unsafe { clang_equalCursors(declaration, clang_getNullCursor()) } != 0 {
        return None;
    }
    if unsafe { clang_getCursorKind(declaration) } != CXCursor_TypedefDecl {
        return None;
    }

    cursor_spelling(declaration)
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
        if !is_main_file(cursor) {
            return false;
        }
        return matches_target_header(cursor, filter.target_header.as_ref());
    }
    let Some(path) = cursor_file_path(cursor) else {
        return false;
    };
    let Some(dir) = &filter.owned_dir else {
        return false;
    };
    path_is_within(&path, dir)
        && is_header_path(&path)
        && match filter.target_header.as_ref() {
            Some(target) => same_path(&path, target),
            None => true,
        }
}

fn matches_target_header(cursor: CXCursor, target_header: Option<&PathBuf>) -> bool {
    let Some(target_header) = target_header else {
        return true;
    };
    let Some(path) = cursor_file_path(cursor) else {
        return false;
    };
    same_path(&path, target_header)
}

fn same_path(path: &Path, target: &Path) -> bool {
    if path == target {
        return true;
    }
    match (path.canonicalize(), target.canonicalize()) {
        (Ok(path), Ok(target)) => path == target,
        _ => false,
    }
}

fn normalized_cursor_file_path(cursor: CXCursor) -> Option<PathBuf> {
    let path = cursor_file_path(cursor)?;
    Some(path.canonicalize().unwrap_or(path))
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
