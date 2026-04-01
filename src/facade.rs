use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{Result, bail};

use crate::{
    config::{Config, KnownModelProjection},
    ir::{IrFunction, IrModule, IrType},
    model::GeneratedGoFile,
};

#[derive(Debug)]
struct AnalyzedFacadeClass<'a> {
    go_name: String,
    handle_name: String,
    constructor: &'a IrFunction,
    destructor: &'a IrFunction,
    general_methods: Vec<&'a IrFunction>,
    model_mapped_methods: Vec<ModelMappedMethod<'a>>,
}

#[derive(Debug)]
struct ModelMappedMethod<'a> {
    function: &'a IrFunction,
    model: KnownModelProjection,
}

#[derive(Debug)]
enum AnalyzedMethod<'a> {
    GeneralApi(&'a IrFunction),
    ModelMapped(ModelMappedMethod<'a>),
}

pub fn render_go_facade(config: &Config, ir: &IrModule) -> Result<Vec<GeneratedGoFile>> {
    let enums = ir.enums.iter().collect::<Vec<_>>();
    let functions = ir
        .functions
        .iter()
        .filter(|function| function.kind == "function")
        .filter(|function| free_function_supported(function))
        .collect::<Vec<_>>();
    let classes = collect_facade_classes(config, ir)?;

    if functions.is_empty() && classes.is_empty() && enums.is_empty() {
        return Ok(Vec::new());
    }

    ensure_unique_go_exports(&functions)?;

    Ok(vec![GeneratedGoFile {
        filename: config.go_filename(""),
        contents: render_go_facade_file(config, &enums, &functions, &classes),
    }])
}

fn collect_facade_classes<'a>(
    config: &Config,
    ir: &'a IrModule,
) -> Result<Vec<AnalyzedFacadeClass<'a>>> {
    let mut methods_by_owner = BTreeMap::<&str, Vec<&IrFunction>>::new();
    for function in ir
        .functions
        .iter()
        .filter(|function| function.kind == "method")
    {
        let Some(owner) = function.owner_cpp_type.as_deref() else {
            continue;
        };
        methods_by_owner.entry(owner).or_default().push(function);
    }

    let constructors = ir
        .functions
        .iter()
        .filter(|function| function.kind == "constructor")
        .filter_map(|function| {
            function
                .owner_cpp_type
                .as_deref()
                .map(|owner| (owner, function))
        })
        .collect::<BTreeMap<_, _>>();
    let destructors = ir
        .functions
        .iter()
        .filter(|function| function.kind == "destructor")
        .filter_map(|function| {
            function
                .owner_cpp_type
                .as_deref()
                .map(|owner| (owner, function))
        })
        .collect::<BTreeMap<_, _>>();

    let mut classes = Vec::new();
    for (owner, methods) in methods_by_owner {
        let mut general_methods = Vec::new();
        let mut model_mapped_methods = Vec::new();
        for function in methods {
            match classify_facade_method(config, function) {
                Some(AnalyzedMethod::GeneralApi(function)) => general_methods.push(function),
                Some(AnalyzedMethod::ModelMapped(method)) => model_mapped_methods.push(method),
                None => {}
            }
        }

        if model_mapped_methods.is_empty() && general_methods.is_empty() {
            continue;
        }

        ensure_unique_method_exports(owner, &general_methods, &model_mapped_methods)?;

        let Some(constructor) = constructors.get(owner).copied() else {
            bail!("facade class `{owner}` has renderable methods but no constructor wrapper");
        };
        if !constructor
            .params
            .iter()
            .all(|param| go_param_supported(&param.ty))
        {
            bail!("facade class `{owner}` constructor params are not supported yet");
        }
        let Some(destructor) = destructors.get(owner).copied() else {
            bail!("facade class `{owner}` has renderable methods but no destructor wrapper");
        };

        classes.push(AnalyzedFacadeClass {
            go_name: leaf_cpp_name(owner).to_string(),
            handle_name: format!("{}Handle", flatten_qualified_cpp_name(owner)),
            constructor,
            destructor,
            general_methods,
            model_mapped_methods,
        });
    }

    Ok(classes)
}

