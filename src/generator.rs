use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::{
    analysis::model_analysis,
    domain::kind::{FieldAccessKind, IrFunctionKind, IrTypeKind},
    facade,
    ir::{IrCallback, IrFunction, IrFunctionKind, IrModule, IrParam, IrType, IrTypeKind},
    parser,
    pipeline::context::{PipelineContext, PipelineInput},
};

pub fn generate_all<T: PipelineInput + ?Sized>(input: &T, write_ir: bool) -> Result<()> {
    let ctx = input.to_pipeline_context();
    let (ctx, parsed) = prepare_with_parsed(&ctx)?;
    let generation_headers = generation_headers(&ctx);

    if generation_headers.len() > 1 && !ctx.uses_default_output_names() {
        bail!(
            "multi-header generation does not support explicit output.header/source/ir overrides; leave them as defaults to emit one wrapper set per header"
        );
    }

    if generation_headers.len() <= 1 {
        let scoped = generation_headers
            .first()
            .cloned()
            .map(|header| ctx.scoped_to_header(header))
            .unwrap_or_else(|| ctx.clone());
        let header_api = scoped
            .target_header
            .as_deref()
            .map(|header| parsed.filter_to_header(header))
            .unwrap_or_else(|| parsed.clone());
        let ir = crate::ir::normalize(&scoped, &header_api)?;
        return generate(&scoped, &ir, write_ir);
    }

    for header in &generation_headers {
        let scoped = ctx.scoped_to_header(header.clone());
        let header_api = parsed.filter_to_header(header);
        if header_api.is_empty() {
            continue;
        }
        let ir = crate::ir::normalize(&scoped, &header_api)?;
        generate(&scoped, &ir, write_ir)?;
    }

    Ok(())
}

fn generation_headers(ctx: &PipelineContext) -> Vec<PathBuf> {
    if ctx.input.dir.is_some() {
        return scan_generation_headers(ctx.input.dir.as_ref().unwrap()).unwrap_or_default();
    }

    ctx.input.headers.clone()
}

fn scan_generation_headers(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut headers = BTreeSet::new();
    for entry in fs::read_dir(dir)
        .with_context(|| format!("failed to read generation directory: {}", dir.display()))?
    {
        let path = entry?.path();
        if path.is_file()
            && matches!(
                path.extension().and_then(|ext| ext.to_str()),
                Some("h" | "hh" | "hpp" | "hxx")
            )
        {
            headers.insert(path);
        }
    }
    Ok(headers.into_iter().collect())
}

pub fn prepare_context<T: PipelineInput + ?Sized>(input: &T) -> Result<PipelineContext> {
    Ok(prepare_with_parsed(input)?.0)
}

pub fn prepare_config<T: PipelineInput + ?Sized>(input: &T) -> Result<PipelineContext> {
    prepare_context(input)
}

pub fn prepare_with_parsed<T: PipelineInput + ?Sized>(
    input: &T,
) -> Result<(PipelineContext, parser::ParsedApi)> {
    let ctx = input.to_pipeline_context();
    let parsed = parser::parse(&ctx)?;
    let ctx = build_pipeline_context(&ctx, &parsed)?;
    Ok((ctx, parsed))
}

fn build_pipeline_context(
    ctx: &PipelineContext,
    parsed: &parser::ParsedApi,
) -> Result<PipelineContext> {
    let known_model_types = collect_known_model_types(parsed);
    let scoped = ctx.clone().with_known_model_types(known_model_types);
    let ir = crate::ir::normalize(&scoped, parsed)?;
    let known_model_projections = model_analysis::collect_known_model_projections(&scoped, &ir)?;
    Ok(scoped.with_known_model_projections(known_model_projections))
}

