use std::{collections::BTreeSet, fs, path::Path};

use anyhow::{Context, Result, bail};

use crate::{
    config::Config,
    facade,
    ir::{IrEnum, IrFunction, IrModule, IrType},
    model, parser,
};

pub fn generate_all(config: &Config, write_ir: bool) -> Result<()> {
    let config = prepare_config(config)?;

    if config.input.headers.len() > 1 && !config.uses_default_output_names() {
        bail!(
            "multi-header generation does not support explicit output.header/source/ir overrides; leave them as defaults to emit one wrapper set per header"
        );
    }

    if config.input.headers.len() <= 1 {
        let scoped = config
            .input
            .headers
            .first()
            .cloned()
            .map(|header| config.scoped_to_header(header))
            .unwrap_or_else(|| config.clone());
        let parsed = parser::parse(&scoped)?;
        let ir = crate::ir::normalize(&scoped, &parsed)?;
        return generate(&scoped, &ir, write_ir);
    }

    for header in &config.input.headers {
        let scoped = config.scoped_to_header(header.clone());
        let parsed = parser::parse(&scoped)?;
        let ir = crate::ir::normalize(&scoped, &parsed)?;
        generate(&scoped, &ir, write_ir)?;
    }

    Ok(())
}

pub fn prepare_config(config: &Config) -> Result<Config> {
    let known_model_types = collect_known_model_types(config)?;
    let known_model_projections = collect_known_model_projections(config)?;
    Ok(config
        .clone()
        .with_known_model_types(known_model_types)
        .with_known_model_projections(known_model_projections))
}

fn collect_known_model_types(config: &Config) -> Result<Vec<String>> {
    let mut known_model_types = BTreeSet::new();

    for header in &config.files.model {
        let scoped = config
            .scoped_to_header(header.clone())
            .with_known_model_types(Vec::new());
        let parsed = parser::parse(&scoped)?;
        for class in parsed.classes {
            let qualified = if class.namespace.is_empty() {
                class.name
            } else {
                format!("{}::{}", class.namespace.join("::"), class.name)
            };
            known_model_types.insert(qualified);
        }
    }

    Ok(known_model_types.into_iter().collect())
}

fn collect_known_model_projections(
    config: &Config,
) -> Result<Vec<crate::config::KnownModelProjection>> {
    let mut projections = Vec::new();

    for header in &config.files.model {
        let scoped = config
            .scoped_to_header(header.clone())
            .with_known_model_types(Vec::new());
        let parsed = parser::parse(&scoped)?;
        let ir = crate::ir::normalize(&scoped, &parsed)?;
        projections.extend(crate::model::collect_known_model_projections(&scoped, &ir)?);
    }

    Ok(projections)
}

pub fn generate(config: &Config, ir: &IrModule, write_ir: bool) -> Result<()> {
    fs::create_dir_all(&config.output.dir).with_context(|| {
        format!(
            "failed to create output dir: {}",
            config.output.dir.display()
        )
    })?;

    let header_path = config.output.dir.join(&config.output.header);
    let source_path = config.output.dir.join(&config.output.source);
    let ir_path = config.output.dir.join(&config.output.ir);
    fs::write(&header_path, render_header(config, ir))
        .with_context(|| format!("failed to write header: {}", header_path.display()))?;
    fs::write(&source_path, render_source(config, ir))
        .with_context(|| format!("failed to write source: {}", source_path.display()))?;
    for go_model in model::render_go_models(config, ir)? {
        let go_path = config.output.dir.join(&go_model.filename);
        fs::write(&go_path, go_model.contents)
            .with_context(|| format!("failed to write Go models: {}", go_path.display()))?;
    }
    for go_facade in facade::render_go_facade(config, ir)? {
        let go_path = config.output.dir.join(&go_facade.filename);
        fs::write(&go_path, go_facade.contents)
            .with_context(|| format!("failed to write Go facade: {}", go_path.display()))?;
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

    for function in &ir.functions {
        out.push_str(&render_function_decl(function));
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
    model::render_go_structs(config, ir)
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

pub use model::GeneratedGoFile;

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