fn render_go_facade_file(
    config: &Config,
    enums: &[&crate::ir::IrEnum],
    functions: &[&IrFunction],
    classes: &[AnalyzedFacadeClass<'_>],
) -> String {
    let package_name = go_package_name(&config.output.dir);
    let includes = collect_include_headers(config, classes);
    let requires_cgo = !functions.is_empty() || !classes.is_empty();
    let requires_errors = !classes.is_empty()
        || functions
            .iter()
            .any(|function| matches!(function.returns.kind.as_str(), "string" | "c_string"));
    let requires_unsafe = functions
        .iter()
        .any(|function| has_string_params(function.params.iter()))
        || classes.iter().any(|class| {
            has_string_params(class.constructor.params.iter())
                || class
                    .general_methods
                    .iter()
                    .any(|function| has_string_params(function.params.iter().skip(1)))
                || class.model_mapped_methods.iter().any(|method| {
                    has_string_params(
                        method
                            .function
                            .params
                            .iter()
                            .skip(1)
                            .take(method.function.params.len().saturating_sub(2)),
                    )
                })
        });

    let mut out = String::new();
    out.push_str(&format!("package {}\n\n", package_name));
    if requires_cgo {
        out.push_str("/*\n");
        out.push_str("#include <stdlib.h>\n");
        for include in includes {
            out.push_str(&format!("#include \"{}\"\n", include));
        }
        out.push_str("*/\n");
        out.push_str("import \"C\"\n\n");
    }
    if requires_errors {
        out.push_str("import \"errors\"\n\n");
    }
    if requires_unsafe {
        out.push_str("import \"unsafe\"\n\n");
    }

    for item in enums {
        out.push_str(&render_go_enum(item));
        out.push('\n');
    }

    let used_models = collect_used_models(classes);
    for projection in &used_models {
        out.push_str(&render_model_mapper(config, projection));
        out.push('\n');
    }

    for function in functions {
        out.push_str(&render_free_function(config, function));
        out.push('\n');
    }

    for class in classes {
        out.push_str(&render_facade_class(class));
        out.push('\n');
        out.push_str(&render_facade_constructor(class));
        out.push('\n');
        out.push_str(&render_facade_close(class));
        out.push('\n');
        for method in &class.general_methods {
            out.push_str(&render_general_api_method(config, class, method));
            out.push('\n');
        }
        for method in &class.model_mapped_methods {
            out.push_str(&render_model_mapped_method(class, method));
            out.push('\n');
        }
    }

    out
}

fn render_go_enum(item: &crate::ir::IrEnum) -> String {
    let mut out = String::new();
    let name = leaf_cpp_name(&item.cpp_name);
    out.push_str(&format!("type {} int64\n\n", name));
    out.push_str("const (\n");
    for variant in &item.variants {
        let value = variant.value.as_deref().unwrap_or("0");
        out.push_str(&format!("    {} {} = {}\n", variant.name, name, value));
    }
    out.push_str(")\n");
    out
}

fn collect_include_headers(config: &Config, classes: &[AnalyzedFacadeClass<'_>]) -> Vec<String> {
    let mut includes = BTreeSet::from([config.raw_include_for_go(&config.output.header)]);
    for projection in collect_used_models(classes) {
        includes.insert(projection.output_header.clone());
    }
    includes.into_iter().collect()
}

fn collect_used_models(classes: &[AnalyzedFacadeClass<'_>]) -> Vec<KnownModelProjection> {
    let mut models = BTreeMap::<String, KnownModelProjection>::new();
    for class in classes {
        for method in &class.model_mapped_methods {
            models
                .entry(method.model.cpp_type.clone())
                .or_insert_with(|| method.model.clone());
        }
    }
    models.into_values().collect()
}

fn render_model_mapper(config: &Config, projection: &KnownModelProjection) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "func map{}FromHandle(handle *C.{}) {} {{\n",
        projection.go_name, projection.handle_name, projection.go_name
    ));
    out.push_str("    if handle == nil {\n");
    out.push_str(&format!("        return {}{{}}\n", projection.go_name));
    out.push_str("    }\n");
    out.push_str(&format!("    model := {}{{}}\n", projection.go_name));
    for field in &projection.fields {
        match field.return_kind.as_str() {
            "string" => {
                out.push_str(&format!(
                    "    raw{} := C.{}(handle)\n",
                    field.go_name, field.getter_symbol
                ));
                out.push_str(&format!("    if raw{} != nil {{\n", field.go_name));
                out.push_str(&format!(
                    "        model.{} = C.GoString(raw{})\n",
                    field.go_name, field.go_name
                ));
                out.push_str(&format!(
                    "        C.{}_string_free(raw{})\n",
                    config.naming.prefix, field.go_name
                ));
                out.push_str("    }\n");
            }
            "c_string" => {
                out.push_str(&format!(
                    "    raw{} := C.{}(handle)\n",
                    field.go_name, field.getter_symbol
                ));
                out.push_str(&format!("    if raw{} != nil {{\n", field.go_name));
                out.push_str(&format!(
                    "        model.{} = C.GoString(raw{})\n",
                    field.go_name, field.go_name
                ));
                out.push_str("    }\n");
            }
            _ => out.push_str(&format!(
                "    model.{} = {}(C.{}(handle))\n",
                field.go_name, field.go_type, field.getter_symbol
            )),
        }
    }
    out.push_str("    return model\n");
    out.push_str("}\n");
    out
}

