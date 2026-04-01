use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::{
    config::Config,
    facade,
    ir::{IrCallback, IrEnum, IrFunction, IrModule, IrParam, IrType},
    parser,
};

pub fn generate_all(config: &Config, write_ir: bool) -> Result<()> {
    let (config, parsed) = prepare_with_parsed(config)?;
    let generation_headers = generation_headers(&config);

    if generation_headers.len() > 1 && !config.uses_default_output_names() {
        bail!(
            "multi-header generation does not support explicit output.header/source/ir overrides; leave them as defaults to emit one wrapper set per header"
        );
    }

    if generation_headers.len() <= 1 {
        let scoped = generation_headers
            .first()
            .cloned()
            .map(|header| config.scoped_to_header(header))
            .unwrap_or_else(|| config.clone());
        let header_api = scoped
            .target_header
            .as_deref()
            .map(|header| parsed.filter_to_header(header))
            .unwrap_or_else(|| parsed.clone());
        let ir = crate::ir::normalize(&scoped, &header_api)?;
        return generate(&scoped, &ir, write_ir);
    }

    for header in &generation_headers {
        let scoped = config.scoped_to_header(header.clone());
        let header_api = parsed.filter_to_header(header);
        let ir = crate::ir::normalize(&scoped, &header_api)?;
        generate(&scoped, &ir, write_ir)?;
    }

    Ok(())
}

