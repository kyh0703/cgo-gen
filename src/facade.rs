use std::{collections::BTreeMap, path::Path};

use anyhow::{Result, bail};

use crate::{
    config::{Config, KnownModelField, KnownModelProjection},
    ir::{IrCallback, IrEnum, IrFunction, IrModule, IrType},
};

#[derive(Debug)]
pub struct GeneratedGoFile {
    pub filename: String,
    pub contents: String,
}

#[derive(Debug)]
struct AnalyzedFacadeClass<'a> {
    go_name: String,
    handle_name: String,
    constructor: &'a IrFunction,
    destructor: &'a IrFunction,
    methods: Vec<&'a IrFunction>,
}

#[derive(Debug, Default)]
struct RenderedCallPrep {
    setup_lines: Vec<String>,
    defer_lines: Vec<String>,
    post_call_lines: Vec<String>,
    args: Vec<String>,
}

#[derive(Debug, Clone)]
struct CallbackUsage<'a> {
    callback: &'a IrCallback,
    function: &'a IrFunction,
    param_index: usize,
}

pub fn render_go_facade(config: &Config, ir: &IrModule) -> Result<Vec<GeneratedGoFile>> {
    let functions = ir
        .functions
        .iter()
        .filter(|function| function.kind == "function")
        .filter(|function| free_function_supported(config, function))
        .collect::<Vec<_>>();
    let enums = ir.enums.iter().collect::<Vec<_>>();
    let classes = collect_facade_classes(config, ir)?;
    let callback_usages = collect_callback_usages(&functions, &classes, ir);

    if functions.is_empty() && classes.is_empty() && enums.is_empty() {
        return Ok(Vec::new());
    }

    ensure_unique_go_exports(&functions)?;

    Ok(vec![GeneratedGoFile {
        filename: config.go_filename(""),
        contents: render_go_facade_file(config, &enums, &functions, &classes, &callback_usages),
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
        if method_supported(config, function) {
            methods_by_owner.entry(owner).or_default().push(function);
        }
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
        ensure_unique_method_exports(owner, &methods)?;

        let Some(constructor) = constructors.get(owner).copied() else {
            continue;
        };
        if !constructor
            .params
            .iter()
            .all(|param| go_param_supported(config, &param.ty))
        {
            continue;
        }
        let Some(destructor) = destructors.get(owner).copied() else {
            continue;
        };

        classes.push(AnalyzedFacadeClass {
            go_name: leaf_cpp_name(owner).to_string(),
            handle_name: format!("{}Handle", flatten_qualified_cpp_name(owner)),
            constructor,
            destructor,
            methods,
        });
    }

    Ok(classes)
}

fn render_go_facade_file(
    config: &Config,
    enums: &[&IrEnum],
    functions: &[&IrFunction],
    classes: &[AnalyzedFacadeClass<'_>],
    callback_usages: &[CallbackUsage<'_>],
) -> String {
    let package_name = go_package_name(&config.output.dir);
    let requires_cgo = !functions.is_empty() || !classes.is_empty();
    let requires_errors = !classes.is_empty()
        || functions
            .iter()
            .any(|function| matches!(function.returns.kind.as_str(), "string" | "c_string"))
        || classes.iter().any(|class| {
            class
                .methods
                .iter()
                .any(|function| matches!(function.returns.kind.as_str(), "string" | "c_string"))
        });
    let requires_unsafe = functions
        .iter()
        .any(|function| {
            has_string_params(function.params.iter())
                || function.returns.kind == "pointer"
        })
        || classes.iter().any(|class| {
            has_string_params(class.constructor.params.iter())
                || class.methods.iter().any(|function| {
                    has_string_params(function.params.iter().skip(1))
                        || function.returns.kind == "pointer"
                })
        });
    let requires_sync = !callback_usages.is_empty();

    let mut out = String::new();
    out.push_str(&format!("package {}\n\n", package_name));
    if requires_cgo {
        out.push_str("/*\n");
        out.push_str("#include <stdlib.h>\n");
        out.push_str(&format!(
            "#include \"{}\"\n",
            config.raw_include_for_go(&config.output.header)
        ));
        out.push_str("*/\n");
        out.push_str("import \"C\"\n\n");
    }
    if requires_errors {
        out.push_str("import \"errors\"\n\n");
    }
    if requires_unsafe {
        out.push_str("import \"unsafe\"\n\n");
    }
    if requires_sync {
        out.push_str("import \"sync\"\n\n");
    }

    for item in enums {
        out.push_str(&render_go_enum(item));
        out.push('\n');
    }
    for callback in used_callbacks(callback_usages) {
        out.push_str(&render_callback_type(callback));
        out.push('\n');
    }
    for usage in callback_usages {
        out.push_str(&render_callback_registry(usage));
        out.push('\n');
        out.push_str(&render_callback_export(usage));
        out.push('\n');
    }

    for function in functions {
        out.push_str(&render_free_function(config, function));
        out.push('\n');
    }

    for class in classes {
        out.push_str(&render_facade_class(class));
        out.push('\n');
        out.push_str(&render_facade_constructor(config, class));
        out.push('\n');
        out.push_str(&render_facade_close(class));
        out.push('\n');
        for method in &class.methods {
            out.push_str(&render_general_api_method(config, class, method));
            out.push('\n');
        }
    }

    out
}

fn render_go_enum(item: &IrEnum) -> String {
    let name = leaf_cpp_name(&item.cpp_name);
    let mut out = String::new();
    out.push_str(&format!("type {} int64\n\n", name));
    out.push_str("const (\n");
    for variant in &item.variants {
        let value = variant.value.as_deref().unwrap_or("0");
        out.push_str(&format!("    {} {} = {}\n", variant.name, name, value));
    }
    out.push_str(")\n");
    out
}

fn collect_callback_usages<'a>(
    functions: &[&'a IrFunction],
    classes: &[AnalyzedFacadeClass<'a>],
    ir: &'a IrModule,
) -> Vec<CallbackUsage<'a>> {
    let callbacks = ir
        .callbacks
        .iter()
        .map(|callback| (callback.name.as_str(), callback))
        .collect::<BTreeMap<_, _>>();
    let mut usages = Vec::new();

    for function in functions {
        usages.extend(callback_usages_for_function(function, &callbacks));
    }
    for class in classes {
        for function in &class.methods {
            usages.extend(callback_usages_for_function(function, &callbacks));
        }
    }

    usages
}

fn callback_usages_for_function<'a>(
    function: &'a IrFunction,
    callbacks: &BTreeMap<&str, &'a IrCallback>,
) -> Vec<CallbackUsage<'a>> {
    function
        .params
        .iter()
        .enumerate()
        .filter_map(|(index, param)| {
            (param.ty.kind == "callback").then(|| {
                callbacks
                    .get(param.ty.cpp_type.as_str())
                    .map(|callback| CallbackUsage {
                        callback,
                        function,
                        param_index: index,
                    })
            })?
        })
        .collect()
}

fn used_callbacks<'a>(usages: &'a [CallbackUsage<'a>]) -> Vec<&'a IrCallback> {
    let mut seen = BTreeMap::<String, &'a IrCallback>::new();
    for usage in usages {
        seen.entry(usage.callback.name.clone())
            .or_insert(usage.callback);
    }
    seen.into_values().collect()
}

fn render_callback_type(callback: &IrCallback) -> String {
    let params = callback
        .params
        .iter()
        .map(|param| {
            format!(
                "{} {}",
                param.name,
                callback_go_type(&param.ty)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let returns = if callback.returns.kind == "void" {
        String::new()
    } else {
        format!(" {}", callback_go_type(&callback.returns))
    };
    format!("type {} func({}){}\n", callback.name, params, returns)
}

fn render_callback_registry(usage: &CallbackUsage<'_>) -> String {
    format!(
        "var {} struct {{\n    mu sync.RWMutex\n    fn {}\n}}\n",
        callback_state_name(usage),
        usage.callback.name
    )
}

fn render_callback_export(usage: &CallbackUsage<'_>) -> String {
    let params = usage
        .callback
        .params
        .iter()
        .map(|param| {
            format!(
                "{} {}",
                param.name,
                callback_cgo_param_type(&param.ty)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = String::new();
    out.push_str(&format!("//export {}\n", callback_go_export_name(usage)));
    out.push_str(&format!(
        "func {}({})",
        callback_go_export_name(usage),
        params
    ));
    if usage.callback.returns.kind != "void" {
        out.push_str(&format!(
            " {}",
            callback_cgo_return_type(&usage.callback.returns)
        ));
    }
    out.push_str(" {\n");
    out.push_str(&format!(
        "    {}.mu.RLock()\n    fn := {}.fn\n    {}.mu.RUnlock()\n    if fn == nil {{\n",
        callback_state_name(usage),
        callback_state_name(usage),
        callback_state_name(usage)
    ));
    if usage.callback.returns.kind == "void" {
        out.push_str("        return\n");
    } else {
        out.push_str(&format!(
            "        return {}\n",
            zero_value_for_go_type(go_type_for_ir(&usage.callback.returns).unwrap_or("int"))
        ));
    }
    out.push_str("    }\n");
    let args = usage
        .callback
        .params
        .iter()
        .map(|param| render_callback_go_arg(&param.ty, &param.name))
        .collect::<Vec<_>>()
        .join(", ");
    if usage.callback.returns.kind == "void" {
        out.push_str(&format!("    fn({})\n", args));
    } else {
        out.push_str(&format!(
            "    return {}(fn({}))\n",
            callback_cgo_return_type(&usage.callback.returns),
            args
        ));
    }
    out.push_str("}\n");
    out
}

fn render_facade_class(class: &AnalyzedFacadeClass<'_>) -> String {
    format!(
        "type {} struct {{\n    ptr *C.{}\n}}\n",
        class.go_name, class.handle_name
    )
}

fn render_facade_constructor(config: &Config, class: &AnalyzedFacadeClass<'_>) -> String {
    let constructor_params = class.constructor.params.iter().collect::<Vec<_>>();
    let params = render_param_list(config, &constructor_params);
    let prep = render_call_prep(config, &constructor_params);

    let mut out = format!(
        "func New{}({}) (*{}, error) {{\n",
        class.go_name, params, class.go_name
    );
    for line in prep.setup_lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    for line in prep.defer_lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str(&format!(
        "    ptr := C.{}({})\n",
        class.constructor.name,
        prep.args.join(", "),
    ));
    for line in prep.post_call_lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str(&format!(
        "    if ptr == nil {{\n        return nil, errors.New(\"wrapper returned nil facade handle\")\n    }}\n    return &{}{{ptr: ptr}}, nil\n}}\n",
        class.go_name
    ));
    out
}

fn render_facade_close(class: &AnalyzedFacadeClass<'_>) -> String {
    let receiver = receiver_name(&class.go_name);
    format!(
        "func ({} *{}) Close() {{\n    if {} == nil || {}.ptr == nil {{\n        return\n    }}\n    C.{}({}.ptr)\n    {}.ptr = nil\n}}\n",
        receiver, class.go_name, receiver, receiver, class.destructor.name, receiver, receiver,
    )
}

fn render_general_api_method(
    config: &Config,
    class: &AnalyzedFacadeClass<'_>,
    function: &IrFunction,
) -> String {
    if has_callback_param(function.params.iter().skip(1)) {
        return render_callback_method(config, class, function);
    }
    let receiver = receiver_name(&class.go_name);
    let method_params = function.params.iter().skip(1).collect::<Vec<_>>();
    let params = render_param_list(config, &method_params);
    let prep = render_call_prep(config, &method_params);
    let call = format!(
        "C.{}({})",
        function.name,
        std::iter::once(format!("{receiver}.ptr"))
            .chain(prep.args)
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut out = String::new();
    out.push_str(&format!(
        "func ({} *{}) {}({})",
        receiver,
        class.go_name,
        go_method_export_name(function),
        params
    ));
    match function.returns.kind.as_str() {
        "void" => out.push_str(" {\n"),
        "string" | "c_string" => out.push_str(" (string, error) {\n"),
        "pointer" => out.push_str(&format!(
            " {} {{\n",
            go_pointer_return_type(&function.returns).unwrap()
        )),
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
        "pointer" => out.push_str("        return nil\n"),
        _ => out.push_str(&format!(
            "        return {}\n",
            zero_value_for_go_type(go_type_for_ir(&function.returns).unwrap())
        )),
    }
    out.push_str("    }\n");
    for line in prep.setup_lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    for line in prep.defer_lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    match function.returns.kind.as_str() {
        "void" => {
            out.push_str(&format!("    {}\n", call));
            for line in prep.post_call_lines {
                out.push_str("    ");
                out.push_str(&line);
                out.push('\n');
            }
        }
        "string" => {
            out.push_str(&format!("    raw := {}\n", call));
            for line in prep.post_call_lines {
                out.push_str("    ");
                out.push_str(&line);
                out.push('\n');
            }
            out.push_str(&format!(
                "    if raw == nil {{\n        return \"\", errors.New(\"wrapper returned nil string\")\n    }}\n    defer C.{}_string_free(raw)\n    return C.GoString(raw), nil\n",
                config.naming.prefix
            ));
        }
        "c_string" => {
            out.push_str(&format!("    raw := {}\n", call));
            for line in prep.post_call_lines {
                out.push_str("    ");
                out.push_str(&line);
                out.push('\n');
            }
            out.push_str(
                "    if raw == nil {\n        return \"\", errors.New(\"wrapper returned nil string\")\n    }\n    return C.GoString(raw), nil\n",
            );
        }
        "pointer" => {
            let go_type = go_pointer_return_type(&function.returns).unwrap();
            out.push_str(&format!("    raw := {}\n", call));
            for line in prep.post_call_lines {
                out.push_str("    ");
                out.push_str(&line);
                out.push('\n');
            }
            out.push_str(&format!(
                "    return ({})(unsafe.Pointer(raw))\n",
                go_type
            ));
        }
        _ => {
            let go_type = go_type_for_ir(&function.returns).unwrap();
            if go_type == "bool" {
                out.push_str(&format!("    result := {}\n", call));
                for line in prep.post_call_lines {
                    out.push_str("    ");
                    out.push_str(&line);
                    out.push('\n');
                }
                out.push_str("    return bool(result)\n");
            } else {
                out.push_str(&format!("    return {}({})\n", go_type, call));
            }
        }
    }
    out.push_str("}\n");
    out
}

fn render_free_function(config: &Config, function: &IrFunction) -> String {
    if has_callback_param(function.params.iter()) {
        return render_callback_free_function(config, function);
    }
    let params_list = function.params.iter().collect::<Vec<_>>();
    let params = render_param_list(config, &params_list);
    let prep = render_call_prep(config, &params_list);
    let call = format!("C.{}({})", function.name, prep.args.join(", "));
    let go_name = go_facade_export_name(function);

    match function.returns.kind.as_str() {
        "void" => {
            let mut out = format!("func {}({}) {{\n", go_name, params);
            for line in prep.setup_lines {
                out.push_str("    ");
                out.push_str(&line);
                out.push('\n');
            }
            for line in prep.defer_lines {
                out.push_str("    ");
                out.push_str(&line);
                out.push('\n');
            }
            out.push_str(&format!("    {}\n", call));
            for line in prep.post_call_lines {
                out.push_str("    ");
                out.push_str(&line);
                out.push('\n');
            }
            out.push_str("}\n");
            out
        }
        "string" => {
            let mut out = format!("func {}({}) (string, error) {{\n", go_name, params);
            out.push_str(&indented_lines(&prep.setup_lines));
            out.push_str(&indented_lines(&prep.defer_lines));
            out.push_str(&format!("    raw := {}\n", call));
            out.push_str(&indented_lines(&prep.post_call_lines));
            out.push_str(&format!(
                "    if raw == nil {{\n        return \"\", errors.New(\"wrapper returned nil string\")\n    }}\n    defer C.{}_string_free(raw)\n    return C.GoString(raw), nil\n}}\n",
                config.naming.prefix
            ));
            out
        }
        "c_string" => {
            let mut out = format!("func {}({}) (string, error) {{\n", go_name, params);
            out.push_str(&indented_lines(&prep.setup_lines));
            out.push_str(&indented_lines(&prep.defer_lines));
            out.push_str(&format!("    raw := {}\n", call));
            out.push_str(&indented_lines(&prep.post_call_lines));
            out.push_str(
                "    if raw == nil {\n        return \"\", errors.New(\"wrapper returned nil string\")\n    }\n    return C.GoString(raw), nil\n}\n",
            );
            out
        }
        "pointer" => {
            let go_type = go_pointer_return_type(&function.returns).unwrap();
            let mut out = format!("func {}({}) {} {{\n", go_name, params, go_type);
            out.push_str(&indented_lines(&prep.setup_lines));
            out.push_str(&indented_lines(&prep.defer_lines));
            out.push_str(&format!("    raw := {}\n", call));
            out.push_str(&indented_lines(&prep.post_call_lines));
            out.push_str(&format!("    return ({})(unsafe.Pointer(raw))\n}}\n", go_type));
            out
        }
        _ => {
            let go_type = go_type_for_ir(&function.returns).unwrap();
            let mut out = format!(
                "func {}({}) {} {{\n",
                go_name,
                params,
                go_type
            );
            out.push_str(&indented_lines(&prep.setup_lines));
            out.push_str(&indented_lines(&prep.defer_lines));
            if go_type == "bool" {
                out.push_str(&format!("    result := {}\n", call));
                out.push_str(&indented_lines(&prep.post_call_lines));
                out.push_str("    return bool(result)\n}\n");
            } else {
                out.push_str(&format!("    return {}({})\n}}\n", go_type, call));
            }
            out
        }
    }
}

fn render_callback_method(
    config: &Config,
    class: &AnalyzedFacadeClass<'_>,
    function: &IrFunction,
) -> String {
    let receiver = receiver_name(&class.go_name);
    let method_params = function.params.iter().skip(1).collect::<Vec<_>>();
    let params = render_param_list(config, &method_params);
    let prep = render_callback_call_prep(config, function, &method_params);
    let call = format!(
        "C.{}_bridge({})",
        function.name,
        std::iter::once(format!("{receiver}.ptr"))
            .chain(prep.args)
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut out = String::new();
    out.push_str(&format!(
        "func ({} *{}) {}({})",
        receiver,
        class.go_name,
        go_method_export_name(function),
        params
    ));
    match function.returns.kind.as_str() {
        "void" => out.push_str(" {\n"),
        "string" | "c_string" => out.push_str(" (string, error) {\n"),
        "pointer" => out.push_str(&format!(
            " {} {{\n",
            go_pointer_return_type(&function.returns).unwrap()
        )),
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
        "pointer" => out.push_str("        return nil\n"),
        _ => out.push_str(&format!(
            "        return {}\n",
            zero_value_for_go_type(go_type_for_ir(&function.returns).unwrap())
        )),
    }
    out.push_str("    }\n");
    out.push_str(&indented_lines(&prep.setup_lines));
    out.push_str(&indented_lines(&prep.defer_lines));
    match function.returns.kind.as_str() {
        "void" => out.push_str(&format!("    {}\n", call)),
        "string" => {
            out.push_str(&format!("    raw := {}\n", call));
            out.push_str(&format!(
                "    if raw == nil {{\n        return \"\", errors.New(\"wrapper returned nil string\")\n    }}\n    defer C.{}_string_free(raw)\n    return C.GoString(raw), nil\n",
                config.naming.prefix
            ));
        }
        "c_string" => {
            out.push_str(&format!("    raw := {}\n", call));
            out.push_str(
                "    if raw == nil {\n        return \"\", errors.New(\"wrapper returned nil string\")\n    }\n    return C.GoString(raw), nil\n",
            );
        }
        "pointer" => {
            let go_type = go_pointer_return_type(&function.returns).unwrap();
            out.push_str(&format!("    raw := {}\n", call));
            out.push_str(&format!(
                "    return ({})(unsafe.Pointer(raw))\n",
                go_type
            ));
        }
        _ => {
            out.push_str(&format!("    result := {}\n", call));
            out.push_str(&format!(
                "    return {}(result)\n",
                go_type_for_ir(&function.returns).unwrap()
            ));
        }
    }
    out.push_str("}\n");
    out
}

fn render_callback_free_function(config: &Config, function: &IrFunction) -> String {
    let params_list = function.params.iter().collect::<Vec<_>>();
    let params = render_param_list(config, &params_list);
    let prep = render_callback_call_prep(config, function, &params_list);
    let call = format!("C.{}_bridge({})", function.name, prep.args.join(", "));
    let go_name = go_facade_export_name(function);

    let mut out = format!("func {}({})", go_name, params);
    match function.returns.kind.as_str() {
        "void" => out.push_str(" {\n"),
        "string" | "c_string" => out.push_str(" (string, error) {\n"),
        "pointer" => out.push_str(&format!(
            " {} {{\n",
            go_pointer_return_type(&function.returns).unwrap()
        )),
        _ => out.push_str(&format!(
            " {} {{\n",
            go_type_for_ir(&function.returns).unwrap()
        )),
    }
    out.push_str(&indented_lines(&prep.setup_lines));
    out.push_str(&indented_lines(&prep.defer_lines));
    match function.returns.kind.as_str() {
        "void" => out.push_str(&format!("    {}\n", call)),
        "string" => {
            out.push_str(&format!("    raw := {}\n", call));
            out.push_str(&format!(
                "    if raw == nil {{\n        return \"\", errors.New(\"wrapper returned nil string\")\n    }}\n    defer C.{}_string_free(raw)\n    return C.GoString(raw), nil\n",
                config.naming.prefix
            ));
        }
        "c_string" => {
            out.push_str(&format!("    raw := {}\n", call));
            out.push_str(
                "    if raw == nil {\n        return \"\", errors.New(\"wrapper returned nil string\")\n    }\n    return C.GoString(raw), nil\n",
            );
        }
        "pointer" => {
            let go_type = go_pointer_return_type(&function.returns).unwrap();
            out.push_str(&format!("    raw := {}\n", call));
            out.push_str(&format!(
                "    return ({})(unsafe.Pointer(raw))\n",
                go_type
            ));
        }
        _ => {
            out.push_str(&format!("    result := {}\n", call));
            out.push_str(&format!(
                "    return {}(result)\n",
                go_type_for_ir(&function.returns).unwrap()
            ));
        }
    }
    out.push_str("}\n");
    out
}

fn render_callback_call_prep(
    config: &Config,
    function: &IrFunction,
    params: &[&crate::ir::IrParam],
) -> RenderedCallPrep {
    let mut prep = RenderedCallPrep::default();

    for (index, param) in params.iter().enumerate() {
        if param.ty.kind == "callback" {
            let state = callback_state_name_from_function(function, index);
            prep.setup_lines.push(format!("{state}.mu.Lock()"));
            prep.setup_lines
                .push(format!("{state}.fn = {}", param.name));
            prep.setup_lines.push(format!("{state}.mu.Unlock()"));
            prep.args.push(format!("C.bool({} != nil)", param.name));
            continue;
        }

        match param.ty.kind.as_str() {
            "string" | "c_string" => {
                let c_name = format!("cArg{index}");
                prep.setup_lines
                    .push(format!("{c_name} := C.CString({})", param.name));
                prep.defer_lines
                    .push(format!("defer C.free(unsafe.Pointer({c_name}))"));
                prep.args.push(c_name);
            }
            "reference" => render_reference_arg(&mut prep, &param.ty, &param.name, index),
            "model_reference" | "model_pointer" => {
                render_model_arg(config, &mut prep, &param.ty, &param.name, index)
            }
            _ => prep.args.push(render_c_arg(&param.ty, &param.name)),
        }
    }

    prep
}

fn render_param_list(config: &Config, params: &[&crate::ir::IrParam]) -> String {
    params
        .iter()
        .map(|param| {
            format!(
                "{} {}",
                param.name,
                go_param_type(config, &param.ty).unwrap()
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_call_prep(config: &Config, params: &[&crate::ir::IrParam]) -> RenderedCallPrep {
    let mut prep = RenderedCallPrep::default();

    for (index, param) in params.iter().enumerate() {
        match param.ty.kind.as_str() {
            "string" | "c_string" => {
                let c_name = format!("cArg{index}");
                prep.setup_lines
                    .push(format!("{c_name} := C.CString({})", param.name));
                prep.defer_lines
                    .push(format!("defer C.free(unsafe.Pointer({c_name}))"));
                prep.args.push(c_name);
            }
            "reference" => render_reference_arg(&mut prep, &param.ty, &param.name, index),
            "model_reference" | "model_pointer" => {
                render_model_arg(config, &mut prep, &param.ty, &param.name, index)
            }
            _ => prep.args.push(render_c_arg(&param.ty, &param.name)),
        }
    }

    prep
}

fn render_model_handle_arg(config: &Config, ty: &IrType, name: &str) -> Option<String> {
    let projection = config.known_model_projection(&ty.cpp_type)?;
    if ty.kind == "model_pointer" {
        Some(format!("optional{}Handle({})", projection.go_name, name))
    } else {
        Some(format!("require{}Handle({})", projection.go_name, name))
    }
}

fn render_reference_arg(prep: &mut RenderedCallPrep, ty: &IrType, name: &str, index: usize) {
    let go_type =
        go_type_for_reference(ty).expect("primitive references must be filtered before rendering");
    let c_name = format!("cArg{index}");
    prep.setup_lines.push(format!("if {name} == nil {{"));
    prep.setup_lines
        .push(format!("    panic(\"{name} reference is nil\")"));
    prep.setup_lines.push("}".to_string());
    prep.setup_lines
        .push(format!("{c_name} := {}(*{})", cgo_cast_type(ty), name));
    prep.post_call_lines
        .push(format!("*{} = {}({})", name, go_type, c_name));
    prep.args.push(format!("&{c_name}"));
}

fn render_c_arg(ty: &IrType, name: &str) -> String {
    format!("{}({})", cgo_cast_type(ty), name)
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

fn render_model_arg(
    config: &Config,
    prep: &mut RenderedCallPrep,
    ty: &IrType,
    name: &str,
    index: usize,
) {
    if let Some(handle_arg) = render_model_handle_arg(config, ty, name) {
        prep.args.push(handle_arg);
        return;
    }
    let handle = ty.handle.as_deref().unwrap_or("void");
    let c_name = format!("cArg{index}");
    prep.setup_lines.push(format!("var {c_name} *C.{handle}"));
    if ty.kind == "model_reference" {
        prep.setup_lines.push(format!("if {name} == nil {{"));
        prep.setup_lines.push(
            "    panic(\"reference facade/model argument cannot be nil\")".to_string(),
        );
        prep.setup_lines.push("}".to_string());
    }
    prep.setup_lines.push(format!("if {name} != nil {{"));
    prep.setup_lines.push(format!("    {c_name} = {name}.ptr"));
    prep.setup_lines.push("}".to_string());
    prep.args.push(c_name);
}

fn has_callback_param<'a>(mut params: impl Iterator<Item = &'a crate::ir::IrParam>) -> bool {
    params.any(|param| param.ty.kind == "callback")
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

fn ensure_unique_method_exports(owner: &str, methods: &[&IrFunction]) -> Result<()> {
    let mut by_export = BTreeMap::<String, Vec<String>>::new();
    for function in methods {
        by_export
            .entry(go_method_export_name(function))
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
                "Go facade method `{owner}.{export}` collides for: {}",
                names.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    bail!("facade export collision detected: {detail}");
}

fn free_function_supported(config: &Config, function: &IrFunction) -> bool {
    go_return_supported(&function.returns)
        && function
            .params
            .iter()
            .all(|param| go_param_supported(config, &param.ty))
}

fn method_supported(config: &Config, function: &IrFunction) -> bool {
    go_return_supported(&function.returns)
        && function
            .params
            .iter()
            .skip(1)
            .all(|param| go_param_supported(config, &param.ty))
}

fn go_param_supported(config: &Config, ty: &IrType) -> bool {
    go_param_type(config, ty).is_some()
}

fn go_param_type(config: &Config, ty: &IrType) -> Option<String> {
    match ty.kind.as_str() {
        "string" | "c_string" => Some("string".to_string()),
        "primitive" => go_type_for_ir(ty).map(str::to_string),
        "reference" => go_type_for_reference(ty).map(|go_type| format!("*{go_type}")),
        "callback" => Some(leaf_cpp_name(&ty.cpp_type)),
        "model_reference" | "model_pointer" => config
            .known_model_projection(&ty.cpp_type)
            .map(|projection| format!("*{}", projection.go_name))
            .or_else(|| Some(format!("*{}", leaf_cpp_name(&base_model_cpp_type(&ty.cpp_type))))),
        _ => None,
    }
}

fn go_return_supported(ty: &IrType) -> bool {
    ty.kind == "void"
        || matches!(ty.kind.as_str(), "string" | "c_string")
        || (ty.kind == "primitive" && go_type_for_ir(ty).is_some())
        || (ty.kind == "pointer" && go_pointer_return_type(ty).is_some())
}

fn go_pointer_return_type(ty: &IrType) -> Option<String> {
    if ty.kind != "pointer" {
        return None;
    }
    let base = ty.cpp_type.trim_end_matches('*').trim();
    primitive_go_type(base)
        .or_else(|| primitive_go_type(ty.c_type.trim_end_matches('*').trim()))
        .map(|go_type| format!("*{go_type}"))
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
        "primitive" => primitive_go_type(&ty.cpp_type).or_else(|| primitive_go_type(&ty.c_type)),
        _ => None,
    }
}

fn go_type_for_reference(ty: &IrType) -> Option<&'static str> {
    if ty.kind != "reference" {
        return None;
    }

    primitive_go_type(&ty.cpp_type).or_else(|| primitive_go_type(&ty.c_type))
}

fn cgo_cast_type(ty: &IrType) -> &'static str {
    primitive_cgo_cast_type(&ty.cpp_type).unwrap_or_else(|| {
        primitive_cgo_cast_type(&ty.c_type).unwrap_or("C.int")
    })
}

fn primitive_go_type(value: &str) -> Option<&'static str> {
    match normalize_type_key(value).as_str() {
        "bool" => Some("bool"),
        "float" => Some("float32"),
        "double" => Some("float64"),
        "int8" | "int8_t" | "signedchar" => Some("int8"),
        "int16" | "int16_t" | "short" => Some("int16"),
        "int32" | "int32_t" => Some("int32"),
        "int64" | "int64_t" | "long" | "longlong" => Some("int64"),
        "uint8" | "uint8_t" | "unsignedchar" => Some("uint8"),
        "uint16" | "uint16_t" | "unsignedshort" => Some("uint16"),
        "uint32" | "uint32_t" | "unsignedint" | "unsigned" => Some("uint32"),
        "int" => Some("int"),
        "uint64" | "uint64_t" | "unsignedlong" | "unsignedlonglong" => Some("uint64"),
        "size_t" => Some("uintptr"),
        _ => None,
    }
}

fn primitive_cgo_cast_type(value: &str) -> Option<&'static str> {
    match normalize_type_key(value).as_str() {
        "bool" => Some("C.bool"),
        "float" => Some("C.float"),
        "double" => Some("C.double"),
        "int8" | "int8_t" | "signedchar" => Some("C.int8_t"),
        "int16" | "int16_t" | "short" => Some("C.int16_t"),
        "int32" | "int32_t" => Some("C.int32_t"),
        "int64" | "int64_t" => Some("C.int64_t"),
        "uint8" | "uint8_t" | "unsignedchar" => Some("C.uint8_t"),
        "uint16" | "uint16_t" | "unsignedshort" => Some("C.uint16_t"),
        "uint32" | "uint32_t" | "unsignedint" | "unsigned" => Some("C.uint32_t"),
        "uint64" | "uint64_t" => Some("C.uint64_t"),
        "int" => Some("C.int"),
        "long" => Some("C.long"),
        "size_t" => Some("C.size_t"),
        _ => None,
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
    let mut out = String::new();
    for (index, segment) in value.split('_').filter(|segment| !segment.is_empty()).enumerate() {
        if index > 0
            && segment
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_digit())
            && !out.is_empty()
        {
            out.push('_');
        }
        for token in split_pascal_tokens(segment)
            .into_iter()
            .filter(|token| !token.is_empty())
        {
            let mut chars = token.chars();
            let Some(first) = chars.next() else {
                continue;
            };
            out.push(first.to_ascii_uppercase());
            out.push_str(&chars.collect::<String>());
        }
    }
    out
}

fn go_facade_export_name(function: &IrFunction) -> String {
    let base = go_export_name(&leaf_cpp_name(&function.cpp_name));
    if !function.name.contains("__") {
        return base;
    }

    format!("{base}{}", go_overload_suffix(function))
}

fn go_method_export_name(function: &IrFunction) -> String {
    let base = go_export_name(method_name(function));
    if !function.name.contains("__") {
        return base;
    }

    format!("{base}{}", go_overload_suffix(function))
}

fn go_overload_suffix(function: &IrFunction) -> String {
    let params = if function.method_of.is_some() {
        function.params.iter().skip(1).collect::<Vec<_>>()
    } else {
        function.params.iter().collect::<Vec<_>>()
    };

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
        "callback" => format!("{}Callback", go_export_name(&leaf_cpp_name(&ty.cpp_type))),
        "string" | "c_string" => string_overload_token(ty),
        "primitive" => primitive_overload_token(ty),
        "model_reference" => format!(
            "{}Ref",
            go_export_name(&flatten_qualified_cpp_name(&base_model_cpp_type(
                &ty.cpp_type
            )))
        ),
        "model_pointer" => model_pointer_overload_token(ty),
        _ => go_export_name(&sanitize_go_token(&ty.cpp_type)),
    }
}

fn model_pointer_overload_token(ty: &IrType) -> String {
    let base = go_export_name(&flatten_qualified_cpp_name(&base_model_cpp_type(
        &ty.cpp_type,
    )));
    let depth = model_pointer_depth(ty);
    format!("{base}{}", "Ptr".repeat(depth.max(1)))
}

fn model_pointer_depth(ty: &IrType) -> usize {
    let cpp_depth = ty.cpp_type.chars().filter(|ch| *ch == '*').count();
    if cpp_depth > 0 {
        return cpp_depth;
    }
    ty.c_type.chars().filter(|ch| *ch == '*').count().max(1)
}

fn primitive_overload_token(ty: &IrType) -> String {
    let cpp_key = normalize_type_key(&ty.cpp_type);
    let c_key = normalize_type_key(&ty.c_type);
    if cpp_key != c_key && !is_builtin_primitive_key(&cpp_key) {
        return go_export_name(&sanitize_go_token(&ty.cpp_type));
    }
    go_type_for_ir(ty)
        .map(go_export_name)
        .unwrap_or_else(|| go_export_name(&sanitize_go_token(&ty.cpp_type)))
}

fn string_overload_token(ty: &IrType) -> String {
    let cpp_key = normalize_type_key(&ty.cpp_type);
    let c_key = normalize_type_key(&ty.c_type);
    if cpp_key != c_key && !cpp_key.is_empty() {
        return go_export_name(&sanitize_go_token(&ty.cpp_type));
    }
    "String".to_string()
}

fn is_builtin_primitive_key(value: &str) -> bool {
    matches!(
        value,
        "bool"
            | "float"
            | "double"
            | "int8"
            | "int8_t"
            | "signedchar"
            | "int16"
            | "int16_t"
            | "short"
            | "int32"
            | "int32_t"
            | "int"
            | "int64"
            | "int64_t"
            | "long"
            | "longlong"
            | "uint8"
            | "uint8_t"
            | "unsignedchar"
            | "uint16"
            | "uint16_t"
            | "unsignedshort"
            | "uint32"
            | "uint32_t"
            | "unsignedint"
            | "unsigned"
            | "uint64"
            | "uint64_t"
            | "unsignedlong"
            | "unsignedlonglong"
            | "size_t"
    )
}

fn callback_state_name(usage: &CallbackUsage<'_>) -> String {
    callback_state_name_from_function(usage.function, usage.param_index)
}

fn callback_state_name_from_function(function: &IrFunction, index: usize) -> String {
    format!("{}_cb{}", sanitize_go_token(&function.name), index)
}

fn callback_go_export_name(usage: &CallbackUsage<'_>) -> String {
    format!("go_{}_cb{}", sanitize_go_token(&usage.function.name), usage.param_index)
}

fn callback_cgo_param_type(ty: &IrType) -> &'static str {
    match ty.kind.as_str() {
        "string" | "c_string" => "*C.char",
        _ => cgo_cast_type_from_c_type(&ty.c_type),
    }
}

fn callback_cgo_return_type(ty: &IrType) -> &'static str {
    cgo_cast_type_from_c_type(&ty.c_type)
}

fn render_callback_go_arg(ty: &IrType, name: &str) -> String {
    match ty.kind.as_str() {
        "string" | "c_string" => format!("C.GoString({name})"),
        _ => format!("{}({})", callback_go_type(ty), name),
    }
}

fn callback_go_type(ty: &IrType) -> &'static str {
    go_type_for_ir(ty).unwrap_or_else(|| go_type_from_c_type(&ty.c_type))
}

fn go_type_from_c_type(c_type: &str) -> &'static str {
    match normalize_type_key(c_type).as_str() {
        "bool" => "bool",
        "float" => "float32",
        "double" => "float64",
        "int8" | "int8_t" => "int8",
        "int16" | "int16_t" | "short" => "int16",
        "int32" | "int32_t" | "int" => "int32",
        "int64" | "int64_t" | "long" => "int64",
        "uint8" | "uint8_t" => "uint8",
        "uint16" | "uint16_t" => "uint16",
        "uint32" | "uint32_t" | "unsignedint" | "unsigned" => "uint32",
        "uint64" | "uint64_t" | "unsignedlong" | "unsignedlonglong" => "uint64",
        "size_t" => "uintptr",
        _ => "int",
    }
}

fn cgo_cast_type_from_c_type(c_type: &str) -> &'static str {
    match normalize_type_key(c_type).as_str() {
        "bool" => "C.bool",
        "float" => "C.float",
        "double" => "C.double",
        "int8" | "int8_t" => "C.int8_t",
        "int16" | "int16_t" => "C.int16_t",
        "int32" | "int32_t" => "C.int32_t",
        "int64" | "int64_t" => "C.int64_t",
        "uint8" | "uint8_t" => "C.uint8_t",
        "uint16" | "uint16_t" => "C.uint16_t",
        "uint32" | "uint32_t" | "unsignedint" | "unsigned" => "C.uint32_t",
        "uint64" | "uint64_t" => "C.uint64_t",
        "short" => "C.short",
        "long" => "C.long",
        "size_t" => "C.size_t",
        _ => "C.int",
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

#[derive(Debug, Clone)]
struct ModelProjection {
    cpp_type: String,
    go_name: String,
    handle_name: String,
    constructor_symbol: String,
    destructor_symbol: String,
    fields: Vec<ModelProjectionField>,
}

#[derive(Debug, Clone)]
struct ModelProjectionField {
    go_name: String,
    go_type: String,
    getter_symbol: String,
    setter_symbol: String,
    return_kind: String,
}

pub fn collect_known_model_projections(
    config: &Config,
    ir: &IrModule,
) -> Result<Vec<KnownModelProjection>> {
    Ok(build_all_model_projections(ir)?
        .into_iter()
        .map(|projection| KnownModelProjection {
            cpp_type: projection.cpp_type,
            handle_name: projection.handle_name,
            go_name: projection.go_name,
            output_header: config.raw_include_for_go(&config.output.header),
            constructor_symbol: projection.constructor_symbol,
            destructor_symbol: Some(projection.destructor_symbol),
            fields: projection
                .fields
                .into_iter()
                .map(|field| KnownModelField {
                    go_name: field.go_name,
                    go_type: field.go_type,
                    getter_symbol: field.getter_symbol,
                    setter_symbol: field.setter_symbol,
                    return_kind: field.return_kind,
                })
                .collect(),
        })
        .collect())
}

fn build_all_model_projections(ir: &IrModule) -> Result<Vec<ModelProjection>> {
    let constructors = ir
        .functions
        .iter()
        .filter(|function| function.kind == "constructor")
        .filter_map(|function| {
            function
                .owner_cpp_type
                .as_deref()
                .map(|owner| (owner.to_string(), function.name.clone()))
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
                .map(|owner| (owner.to_string(), function.name.clone()))
        })
        .collect::<BTreeMap<_, _>>();

    let mut methods_by_owner = BTreeMap::<String, Vec<&IrFunction>>::new();
    for function in ir
        .functions
        .iter()
        .filter(|function| function.kind == "method")
    {
        let Some(owner) = &function.owner_cpp_type else {
            continue;
        };
        methods_by_owner
            .entry(owner.clone())
            .or_default()
            .push(function);
    }

    let mut projections = Vec::new();
    for (owner, class_methods) in methods_by_owner {
        if let Some(projection) = build_model_projection(
            &owner,
            &class_methods,
            constructors.get(&owner),
            destructors.get(&owner),
        )? {
            projections.push(projection);
        }
    }
    Ok(projections)
}

fn build_model_projection(
    owner: &str,
    class_methods: &[&IrFunction],
    constructor_symbol: Option<&String>,
    destructor_symbol: Option<&String>,
) -> Result<Option<ModelProjection>> {
    let setters = class_methods
        .iter()
        .filter_map(|function| setter_suffix(function).map(|suffix| (suffix.to_string(), *function)))
        .collect::<BTreeMap<_, _>>();

    let mut fields = Vec::new();
    let mut seen = BTreeMap::<String, ()>::new();
    for function in class_methods {
        let Some(suffix) = getter_suffix(function) else {
            continue;
        };
        let Some(setter) = setters.get(suffix) else {
            continue;
        };
        if seen.insert(suffix.to_string(), ()).is_some() {
            continue;
        }

        let Some(getter_ty) = go_type_for_ir(&function.returns) else {
            continue;
        };
        let Some(setter_param) = setter.params.get(1) else {
            bail!(
                "setter `{}` on `{owner}` is missing its value parameter",
                setter.cpp_name
            );
        };
        let Some(setter_ty) = go_type_for_ir(&setter_param.ty) else {
            continue;
        };

        if getter_ty != setter_ty {
            continue;
        }

        fields.push(ModelProjectionField {
            go_name: go_field_name(suffix),
            go_type: getter_ty.to_string(),
            getter_symbol: function.name.clone(),
            setter_symbol: setter.name.clone(),
            return_kind: function.returns.kind.clone(),
        });
    }

    if fields.is_empty() {
        return Ok(None);
    }

    let constructor_symbol = constructor_symbol
        .ok_or_else(|| anyhow::anyhow!("model projection `{owner}` is missing a constructor wrapper"))?;
    let destructor_symbol = destructor_symbol
        .ok_or_else(|| anyhow::anyhow!("model projection `{owner}` is missing a destructor wrapper"))?;

    Ok(Some(ModelProjection {
        cpp_type: owner.to_string(),
        go_name: leaf_cpp_name(owner),
        handle_name: format!("{}Handle", flatten_qualified_cpp_name(owner)),
        constructor_symbol: constructor_symbol.clone(),
        destructor_symbol: destructor_symbol.clone(),
        fields,
    }))
}

fn getter_suffix(function: &IrFunction) -> Option<&str> {
    if function.kind != "method" || function.params.len() != 1 || function.returns.kind == "void" {
        return None;
    }
    function
        .cpp_name
        .rsplit("::")
        .next()
        .and_then(|name| name.strip_prefix("Get"))
        .filter(|suffix| !suffix.is_empty())
}

fn setter_suffix(function: &IrFunction) -> Option<&str> {
    if function.kind != "method" || function.params.len() != 2 || function.returns.kind != "void" {
        return None;
    }
    function
        .cpp_name
        .rsplit("::")
        .next()
        .and_then(|name| name.strip_prefix("Set"))
        .filter(|suffix| !suffix.is_empty())
}

fn go_field_name(value: &str) -> String {
    value
        .split('_')
        .flat_map(split_pascal_tokens)
        .map(|token| match token.to_ascii_lowercase().as_str() {
            "id" => "ID".to_string(),
            "url" => "URL".to_string(),
            "db" => "DB".to_string(),
            "api" => "API".to_string(),
            "http" => "HTTP".to_string(),
            "https" => "HTTPS".to_string(),
            "json" => "JSON".to_string(),
            "xml" => "XML".to_string(),
            other if token.chars().all(|ch| ch.is_uppercase()) => other.to_ascii_uppercase(),
            _ => token,
        })
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{KnownModelField, KnownModelProjection},
        ir::IrParam,
    };

    fn test_config_with_known_model() -> Config {
        Config {
            known_model_projections: vec![KnownModelProjection {
                cpp_type: "ThingModel".to_string(),
                handle_name: "ThingModelHandle".to_string(),
                go_name: "ThingModel".to_string(),
                output_header: "raw/thing_model_wrapper.h".to_string(),
                constructor_symbol: "cgowrap_ThingModel_new".to_string(),
                destructor_symbol: Some("cgowrap_ThingModel_delete".to_string()),
                fields: vec![KnownModelField {
                    go_name: "Value".to_string(),
                    go_type: "int".to_string(),
                    getter_symbol: "cgowrap_ThingModel_GetValue".to_string(),
                    setter_symbol: "cgowrap_ThingModel_SetValue".to_string(),
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

    fn model_type(kind: &str, cpp_type: &str) -> IrType {
        IrType {
            kind: kind.to_string(),
            cpp_type: cpp_type.to_string(),
            c_type: format!("{cpp_type}Handle*"),
            handle: Some(format!("{cpp_type}Handle")),
        }
    }

    fn reference_type(cpp_type: &str, c_type: &str) -> IrType {
        IrType {
            kind: "reference".to_string(),
            cpp_type: cpp_type.to_string(),
            c_type: c_type.to_string(),
            handle: None,
        }
    }

    #[test]
    fn method_supports_known_model_reference_params() {
        let config = test_config_with_known_model();
        let function = IrFunction {
            name: "cgowrap_Api_GetThing".to_string(),
            kind: "method".to_string(),
            cpp_name: "Api::GetThing".to_string(),
            method_of: Some("Api".to_string()),
            owner_cpp_type: Some("Api".to_string()),
            is_const: Some(false),
            returns: primitive_type("bool", "bool"),
            params: vec![
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
                    ty: model_type("model_reference", "ThingModel"),
                },
                IrParam {
                    name: "id".to_string(),
                    ty: primitive_type("int", "int"),
                },
            ],
        };

        assert!(method_supported(&config, &function));
    }

    #[test]
    fn method_supports_unknown_model_params_as_handles() {
        let config = test_config_with_known_model();
        let function = IrFunction {
            name: "cgowrap_Api_GetThing".to_string(),
            kind: "method".to_string(),
            cpp_name: "Api::GetThing".to_string(),
            method_of: Some("Api".to_string()),
            owner_cpp_type: Some("Api".to_string()),
            is_const: Some(false),
            returns: primitive_type("bool", "bool"),
            params: vec![
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
                    name: "value".to_string(),
                    ty: model_type("model_reference", "UnknownThing"),
                },
            ],
        };

        assert!(method_supported(&config, &function));
    }

    #[test]
    fn method_supports_primitive_reference_and_known_model_params() {
        let config = test_config_with_known_model();
        let function = IrFunction {
            name: "cgowrap_Api_NextThing".to_string(),
            kind: "method".to_string(),
            cpp_name: "Api::NextThing".to_string(),
            method_of: Some("Api".to_string()),
            owner_cpp_type: Some("Api".to_string()),
            is_const: Some(false),
            returns: primitive_type("bool", "bool"),
            params: vec![
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
                    name: "pos".to_string(),
                    ty: reference_type("int32&", "int32_t*"),
                },
                IrParam {
                    name: "out".to_string(),
                    ty: model_type("model_reference", "ThingModel"),
                },
            ],
        };

        assert!(method_supported(&config, &function));
        assert_eq!(
            go_param_type(&config, &function.params[1].ty),
            Some("*int32".to_string())
        );
    }

    #[test]
    fn overload_tokens_distinguish_model_ref_and_ptr() {
        assert_eq!(
            go_overload_token(&model_type("model_reference", "ThingModel")),
            "ThingModelRef"
        );
        assert_eq!(
            go_overload_token(&model_type("model_pointer", "ThingModel")),
            "ThingModelPtr"
        );
        assert_eq!(
            go_overload_token(&IrType {
                kind: "model_pointer".to_string(),
                cpp_type: "ThingModel**".to_string(),
                c_type: "ThingModelHandle**".to_string(),
                handle: Some("ThingModelHandle".to_string()),
            }),
            "ThingModelPtrPtr"
        );
    }

    #[test]
    fn overload_tokens_preserve_typedef_identity_for_alias_backed_scalars() {
        assert_eq!(
            go_overload_token(&primitive_type("time_t", "int64_t")),
            "TimeT"
        );
        assert_eq!(
            go_overload_token(&primitive_type("uint32", "uint32_t")),
            "Uint32"
        );
        assert_eq!(
            go_overload_token(&IrType {
                kind: "c_string".to_string(),
                cpp_type: "NPCSTR".to_string(),
                c_type: "const char*".to_string(),
                handle: None,
            }),
            "NPCSTR"
        );
        assert_eq!(
            go_overload_token(&IrType {
                kind: "string".to_string(),
                cpp_type: "NPSTR".to_string(),
                c_type: "char*".to_string(),
                handle: None,
            }),
            "NPSTR"
        );
    }
}