fn render_facade_class(class: &AnalyzedFacadeClass<'_>) -> String {
    format!(
        "type {} struct {{\n    ptr *C.{}\n}}\n",
        class.go_name, class.handle_name
    )
}

fn render_facade_constructor(class: &AnalyzedFacadeClass<'_>) -> String {
    let params = class
        .constructor
        .params
        .iter()
        .map(|param| format!("{} {}", param.name, go_type_for_ir(&param.ty).unwrap()))
        .collect::<Vec<_>>()
        .join(", ");
    let constructor_params = class.constructor.params.iter().collect::<Vec<_>>();
    let (setup_lines, cleanup_lines, rendered_args) = render_call_prep(&constructor_params);

    let mut out = format!(
        "func New{}({}) (*{}, error) {{\n",
        class.go_name, params, class.go_name
    );
    for line in setup_lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    for line in cleanup_lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str(&format!(
        "    ptr := C.{}({})\n    if ptr == nil {{\n        return nil, errors.New(\"wrapper returned nil facade handle\")\n    }}\n    return &{}{{ptr: ptr}}, nil\n}}\n",
        class.constructor.name,
        rendered_args.join(", "),
        class.go_name
    ));
    out
}

fn render_facade_close(class: &AnalyzedFacadeClass<'_>) -> String {
    format!(
        "func ({} *{}) Close() {{\n    if {} == nil || {}.ptr == nil {{\n        return\n    }}\n    C.{}({}.ptr)\n    {}.ptr = nil\n}}\n",
        receiver_name(&class.go_name),
        class.go_name,
        receiver_name(&class.go_name),
        receiver_name(&class.go_name),
        class.destructor.name,
        receiver_name(&class.go_name),
        receiver_name(&class.go_name),
    )
}