fn generation_headers(config: &Config) -> Vec<PathBuf> {
    if config.input.dir.is_some() {
        return scan_generation_headers(config.input.dir.as_ref().unwrap()).unwrap_or_default();
    }

    config.input.headers.clone()
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

pub fn prepare_config(config: &Config) -> Result<Config> {
    Ok(prepare_with_parsed(config)?.0)
}

pub fn prepare_with_parsed(config: &Config) -> Result<(Config, parser::ParsedApi)> {
    let parsed = parser::parse(config)?;
    let config = prepare_config_from_parsed(config, &parsed)?;
    Ok((config, parsed))
}

fn prepare_config_from_parsed(config: &Config, parsed: &parser::ParsedApi) -> Result<Config> {
    let known_model_types = collect_known_model_types(parsed);
    let scoped = config.clone().with_known_model_types(known_model_types);
    let ir = crate::ir::normalize(&scoped, parsed)?;
    let known_model_projections = facade::collect_known_model_projections(&scoped, &ir)?;
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

pub fn generate(config: &Config, ir: &IrModule, write_ir: bool) -> Result<()> {
    fs::create_dir_all(config.raw_output_dir()).with_context(|| {
        format!(
            "failed to create raw output dir: {}",
            config.raw_output_dir().display()
        )
    })?;

    let header_path = config.raw_output_dir().join(&config.output.header);
    let source_path = config.raw_output_dir().join(&config.output.source);
    let ir_path = config.raw_output_dir().join(&config.output.ir);
    fs::write(&header_path, render_header(config, ir))
        .with_context(|| format!("failed to write header: {}", header_path.display()))?;
    fs::write(&source_path, render_source(config, ir))
        .with_context(|| format!("failed to write source: {}", source_path.display()))?;
    for go_file in facade::render_go_facade(config, ir)? {
        fs::create_dir_all(config.go_output_dir()).with_context(|| {
            format!(
                "failed to create go output dir: {}",
                config.go_output_dir().display()
            )
        })?;
        let go_path = config.go_output_dir().join(&go_file.filename);
        fs::write(&go_path, go_file.contents)
            .with_context(|| format!("failed to write Go wrapper: {}", go_path.display()))?;
    }
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

pub fn render_header(config: &Config, ir: &IrModule) -> String {
    let guard = format!(
        "{}_{}",
        config.naming.prefix.to_uppercase(),
        config.output.header.replace('.', "_").to_uppercase()
    );
    let mut out = String::new();
    out.push_str(&format!("#ifndef {guard}\n#define {guard}\n\n"));
    out.push_str("#include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n\n");
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

    for item in &ir.enums {
        render_enum_decl(&mut out, item);
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
        .any(|function| function.returns.kind == "string")
    {
        out.push_str(&format!(
            "void {}_string_free(char* value);\n\n",
            config.naming.prefix
        ));
    }

    out.push_str("#ifdef __cplusplus\n}\n#endif\n\n");
    out.push_str(&format!("#endif /* {guard} */\n"));
    out
}

pub fn render_source(config: &Config, ir: &IrModule) -> String {
    let mut out = String::new();
    out.push_str(&format!("#include \"{}\"\n", config.output.header));
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
    for function in ir
        .functions
        .iter()
        .filter(|function| function.params.iter().any(|param| param.ty.kind == "callback"))
    {
        out.push_str(&render_callback_bridge_def(function, &callback_map));
        out.push('\n');
    }

    if ir
        .functions
        .iter()
        .any(|function| function.returns.kind == "string")
    {
        out.push_str(&render_string_free(config));
    }

    out
}

pub fn render_go_structs(config: &Config, ir: &IrModule) -> Result<Vec<GeneratedGoFile>> {
    facade::render_go_facade(config, ir)
}

fn render_enum_decl(out: &mut String, item: &IrEnum) {
    out.push_str(&format!("typedef enum {} {{\n", item.name));
    for variant in &item.variants {
        match &variant.value {
            Some(value) => out.push_str(&format!("    {} = {},\n", variant.name, value)),
            None => out.push_str(&format!("    {},\n", variant.name)),
        }
    }
    out.push_str(&format!("}} {};\n\n", item.name));
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
    let body = match function.kind.as_str() {
        "constructor" => render_constructor_body(function),
        "destructor" => render_destructor_body(function),
        "method" => render_method_body(function),
        _ => render_free_function_body(function),
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
    match function.returns.kind.as_str() {
        "void" => format!("    {}({});\n", target, args),
        "string" => format!(
            "    std::string result = {}({});\n    char* buffer = static_cast<char*>(std::malloc(result.size() + 1));\n    if (buffer == nullptr) {{\n        return nullptr;\n    }}\n    std::memcpy(buffer, result.c_str(), result.size() + 1);\n    return buffer;\n",
            target, args
        ),
        _ if function.returns.handle.is_some() => format!(
            "    return reinterpret_cast<{}>({}({}));\n",
            function.returns.c_type, target, args
        ),
        _ => format!("    return {}({});\n", target, args),
    }
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
    match ty.kind.as_str() {
        "string" => format!("std::string({name} != nullptr ? {name} : \"\")"),
        "reference" => format!("*{name}"),
        "model_reference" => format!(
            "*reinterpret_cast<{}*>({name})",
            base_model_cpp_type(&ty.cpp_type)
        ),
        "model_pointer" => format!(
            "reinterpret_cast<{}*>({name})",
            base_model_cpp_type(&ty.cpp_type)
        ),
        _ => name.to_string(),
    }
}

fn callback_bridge_functions(ir: &IrModule) -> Vec<IrFunction> {
    ir.functions
        .iter()
        .filter(|function| function.params.iter().any(|param| param.ty.kind == "callback"))
        .map(make_callback_bridge_function)
        .collect()
}

fn make_callback_bridge_function(function: &IrFunction) -> IrFunction {
    let params = function
        .params
        .iter()
        .enumerate()
        .filter_map(|(index, param)| {
            if param.ty.kind == "callback" {
                Some(IrParam {
                    name: format!("use_cb{index}"),
                    ty: IrType {
                        kind: "primitive".to_string(),
                        cpp_type: "bool".to_string(),
                        c_type: "bool".to_string(),
                        handle: None,
                    },
                })
            } else {
                Some(param.clone())
            }
        })
        .collect::<Vec<_>>();

    IrFunction {
        name: format!("{}_bridge", function.name),
        kind: "function".to_string(),
        cpp_name: function.cpp_name.clone(),
        method_of: function.method_of.clone(),
        owner_cpp_type: function.owner_cpp_type.clone(),
        is_const: function.is_const,
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
        if param.ty.kind != "callback" {
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
            if param.ty.kind == "callback" {
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

    match function.returns.kind.as_str() {
        "void" => out.push_str(&format!("    {}({});\n", target, call_args)),
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
    let invoke = if callback.returns.kind == "void" {
        format!("{}({});", go_symbol, call_args)
    } else {
        format!("return {}({});", go_symbol, call_args)
    };
    format!(
        "    extern {} {}({});\n    auto {} = []({}) -> {} {{ {} }};\n",
        callback.returns.c_type,
        go_symbol,
        params,
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

fn render_string_free(config: &Config) -> String {
    format!(
        "void {}_string_free(char* value) {{\n    std::free(value);\n}}\n",
        config.naming.prefix
    )
}