fn collect_known_model_types(parsed: &parser::ParsedApi) -> Vec<String> {
    parsed
        .classes
        .iter()
        .map(|class| {
            if class.namespace.is_empty() {
                class.name.clone()
            } else {
                format!("{}::{}", class.namespace.join("::"), class.name)
            }
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn generate<T: PipelineInput + ?Sized>(input: &T, ir: &IrModule, write_ir: bool) -> Result<()> {
    let ctx = input.to_pipeline_context();
    fs::create_dir_all(ctx.output_dir()).with_context(|| {
        format!(
            "failed to create output dir: {}",
            ctx.output_dir().display()
        )
    })?;

    let header_path = ctx.output_dir().join(&ctx.output.header);
    let source_path = ctx.output_dir().join(&ctx.output.source);
    let ir_path = ctx.output_dir().join(&ctx.output.ir);
    fs::write(&header_path, render_header(&ctx, ir))
        .with_context(|| format!("failed to write header: {}", header_path.display()))?;
    fs::write(&source_path, render_source(&ctx, ir))
        .with_context(|| format!("failed to write source: {}", source_path.display()))?;
    for go_file in facade::render_go_facade(&ctx, ir)? {
        fs::create_dir_all(ctx.output_dir()).with_context(|| {
            format!(
                "failed to create go output dir: {}",
                ctx.output_dir().display()
            )
        })?;
        let go_path = ctx.output_dir().join(&go_file.filename);
        fs::write(&go_path, go_file.contents)
            .with_context(|| format!("failed to write Go wrapper: {}", go_path.display()))?;
    }
    write_go_package_metadata(&ctx)?;
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

fn write_go_package_metadata(ctx: &PipelineContext) -> Result<()> {
    let Some(go_module) = ctx.go_module.as_deref() else {
        return Ok(());
    };

    let go_mod_path = ctx.output_dir().join("go.mod");
    fs::write(&go_mod_path, render_go_mod(go_module))
        .with_context(|| format!("failed to write go.mod: {}", go_mod_path.display()))?;

    let build_flags_path = ctx.output_dir().join("build_flags.go");
    fs::write(&build_flags_path, render_build_flags(ctx)).with_context(|| {
        format!(
            "failed to write build_flags.go: {}",
            build_flags_path.display()
        )
    })?;

    Ok(())
}

fn render_go_mod(go_module: &str) -> String {
    format!("module {go_module}\n\ngo 1.25\n")
}

fn render_build_flags(ctx: &PipelineContext) -> String {
    let package_name = go_package_name(&ctx.output.dir);
    let cxxflags = exported_cxxflags(ctx);
    let cxxflags_line = cxxflags.join(" ");
    format!(
        "package {package_name}\n\n/*\n#cgo CFLAGS: -I${{SRCDIR}}\n#cgo CXXFLAGS: {cxxflags_line}\n*/\nimport \"C\"\n"
    )
}

fn exported_cxxflags(ctx: &PipelineContext) -> Vec<String> {
    let mut flags = vec!["-I${SRCDIR}".to_string()];
    let mut index = 0;
    let raw = ctx.raw_clang_args();

    while index < raw.len() {
        let arg = &raw[index];

        if arg == "-I" || arg == "-isystem" || arg == "-D" {
            if let Some(value) = raw.get(index + 1) {
                flags.push(arg.clone());
                flags.push(value.clone());
            }
            index += 2;
            continue;
        }

        if (arg.starts_with("-I") && arg.len() > 2)
            || (arg.starts_with("-isystem") && arg.len() > "-isystem".len())
            || (arg.starts_with("-D") && arg.len() > 2)
            || arg.starts_with("-std=")
        {
            flags.push(arg.clone());
        }

        index += 1;
    }

    flags
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

pub fn render_header<T: PipelineInput + ?Sized>(input: &T, ir: &IrModule) -> String {
    let ctx = input.to_pipeline_context();
    let guard = format!(
        "{}_{}",
        ctx.naming.prefix.to_uppercase(),
        ctx.output.header.replace('.', "_").to_uppercase()
    );
    let mut out = String::new();
    out.push_str(&format!("#ifndef {guard}\n#define {guard}\n\n"));
    out.push_str("#include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n\n");
    if ir_uses_struct_timeval(ir) {
        out.push_str("#include <sys/time.h>\n\n");
    }
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

    for callback in &ir.callbacks {
        render_callback_decl(&mut out, callback);
    }

    for function in &ir.functions {
        out.push_str(&render_function_decl(function));
        out.push('\n');
    }
    for function in callback_bridge_functions(ir) {
        out.push_str(&render_function_decl(&function));
        out.push('\n');
    }

    if ir
        .functions
        .iter()
        .any(|function| function.returns.kind == IrTypeKind::String)
    {
        out.push_str(&format!(
            "void {}_string_free(char* value);\n\n",
            ctx.naming.prefix
        ));
    }

    out.push_str("#ifdef __cplusplus\n}\n#endif\n\n");
    out.push_str(&format!("#endif /* {guard} */\n"));
    out
}

pub fn render_source<T: PipelineInput + ?Sized>(input: &T, ir: &IrModule) -> String {
    let ctx = input.to_pipeline_context();
    let mut out = String::new();
    out.push_str(&format!("#include \"{}\"\n", ctx.output.header));
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
    let callback_map = callback_map(ir);
    for function in ir.functions.iter().filter(|function| {
        function
            .params
            .iter()
            .any(|param| param.ty.kind == IrTypeKind::Callback)
    }) {
        out.push_str(&render_callback_bridge_def(function, &callback_map));
        out.push('\n');
    }

    if ir
        .functions
        .iter()
        .any(|function| function.returns.kind == IrTypeKind::String)
    {
        out.push_str(&render_string_free(&ctx));
    }

    out
}

pub fn render_go_structs<T: PipelineInput + ?Sized>(
    input: &T,
    ir: &IrModule,
) -> Result<Vec<GeneratedGoFile>> {
    facade::render_go_facade(input, ir)
}

fn render_callback_decl(out: &mut String, callback: &IrCallback) {
    let params = if callback.params.is_empty() {
        "void".to_string()
    } else {
        callback
            .params
            .iter()
            .map(|param| format!("{} {}", param.ty.c_type, param.name))
            .collect::<Vec<_>>()
            .join(", ")
    };
    out.push_str(&format!(
        "typedef {} (*{})({});\n\n",
        callback.returns.c_type, callback.name, params
    ));
}

pub use crate::facade::GeneratedGoFile;

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
    let body = match function.kind {
        IrFunctionKind::Constructor => render_constructor_body(function),
        IrFunctionKind::Destructor => render_destructor_body(function),
        IrFunctionKind::Method => render_method_body(function),
        IrFunctionKind::Function => render_free_function_body(function),
    };
    format!("{signature} {{\n{body}}}\n")
}

fn render_callback_bridge_def(
    function: &IrFunction,
    callbacks: &std::collections::BTreeMap<String, IrCallback>,
) -> String {
    let bridge = make_callback_bridge_function(function);
    let signature = format!(
        "{} {}({})",
        bridge.returns.c_type,
        bridge.name,
        render_param_list(&bridge)
    );
    let body = render_callback_bridge_body(function, callbacks);
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
    if let Some(accessor) = &function.field_accessor {
        let receiver = if function.is_const.unwrap_or(false) {
            format!("reinterpret_cast<const {}*>(self)", owner)
        } else {
            format!("reinterpret_cast<{}*>(self)", owner)
        };
        return match accessor.access {
            FieldAccessKind::Get => {
                render_field_getter_body(function, &receiver, &accessor.field_name)
            }
            FieldAccessKind::Set => {
                render_field_setter_body(function, &receiver, &accessor.field_name)
            }
        };
    }
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
    match function.returns.kind {
        IrTypeKind::Void => format!("    {}({});\n", target, args),
        IrTypeKind::String => format!(
            "    std::string result = {}({});\n    char* buffer = static_cast<char*>(std::malloc(result.size() + 1));\n    if (buffer == nullptr) {{\n        return nullptr;\n    }}\n    std::memcpy(buffer, result.c_str(), result.size() + 1);\n    return buffer;\n",
            target, args
        ),
        IrTypeKind::ModelView => render_model_view_return(function, target, &args),
        IrTypeKind::ModelValue => format!(
            "    return reinterpret_cast<{}>(new {}({}({})));\n",
            function.returns.c_type,
            base_model_cpp_type(&function.returns.cpp_type),
            target,
            args
        ),
        _ if function.returns.handle.is_some() => format!(
            "    return reinterpret_cast<{}>({}({}));\n",
            function.returns.c_type, target, args
        ),
        _ => format!("    return {}({});\n", target, args),
    }
}

fn render_field_getter_body(function: &IrFunction, receiver: &str, field_name: &str) -> String {
    match function.returns.kind {
        IrTypeKind::ModelValue => format!(
            "    return reinterpret_cast<{}>(new {}({receiver}->{}));\n",
            function.returns.c_type,
            base_model_cpp_type(&function.returns.cpp_type),
            field_name
        ),
        _ => format!("    return {receiver}->{};\n", field_name),
    }
}

fn render_field_setter_body(function: &IrFunction, receiver: &str, field_name: &str) -> String {
    let Some(value_param) = function.params.get(1) else {
        return format!("    {receiver}->{} = value;\n", field_name);
    };
    match value_param.ty.kind {
        IrTypeKind::ModelValue => format!(
            "    {receiver}->{} = *reinterpret_cast<{}*>(value);\n",
            field_name,
            base_model_cpp_type(&value_param.ty.cpp_type)
        ),
        _ => format!("    {receiver}->{} = value;\n", field_name),
    }
}

fn render_model_view_return(function: &IrFunction, target: &str, args: &str) -> String {
    let base = base_model_cpp_type(&function.returns.cpp_type);
    if function.returns.cpp_type.trim_end().ends_with('*') {
        return format!(
            "    auto result = {}({});\n    if (result == nullptr) {{\n        return nullptr;\n    }}\n    return reinterpret_cast<{}>(new {}(*result));\n",
            target, args, function.returns.c_type, base
        );
    }

    format!(
        "    return reinterpret_cast<{}>(new {}({}({})));\n",
        function.returns.c_type, base, target, args
    )
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
    match ty.kind {
        IrTypeKind::Primitive if ty.cpp_type != ty.c_type => {
            format!("static_cast<{}>({name})", ty.cpp_type)
        }
        IrTypeKind::String => format!("std::string({name} != nullptr ? {name} : \"\")"),
        IrTypeKind::Reference => primitive_alias_cast_target(&ty)
            .map(|cpp_type| format!("*reinterpret_cast<{}*>({name})", cpp_type))
            .unwrap_or_else(|| format!("*{name}")),
        IrTypeKind::Pointer => primitive_alias_cast_target(&ty)
            .map(|cpp_type| format!("reinterpret_cast<{}*>({name})", cpp_type))
            .unwrap_or_else(|| name.to_string()),
        IrTypeKind::ExternStructReference => format!("*{name}"),
        IrTypeKind::ModelReference => format!(
            "*reinterpret_cast<{}*>({name})",
            base_model_cpp_type(&ty.cpp_type)
        ),
        IrTypeKind::ModelPointer => format!(
            "reinterpret_cast<{}*>({name})",
            base_model_cpp_type(&ty.cpp_type)
        ),
        _ => name.to_string(),
    }
}

fn primitive_alias_cast_target(ty: &IrType) -> Option<&str> {
    let cpp_base = match ty.kind {
        IrTypeKind::Reference => ty.cpp_type.trim_end_matches('&').trim(),
        IrTypeKind::Pointer => ty.cpp_type.trim_end_matches('*').trim(),
        _ => return None,
    };
    let c_base = ty.c_type.trim_end_matches('*').trim();
    if generator_supported_primitive(cpp_base) && cpp_base != c_base {
        Some(cpp_base)
    } else {
        None
    }
}

fn char_array_length(cpp_type: &str) -> Option<usize> {
    let trimmed = cpp_type.trim().trim_start_matches("const ").trim();
    let prefix = trimmed.strip_prefix("char[")?;
    let len = prefix.strip_suffix(']')?;
    len.parse().ok()
}

fn generator_supported_primitive(name: &str) -> bool {
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

fn callback_bridge_functions(ir: &IrModule) -> Vec<IrFunction> {
    ir.functions
        .iter()
        .filter(|function| {
            function
                .params
                .iter()
                .any(|param| param.ty.kind == IrTypeKind::Callback)
        })
        .map(make_callback_bridge_function)
        .collect()
}

fn make_callback_bridge_function(function: &IrFunction) -> IrFunction {
    let params = function
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| {
            if param.ty.kind == IrTypeKind::Callback {
                IrParam {
                    name: format!("use_cb{index}"),
                    ty: IrType {
                        kind: IrTypeKind::Primitive,
                        cpp_type: "bool".to_string(),
                        c_type: "bool".to_string(),
                        handle: None,
                    },
                }
            } else {
                param.clone()
            }
        })
        .collect::<Vec<_>>();

    IrFunction {
        name: format!("{}_bridge", function.name),
        kind: IrFunctionKind::Function,
        cpp_name: function.cpp_name.clone(),
        method_of: function.method_of.clone(),
        owner_cpp_type: function.owner_cpp_type.clone(),
        is_const: function.is_const,
        field_accessor: None,
        returns: function.returns.clone(),
        params,
    }
}

fn callback_map(ir: &IrModule) -> std::collections::BTreeMap<String, IrCallback> {
    ir.callbacks
        .iter()
        .map(|callback| (callback.name.clone(), callback.clone()))
        .collect()
}

fn render_callback_bridge_body(
    function: &IrFunction,
    callbacks: &std::collections::BTreeMap<String, IrCallback>,
) -> String {
    let mut out = String::new();

    for (index, param) in function.params.iter().enumerate() {
        if param.ty.kind != IrTypeKind::Callback {
            continue;
        }
        let callback = callbacks
            .get(&param.ty.cpp_type)
            .expect("callback bridge requires callback typedef metadata");
        out.push_str(&render_callback_trampoline_decl(function, index, callback));
    }

    let target = function.name.clone();
    let call_args = function
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| {
            if param.ty.kind == IrTypeKind::Callback {
                format!(
                    "use_cb{index} ? {} : nullptr",
                    callback_trampoline_name(function, index)
                )
            } else {
                param.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    match function.returns.kind {
        IrTypeKind::Void => out.push_str(&format!("    {}({});\n", target, call_args)),
        _ => out.push_str(&format!("    return {}({});\n", target, call_args)),
    }

    out
}

fn render_callback_trampoline_decl(
    function: &IrFunction,
    index: usize,
    callback: &IrCallback,
) -> String {
    let params = if callback.params.is_empty() {
        "void".to_string()
    } else {
        callback
            .params
            .iter()
            .map(|param| format!("{} {}", param.ty.c_type, param.name))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let call_args = callback
        .params
        .iter()
        .map(|param| param.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let go_symbol = callback_go_export_name(function, index);
    let invoke = if callback.returns.kind == IrTypeKind::Void {
        format!("{}({});", go_symbol, call_args)
    } else {
        format!("return {}({});", go_symbol, call_args)
    };
    format!(
        "    extern {} {}({});\n    {} {} = []({}) -> {} {{ {} }};\n",
        callback.returns.c_type,
        go_symbol,
        params,
        callback.name,
        callback_trampoline_name(function, index),
        params,
        callback.returns.c_type,
        invoke
    )
}

fn callback_trampoline_name(function: &IrFunction, index: usize) -> String {
    format!("{}_cb{}_trampoline", function.name, index)
}

fn callback_go_export_name(function: &IrFunction, index: usize) -> String {
    format!("go_{}_cb{}", function.name, index)
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

fn ir_uses_struct_timeval(ir: &IrModule) -> bool {
    ir.functions
        .iter()
        .flat_map(|function| {
            std::iter::once(&function.returns).chain(function.params.iter().map(|param| &param.ty))
        })
        .chain(ir.callbacks.iter().flat_map(|callback| {
            std::iter::once(&callback.returns).chain(callback.params.iter().map(|param| &param.ty))
        }))
        .any(|ty| {
            matches!(
                ty.kind,
                IrTypeKind::ExternStructReference | IrTypeKind::ExternStructPointer
            ) && base_model_cpp_type(&ty.c_type) == "struct timeval"
        })
}

fn render_string_free(ctx: &PipelineContext) -> String {
    format!(
        "void {}_string_free(char* value) {{\n    std::free(value);\n}}\n",
        ctx.naming.prefix
    )
}