fn render_model_mapped_method(
    class: &AnalyzedFacadeClass<'_>,
    method: &ModelMappedMethod<'_>,
) -> String {
    let receiver = receiver_name(&class.go_name);
    let method_params = method
        .function
        .params
        .iter()
        .skip(1)
        .take(method.function.params.len().saturating_sub(2))
        .collect::<Vec<_>>();
    let params = method_params
        .iter()
        .map(|param| format!("{} {}", param.name, go_type_for_ir(&param.ty).unwrap()))
        .collect::<Vec<_>>()
        .join(", ");
    let (setup_lines, cleanup_lines, rendered_args) = render_call_prep(&method_params);
    let call_args = std::iter::once(format!("{receiver}.ptr"))
        .chain(rendered_args)
        .chain(std::iter::once("out".to_string()))
        .collect::<Vec<_>>()
        .join(", ");

    let mut out = String::new();
    out.push_str(&format!(
        "func ({} *{}) {}({}) ({}, error) {{\n",
        receiver,
        class.go_name,
        go_method_export_name(method.function, true),
        params,
        method.model.go_name
    ));
    out.push_str(&format!(
        "    if {} == nil || {}.ptr == nil {{\n        return {}{{}}, errors.New(\"facade receiver is nil\")\n    }}\n",
        receiver, receiver, method.model.go_name
    ));
    for line in setup_lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    for line in cleanup_lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str(&format!(
        "    out := C.{}()\n    if out == nil {{\n        return {}{{}}, errors.New(\"failed to allocate model handle\")\n    }}\n",
        method.model.constructor_symbol, method.model.go_name
    ));
    if let Some(destructor) = &method.model.destructor_symbol {
        out.push_str(&format!("    defer C.{}(out)\n", destructor));
    }
    out.push_str(&format!(
        "    if !bool(C.{}({})) {{\n        return {}{{}}, errors.New(\"facade call failed\")\n    }}\n",
        method.function.name, call_args, method.model.go_name
    ));
    out.push_str(&format!(
        "    return map{}FromHandle(out), nil\n",
        method.model.go_name
    ));
    out.push_str("}\n");
    out
}

fn render_general_api_method(
    config: &Config,
    class: &AnalyzedFacadeClass<'_>,
    function: &IrFunction,
) -> String {
    let receiver = receiver_name(&class.go_name);
    let method_params = function.params.iter().skip(1).collect::<Vec<_>>();
    let params = method_params
        .iter()
        .map(|param| format!("{} {}", param.name, go_type_for_ir(&param.ty).unwrap()))
        .collect::<Vec<_>>()
        .join(", ");
    let (setup_lines, cleanup_lines, rendered_args) = render_call_prep(&method_params);
    let call = format!(
        "C.{}({})",
        function.name,
        std::iter::once(format!("{receiver}.ptr"))
            .chain(rendered_args)
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut out = String::new();
    out.push_str(&format!(
        "func ({} *{}) {}({})",
        receiver,
        class.go_name,
        go_method_export_name(function, false),
        params
    ));
    match function.returns.kind.as_str() {
        "void" => out.push_str(" {\n"),
        "string" | "c_string" => out.push_str(" (string, error) {\n"),
        _ => out.push_str(&format!(
            " {} {{\n",
            go_type_for_ir(&function.returns).unwrap()
        )),
    }
    out.push_str(&format!(
        "    if {} == nil || {}.ptr == nil {{\n",
        receiver, receiver
    ));
    match function.returns.kind.as_str() {
        "void" => out.push_str("        return\n"),
        "string" | "c_string" => {
            out.push_str("        return \"\", errors.New(\"facade receiver is nil\")\n")
        }
        _ => out.push_str(&format!(
            "        return {}\n",
            zero_value_for_go_type(go_type_for_ir(&function.returns).unwrap())
        )),
    }
    out.push_str("    }\n");
    for line in setup_lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    for line in cleanup_lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    match function.returns.kind.as_str() {
        "void" => out.push_str(&format!("    {}\n", call)),
        "string" => out.push_str(&format!(
            "    raw := {}\n    if raw == nil {{\n        return \"\", errors.New(\"wrapper returned nil string\")\n    }}\n    defer C.{}_string_free(raw)\n    return C.GoString(raw), nil\n",
            call, config.naming.prefix
        )),
        "c_string" => out.push_str(&format!(
            "    raw := {}\n    if raw == nil {{\n        return \"\", errors.New(\"wrapper returned nil string\")\n    }}\n    return C.GoString(raw), nil\n",
            call
        )),
        _ => out.push_str(&format!(
            "    return {}({})\n",
            go_type_for_ir(&function.returns).unwrap(),
            call
        )),
    }
    out.push_str("}\n");
    out
}

fn render_free_function(config: &Config, function: &IrFunction) -> String {
    let go_name = go_facade_export_name(function);
    let params_list = function.params.iter().collect::<Vec<_>>();
    let params = params_list
        .iter()
        .map(|param| format!("{} {}", param.name, go_type_for_ir(&param.ty).unwrap()))
        .collect::<Vec<_>>()
        .join(", ");
    let (setup_lines, cleanup_lines, rendered_args) = render_call_prep(&params_list);
    let call = format!("C.{}({})", function.name, rendered_args.join(", "));

    match function.returns.kind.as_str() {
        "void" => {
            let mut out = format!("func {}({}) {{\n", go_name, params);
            for line in setup_lines {
                out.push_str("    ");
                out.push_str(&line);
                out.push('\n');
            }
            for line in cleanup_lines {
                out.push_str("    ");
                out.push_str(&line);
                out.push('\n');
            }
            out.push_str(&format!("    {}\n", call));
            out.push('}');
            out
        }
        "string" => format!(
            "func {}({}) (string, error) {{\n{}{}    raw := {}\n    if raw == nil {{\n        return \"\", errors.New(\"wrapper returned nil string\")\n    }}\n    defer C.{}_string_free(raw)\n    return C.GoString(raw), nil\n}}",
            go_name,
            params,
            indented_lines(&setup_lines),
            indented_lines(&cleanup_lines),
            call,
            config.naming.prefix
        ),
        "c_string" => format!(
            "func {}({}) (string, error) {{\n{}{}    raw := {}\n    if raw == nil {{\n        return \"\", errors.New(\"wrapper returned nil string\")\n    }}\n    return C.GoString(raw), nil\n}}",
            go_name,
            params,
            indented_lines(&setup_lines),
            indented_lines(&cleanup_lines),
            call
        ),
        _ => format!(
            "func {}({}) {} {{\n{}{}    return {}({})\n}}",
            go_name,
            params,
            go_type_for_ir(&function.returns).unwrap(),
            indented_lines(&setup_lines),
            indented_lines(&cleanup_lines),
            go_type_for_ir(&function.returns).unwrap(),
            call
        ),
    }
}

fn ensure_unique_go_exports(functions: &[&IrFunction]) -> Result<()> {
    let mut by_export = BTreeMap::<String, Vec<String>>::new();
    for function in functions {
        by_export
            .entry(go_facade_export_name(function))
            .or_default()
            .push(function.cpp_name.clone());
    }

    let collisions = by_export
        .into_iter()
        .filter(|(_, names)| names.len() > 1)
        .collect::<Vec<_>>();
    if collisions.is_empty() {
        return Ok(());
    }

    let detail = collisions
        .into_iter()
        .map(|(export, names)| {
            format!(
                "Go facade export `{export}` collides for: {}",
                names.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    bail!("facade export collision detected: {detail}");
}

fn ensure_unique_method_exports(
    owner: &str,
    general_methods: &[&IrFunction],
    model_mapped_methods: &[ModelMappedMethod<'_>],
) -> Result<()> {
    let mut by_export = BTreeMap::<String, Vec<String>>::new();
    for function in general_methods {
        by_export
            .entry(go_method_export_name(function, false))
            .or_default()
            .push(function.cpp_name.clone());
    }
    for method in model_mapped_methods {
        by_export
            .entry(go_method_export_name(method.function, true))
            .or_default()
            .push(method.function.cpp_name.clone());
    }

    let collisions = by_export
        .into_iter()
        .filter(|(_, names)| names.len() > 1)
        .collect::<Vec<_>>();
    if collisions.is_empty() {
        return Ok(());
    }

    let detail = collisions
        .into_iter()
        .map(|(export, names)| {
            format!(
                "Go facade method `{owner}.{export}` collides for: {}",
                names.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    bail!("facade export collision detected: {detail}");
}

fn free_function_supported(function: &IrFunction) -> bool {
    go_return_supported(&function.returns)
        && function
            .params
            .iter()
            .all(|param| go_param_supported(&param.ty))
}

fn classify_facade_method<'a>(
    config: &Config,
    function: &'a IrFunction,
) -> Option<AnalyzedMethod<'a>> {
    if let Some(model) = model_projection_for_out_param(config, function) {
        return Some(AnalyzedMethod::ModelMapped(ModelMappedMethod {
            function,
            model,
        }));
    }

    general_method_supported(function).then_some(AnalyzedMethod::GeneralApi(function))
}

fn general_method_supported(function: &IrFunction) -> bool {
    model_out_param(function).is_none()
        && go_return_supported(&function.returns)
        && function
            .params
            .iter()
            .skip(1)
            .all(|param| go_param_supported(&param.ty))
}

fn liftable_method_supported(function: &IrFunction) -> bool {
    if function.returns.kind != "primitive"
        || normalize_type_key(&function.returns.cpp_type) != "bool"
    {
        return false;
    }
    if function.params.len() < 3 {
        return false;
    }
    function
        .params
        .iter()
        .skip(1)
        .take(function.params.len() - 2)
        .all(|param| go_param_supported(&param.ty))
}

fn model_projection_for_out_param(
    config: &Config,
    function: &IrFunction,
) -> Option<KnownModelProjection> {
    let model_param = model_out_param(function)?;
    let model = config.known_model_projection(&model_param.cpp_type)?;
    if !liftable_method_supported(function)
        || model.constructor_symbol.is_empty()
        || model.destructor_symbol.is_none()
    {
        return None;
    }
    Some(model.clone())
}

fn model_out_param(function: &IrFunction) -> Option<&IrType> {
    let ty = &function.params.last()?.ty;
    matches!(ty.kind.as_str(), "model_reference" | "model_pointer").then_some(ty)
}

fn render_c_arg(ty: &IrType, name: &str) -> String {
    format!("{}({})", cgo_cast_type(ty), name)
}

fn render_call_prep(params: &[&crate::ir::IrParam]) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut setup_lines = Vec::new();
    let mut cleanup_lines = Vec::new();
    let mut args = Vec::new();

    for (index, param) in params.iter().enumerate() {
        match param.ty.kind.as_str() {
            "string" | "c_string" => {
                let c_name = format!("cArg{index}");
                setup_lines.push(format!("{c_name} := C.CString({})", param.name));
                cleanup_lines.push(format!("defer C.free(unsafe.Pointer({c_name}))"));
                args.push(c_name);
            }
            _ => args.push(render_c_arg(&param.ty, &param.name)),
        }
    }

    (setup_lines, cleanup_lines, args)
}

fn indented_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    lines
        .iter()
        .map(|line| format!("    {line}\n"))
        .collect::<String>()
}

fn has_string_params<'a>(mut params: impl Iterator<Item = &'a crate::ir::IrParam>) -> bool {
    params.any(|param| matches!(param.ty.kind.as_str(), "string" | "c_string"))
}

fn go_param_supported(ty: &IrType) -> bool {
    matches!(ty.kind.as_str(), "string" | "c_string")
        || (ty.kind == "primitive" && go_type_for_ir(ty).is_some())
}

fn go_return_supported(ty: &IrType) -> bool {
    ty.kind == "void" || go_param_supported(ty)
}

fn zero_value_for_go_type(go_type: &str) -> &'static str {
    match go_type {
        "bool" => "false",
        "string" => "\"\"",
        "float32" | "float64" | "int" | "int8" | "int16" | "int32" | "int64" | "uint8"
        | "uint16" | "uint32" | "uint64" | "uintptr" => "0",
        _ => "0",
    }
}

fn go_type_for_ir(ty: &IrType) -> Option<&'static str> {
    match ty.kind.as_str() {
        "string" | "c_string" => Some("string"),
        "primitive" => match normalize_type_key(&ty.cpp_type).as_str() {
            "bool" => Some("bool"),
            "float" => Some("float32"),
            "double" => Some("float64"),
            "int8" | "int8_t" => Some("int8"),
            "int16" | "int16_t" => Some("int16"),
            "int32" | "int32_t" => Some("int32"),
            "int64" | "int64_t" => Some("int64"),
            "uint8" | "uint8_t" => Some("uint8"),
            "uint16" | "uint16_t" => Some("uint16"),
            "uint32" | "uint32_t" => Some("uint32"),
            "uint64" | "uint64_t" => Some("uint64"),
            "int" => Some("int"),
            "short" => Some("int16"),
            "long" => Some("int64"),
            "size_t" => Some("uintptr"),
            _ => None,
        },
        _ => None,
    }
}

fn cgo_cast_type(ty: &IrType) -> &'static str {
    match normalize_type_key(&ty.cpp_type).as_str() {
        "bool" => "C.bool",
        "float" => "C.float",
        "double" => "C.double",
        "int8" | "int8_t" => "C.int8_t",
        "int16" | "int16_t" => "C.int16_t",
        "int32" | "int32_t" => "C.int32_t",
        "int64" | "int64_t" => "C.int64_t",
        "uint8" | "uint8_t" => "C.uint8_t",
        "uint16" | "uint16_t" => "C.uint16_t",
        "uint32" | "uint32_t" => "C.uint32_t",
        "uint64" | "uint64_t" => "C.uint64_t",
        "short" => "C.short",
        "long" => "C.long",
        "size_t" => "C.size_t",
        _ => "C.int",
    }
}

fn normalize_type_key(value: &str) -> String {
    value
        .replace(' ', "")
        .trim_start_matches("const")
        .trim_end_matches('&')
        .trim_end_matches('*')
        .to_string()
}

fn go_export_name(value: &str) -> String {
    value
        .split('_')
        .flat_map(split_pascal_tokens)
        .filter(|token| !token.is_empty())
        .map(|token| {
            let mut chars = token.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            format!(
                "{}{}",
                first.to_ascii_uppercase(),
                chars.collect::<String>()
            )
        })
        .collect::<String>()
}

fn go_facade_export_name(function: &IrFunction) -> String {
    let base = go_export_name(&leaf_cpp_name(&function.cpp_name));
    if !function.name.contains("__") {
        return base;
    }

    format!("{base}{}", go_overload_suffix(function, false))
}

fn go_method_export_name(function: &IrFunction, drop_model_out_param: bool) -> String {
    let base = go_export_name(method_name(function));
    if !function.name.contains("__") {
        return base;
    }

    format!(
        "{base}{}",
        go_overload_suffix(function, drop_model_out_param)
    )
}

fn go_overload_suffix(function: &IrFunction, drop_model_out_param: bool) -> String {
    let mut params = function.params.iter().collect::<Vec<_>>();
    if function.method_of.is_some() && !params.is_empty() {
        params.remove(0);
    }
    if drop_model_out_param && !params.is_empty() {
        params.pop();
    }

    let mut suffix = params
        .iter()
        .map(|param| go_overload_token(&param.ty))
        .collect::<String>();
    if suffix.is_empty() {
        suffix = "Void".to_string();
    }
    if function.is_const == Some(true) {
        suffix.push_str("Const");
    }
    suffix
}

fn go_overload_token(ty: &IrType) -> String {
    match ty.kind.as_str() {
        "string" | "c_string" => "String".to_string(),
        "primitive" => go_type_for_ir(ty)
            .map(go_export_name)
            .unwrap_or_else(|| go_export_name(&sanitize_go_token(&ty.cpp_type))),
        _ => go_export_name(&sanitize_go_token(&ty.cpp_type)),
    }
}

fn sanitize_go_token(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn method_name(function: &IrFunction) -> &str {
    function
        .cpp_name
        .rsplit("::")
        .next()
        .unwrap_or(&function.cpp_name)
}

fn receiver_name(value: &str) -> String {
    value
        .chars()
        .next()
        .map(|ch| ch.to_ascii_lowercase().to_string())
        .unwrap_or_else(|| "v".to_string())
}

fn split_pascal_tokens(value: &str) -> Vec<String> {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Vec::new();
    }

    let mut tokens = Vec::new();
    let mut start = 0;
    for index in 1..chars.len() {
        let prev = chars[index - 1];
        let current = chars[index];
        let next = chars.get(index + 1).copied();

        let boundary = (prev.is_lowercase() && current.is_uppercase())
            || (prev.is_ascii_digit() && !current.is_ascii_digit())
            || (!prev.is_ascii_digit() && current.is_ascii_digit())
            || (prev.is_uppercase()
                && current.is_uppercase()
                && next.map(|ch| ch.is_lowercase()).unwrap_or(false));

        if boundary {
            tokens.push(chars[start..index].iter().collect::<String>());
            start = index;
        }
    }
    tokens.push(chars[start..].iter().collect::<String>());
    tokens
}

fn leaf_cpp_name(value: &str) -> String {
    value.rsplit("::").next().unwrap_or(value).to_string()
}

fn flatten_qualified_cpp_name(value: &str) -> String {
    value.split("::").collect::<Vec<_>>().join("")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KnownModelField;
    use crate::ir::IrParam;

    fn test_config_with_known_model() -> Config {
        Config {
            known_model_projections: vec![KnownModelProjection {
                cpp_type: "ThingModel".to_string(),
                handle_name: "ThingModelHandle".to_string(),
                go_name: "ThingModel".to_string(),
                output_header: "thing_model_wrapper.h".to_string(),
                constructor_symbol: "cgowrap_ThingModel_new".to_string(),
                destructor_symbol: Some("cgowrap_ThingModel_delete".to_string()),
                fields: vec![KnownModelField {
                    go_name: "Value".to_string(),
                    go_type: "int".to_string(),
                    getter_symbol: "cgowrap_ThingModel_GetValue".to_string(),
                    return_kind: "primitive".to_string(),
                }],
            }],
            ..Config::default()
        }
    }

    fn primitive_type(cpp_type: &str, c_type: &str) -> IrType {
        IrType {
            kind: "primitive".to_string(),
            cpp_type: cpp_type.to_string(),
            c_type: c_type.to_string(),
            handle: None,
        }
    }

    fn model_reference_type(cpp_type: &str) -> IrType {
        IrType {
            kind: "model_reference".to_string(),
            cpp_type: cpp_type.to_string(),
            c_type: format!("{cpp_type}Handle*"),
            handle: Some(format!("{cpp_type}Handle")),
        }
    }

    fn method(name: &str, params: Vec<IrParam>) -> IrFunction {
        IrFunction {
            name: format!("cgowrap_Api_{name}"),
            kind: "method".to_string(),
            cpp_name: format!("Api::{name}"),
            method_of: Some("Api".to_string()),
            owner_cpp_type: Some("Api".to_string()),
            is_const: Some(false),
            returns: primitive_type("bool", "bool"),
            params,
        }
    }

    #[test]
    fn classify_facade_method_marks_known_model_out_param_as_model_mapped() {
        let config = test_config_with_known_model();
        let function = method(
            "GetThing",
            vec![
                IrParam {
                    name: "self".to_string(),
                    ty: IrType {
                        kind: "opaque".to_string(),
                        cpp_type: "Api".to_string(),
                        c_type: "ApiHandle*".to_string(),
                        handle: Some("ApiHandle".to_string()),
                    },
                },
                IrParam {
                    name: "id".to_string(),
                    ty: primitive_type("int", "int"),
                },
                IrParam {
                    name: "out".to_string(),
                    ty: model_reference_type("ThingModel"),
                },
            ],
        );

        let classified = classify_facade_method(&config, &function).unwrap();
        assert!(matches!(classified, AnalyzedMethod::ModelMapped(_)));
    }

    #[test]
    fn classify_facade_method_does_not_lift_known_model_outside_last_position() {
        let config = test_config_with_known_model();
        let function = method(
            "GetThing",
            vec![
                IrParam {
                    name: "self".to_string(),
                    ty: IrType {
                        kind: "opaque".to_string(),
                        cpp_type: "Api".to_string(),
                        c_type: "ApiHandle*".to_string(),
                        handle: Some("ApiHandle".to_string()),
                    },
                },
                IrParam {
                    name: "out".to_string(),
                    ty: model_reference_type("ThingModel"),
                },
                IrParam {
                    name: "id".to_string(),
                    ty: primitive_type("int", "int"),
                },
            ],
        );

        assert!(classify_facade_method(&config, &function).is_none());
    }
}
