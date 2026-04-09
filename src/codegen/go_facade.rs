use std::{collections::{BTreeMap, BTreeSet}, path::Path};

use anyhow::{Result, bail};

use crate::{
    codegen::ir_norm,
    domain::kind::{IrFunctionKind, IrTypeKind},
    ir::{IrCallback, IrEnum, IrFunction, IrModule, IrType, OpaqueType},
    pipeline::context::PipelineContext,
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
    constructor: Option<&'a IrFunction>,
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

pub fn render_go_facade(
    config: &PipelineContext,
    ir: &IrModule,
    globally_emitted_opaques: &BTreeSet<String>,
) -> Result<Vec<GeneratedGoFile>> {
    let functions = ir
        .functions
        .iter()
        .filter(|function| function.kind == IrFunctionKind::Function)
        .filter(|function| free_function_supported(&config, function))
        .collect::<Vec<_>>();
    let enums = ir.enums.iter().collect::<Vec<_>>();
    let classes = collect_facade_classes(&config, ir)?;
    let callback_usages = collect_callback_usages(&functions, &classes, ir);

    if functions.is_empty() && classes.is_empty() && enums.is_empty() {
        return Ok(Vec::new());
    }

    ensure_unique_go_exports(&functions)?;

    // Exclude opaque types already declared in another file (primary class handles and
    // any non-class opaque types claimed by a previously-processed header).
    let local_opaque_types: Vec<&OpaqueType> = ir
        .opaque_types
        .iter()
        .filter(|ot| !globally_emitted_opaques.contains(&ot.name))
        .collect();

    Ok(vec![GeneratedGoFile {
        filename: config.go_filename(""),
        contents: render_go_facade_file(
            &config,
            &enums,
            &functions,
            &classes,
            &callback_usages,
            &local_opaque_types,
        ),
    }])
}

fn collect_facade_classes<'a>(
    config: &PipelineContext,
    ir: &'a IrModule,
) -> Result<Vec<AnalyzedFacadeClass<'a>>> {
    let mut methods_by_owner = BTreeMap::<&str, Vec<&IrFunction>>::new();
    for function in ir
        .functions
        .iter()
        .filter(|function| function.kind == IrFunctionKind::Method)
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
        .filter(|function| function.kind == IrFunctionKind::Constructor)
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
        .filter(|function| function.kind == IrFunctionKind::Destructor)
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

        let constructor = constructors.get(owner).copied();
        if let Some(ctor) = constructor {
            if !ctor
                .params
                .iter()
                .all(|param| go_param_supported(config, &param.ty))
            {
                continue;
            }
        }
        let Some(destructor) = destructors.get(owner).copied() else {
            continue;
        };

        classes.push(AnalyzedFacadeClass {
            go_name: go_export_name(&leaf_cpp_name(owner)),
            handle_name: format!("{}Handle", flatten_qualified_cpp_name(owner)),
            constructor,
            destructor,
            methods,
        });
    }

    Ok(classes)
}

fn render_go_facade_file(
    config: &PipelineContext,
    enums: &[&IrEnum],
    functions: &[&IrFunction],
    classes: &[AnalyzedFacadeClass<'_>],
    callback_usages: &[CallbackUsage<'_>],
    opaque_types: &[&OpaqueType],
) -> String {
    let package_name = go_package_name(&config.output.dir);
    let requires_cgo = !functions.is_empty() || !classes.is_empty();
    let requires_errors = classes.iter().any(|class| class.constructor.is_some())
        || functions.iter().any(|function| {
            matches!(
                function.returns.kind,
                IrTypeKind::String | IrTypeKind::CString | IrTypeKind::FixedByteArray
            )
        })
        || classes.iter().any(|class| {
            class.methods.iter().any(|function| {
                matches!(
                    function.returns.kind,
                    IrTypeKind::String | IrTypeKind::CString | IrTypeKind::FixedByteArray
                )
            })
        });
    let requires_unsafe = functions.iter().any(|function| {
        has_string_params(function.params.iter())
            || has_pointer_params(function.params.iter())
            || has_byte_array_params(function.params.iter())
            || has_void_model_params(function.params.iter())
            || matches!(
                function.returns.kind,
                IrTypeKind::Pointer
                    | IrTypeKind::FixedByteArray
                    | IrTypeKind::FixedArray
                    | IrTypeKind::FixedModelArray
            )
    }) || classes.iter().any(|class| {
        class.constructor.iter().any(|ctor| {
            has_string_params(ctor.params.iter())
                || has_pointer_params(ctor.params.iter())
                || has_byte_array_params(ctor.params.iter())
                || has_void_model_params(ctor.params.iter())
        }) || class.methods.iter().any(|function| {
            has_string_params(function.params.iter().skip(1))
                || has_pointer_params(function.params.iter().skip(1))
                || has_byte_array_params(function.params.iter().skip(1))
                || has_void_model_params(function.params.iter().skip(1))
                || matches!(
                    function.returns.kind,
                    IrTypeKind::Pointer | IrTypeKind::FixedByteArray
                )
        })
    });
    let requires_sync = !callback_usages.is_empty();

    let mut out = String::new();
    out.push_str(&format!("package {}\n\n", package_name));
    if requires_cgo {
        out.push_str("/*\n");
        out.push_str("#include <stdlib.h>\n");
        if ir_uses_struct_timeval(functions, classes) {
            out.push_str("#include <sys/time.h>\n");
        }
        out.push_str(&format!(
            "#include \"{}\"\n",
            config.generated_header_include(&config.output.header)
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

    let covered_handles = classes
        .iter()
        .map(|class| class.handle_name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    // Also track Go names used by primary class wrappers to catch cases where a typedef
    // and a class produce the same Go name (e.g. _LegId class → "LegId", LegId opaque → "LegId").
    let covered_go_names: BTreeSet<String> = classes
        .iter()
        .map(|class| class.go_name.clone())
        .collect();

    for opaque in opaque_types {
        if covered_handles.contains(opaque.name.as_str()) {
            continue;
        }
        let base = opaque.name.strip_suffix("Handle").unwrap_or(&opaque.name);
        let go_name = go_export_name(base);
        if covered_go_names.contains(&go_name) {
            continue;
        }
        out.push_str(&format!(
            "type {} struct {{\n    ptr *C.{}\n}}\n\n",
            go_name, opaque.name
        ));
    }

    for class in classes {
        out.push_str(&render_facade_class(class));
        out.push('\n');
        if let Some(_) = class.constructor {
            out.push_str(&render_facade_constructor(config, class));
            out.push('\n');
        }
        out.push_str(&render_facade_close(class));
        out.push('\n');
        out.push_str(&render_handle_helpers(class));
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
            (param.ty.kind == IrTypeKind::Callback).then(|| {
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
        .map(|param| format!("{} {}", param.name, callback_go_type(&param.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    let returns = if callback.returns.kind == IrTypeKind::Void {
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
        .map(|param| format!("{} {}", param.name, callback_cgo_param_type(&param.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = String::new();
    out.push_str(&format!("//export {}\n", callback_go_export_name(usage)));
    out.push_str(&format!(
        "func {}({})",
        callback_go_export_name(usage),
        params
    ));
    if usage.callback.returns.kind != IrTypeKind::Void {
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
    if usage.callback.returns.kind == IrTypeKind::Void {
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
    if usage.callback.returns.kind == IrTypeKind::Void {
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

fn render_facade_constructor(config: &PipelineContext, class: &AnalyzedFacadeClass<'_>) -> String {
    let constructor = class.constructor.expect("render_facade_constructor called without constructor");
    let constructor_params = constructor.params.iter().collect::<Vec<_>>();
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
        constructor.name,
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

fn render_handle_helpers(class: &AnalyzedFacadeClass<'_>) -> String {
    let go_name = &class.go_name;
    let handle = &class.handle_name;
    let receiver = receiver_name(go_name);
    format!(
        "func require{go_name}Handle({receiver} *{go_name}) *C.{handle} {{\n\
         \x20   if {receiver} == nil || {receiver}.ptr == nil {{\n\
         \x20       panic(\"{go_name} handle is required but nil\")\n\
         \x20   }}\n\
         \x20   return {receiver}.ptr\n\
         }}\n\
         \n\
         func optional{go_name}Handle({receiver} *{go_name}) *C.{handle} {{\n\
         \x20   if {receiver} == nil {{\n\
         \x20       return nil\n\
         \x20   }}\n\
         \x20   return {receiver}.ptr\n\
         }}\n"
    )
}

fn render_general_api_method(
    config: &PipelineContext,
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

    let sig = go_return_sig(config, &function.returns);
    let sig_part = if sig.is_empty() {
        String::new()
    } else {
        format!(" {sig}")
    };
    let mut out = format!(
        "func ({receiver} *{}) {}({}){sig_part} {{\n",
        class.go_name,
        go_method_export_name(function),
        params
    );
    out.push_str(&format!(
        "    if {receiver} == nil || {receiver}.ptr == nil {{\n        {}\n    }}\n",
        go_nil_return_stmt(&function.returns)
    ));
    out.push_str(&indented_lines(&prep.setup_lines));
    out.push_str(&indented_lines(&prep.defer_lines));
    out.push_str(&render_go_call_return(
        config,
        function,
        &call,
        &prep.post_call_lines,
    ));
    out.push_str("}\n");
    out
}

fn render_free_function(config: &PipelineContext, function: &IrFunction) -> String {
    if has_callback_param(function.params.iter()) {
        return render_callback_free_function(config, function);
    }
    let params_list = function.params.iter().collect::<Vec<_>>();
    let params = render_param_list(config, &params_list);
    let prep = render_call_prep(config, &params_list);
    let call = format!("C.{}({})", function.name, prep.args.join(", "));
    let go_name = go_facade_export_name(function);

    let sig = go_return_sig(config, &function.returns);
    let sig_part = if sig.is_empty() {
        String::new()
    } else {
        format!(" {sig}")
    };
    let mut out = format!("func {go_name}({params}){sig_part} {{\n");
    out.push_str(&indented_lines(&prep.setup_lines));
    out.push_str(&indented_lines(&prep.defer_lines));
    out.push_str(&render_go_call_return(
        config,
        function,
        &call,
        &prep.post_call_lines,
    ));
    out.push_str("}\n");
    out
}

fn render_callback_method(
    config: &PipelineContext,
    class: &AnalyzedFacadeClass<'_>,
    function: &IrFunction,
) -> String {
    let receiver = receiver_name(&class.go_name);
    let method_params = function.params.iter().skip(1).collect::<Vec<_>>();
    let params = render_param_list(config, &method_params);
    let prep = render_callback_call_prep(config, function, &method_params, 1);
    let call = format!(
        "C.{}_bridge({})",
        function.name,
        std::iter::once(format!("{receiver}.ptr"))
            .chain(prep.args)
            .collect::<Vec<_>>()
            .join(", ")
    );

    let sig = go_return_sig(config, &function.returns);
    let sig_part = if sig.is_empty() {
        String::new()
    } else {
        format!(" {sig}")
    };
    let mut out = format!(
        "func ({receiver} *{}) {}({}){sig_part} {{\n",
        class.go_name,
        go_method_export_name(function),
        params
    );
    out.push_str(&format!(
        "    if {receiver} == nil || {receiver}.ptr == nil {{\n        {}\n    }}\n",
        go_nil_return_stmt(&function.returns)
    ));
    out.push_str(&indented_lines(&prep.setup_lines));
    out.push_str(&indented_lines(&prep.defer_lines));
    out.push_str(&render_go_call_return(
        config,
        function,
        &call,
        &prep.post_call_lines,
    ));
    out.push_str("}\n");
    out
}

fn render_callback_free_function(config: &PipelineContext, function: &IrFunction) -> String {
    let params_list = function.params.iter().collect::<Vec<_>>();
    let params = render_param_list(config, &params_list);
    let prep = render_callback_call_prep(config, function, &params_list, 0);
    let call = format!("C.{}_bridge({})", function.name, prep.args.join(", "));
    let go_name = go_facade_export_name(function);

    let sig = go_return_sig(config, &function.returns);
    let sig_part = if sig.is_empty() {
        String::new()
    } else {
        format!(" {sig}")
    };
    let mut out = format!("func {go_name}({params}){sig_part} {{\n");
    out.push_str(&indented_lines(&prep.setup_lines));
    out.push_str(&indented_lines(&prep.defer_lines));
    out.push_str(&render_go_call_return(
        config,
        function,
        &call,
        &prep.post_call_lines,
    ));
    out.push_str("}\n");
    out
}

fn render_callback_call_prep(
    config: &PipelineContext,
    function: &IrFunction,
    params: &[&ir_norm::IrParam],
    param_offset: usize,
) -> RenderedCallPrep {
    let mut prep = RenderedCallPrep::default();

    for (index, param) in params.iter().enumerate() {
        if param.ty.kind == IrTypeKind::Callback {
            let state = callback_state_name_from_function(function, index + param_offset);
            prep.setup_lines.push(format!("{state}.mu.Lock()"));
            prep.setup_lines
                .push(format!("{state}.fn = {}", param.name));
            prep.setup_lines.push(format!("{state}.mu.Unlock()"));
            prep.args.push(format!("C.bool({} != nil)", param.name));
            continue;
        }

        match param.ty.kind {
            IrTypeKind::String | IrTypeKind::CString => {
                let c_name = format!("cArg{index}");
                prep.setup_lines
                    .push(format!("{c_name} := C.CString({})", param.name));
                prep.defer_lines
                    .push(format!("defer C.free(unsafe.Pointer({c_name}))"));
                prep.args.push(c_name);
            }
            IrTypeKind::FixedByteArray => {
                let c_name = format!("cArg{index}");
                prep.setup_lines.push(format!(
                    "{c_name} := (*C.uint8_t)(unsafe.Pointer(&{}[0]))",
                    param.name
                ));
                prep.args.push(c_name);
            }
            IrTypeKind::FixedArray => {
                let c_name = format!("cArg{index}");
                let elem = ir_norm::fixed_array_elem_type(&param.ty.cpp_type).unwrap_or("int32_t");
                let c_elem = primitive_cgo_cast_type(elem).unwrap_or("C.int32_t");
                prep.setup_lines.push(format!(
                    "{c_name} := (*{c_elem})(unsafe.Pointer(&{}[0]))",
                    param.name
                ));
                prep.args.push(c_name);
            }
            IrTypeKind::FixedModelArray => {
                let c_handle = param.ty.handle.as_deref().unwrap_or("");
                let elem_cpp =
                    ir_norm::fixed_array_elem_type(&param.ty.cpp_type).unwrap_or("");
                let go_name = go_export_name(&flatten_qualified_cpp_name(elem_cpp));
                let handles_name = format!("cHandles{index}");
                let c_name = format!("cArg{index}");
                prep.setup_lines.push(format!(
                    "{handles_name} := make([]*C.{c_handle}, len({}))",
                    param.name
                ));
                prep.setup_lines.push(format!(
                    "for _i, _v := range {} {{ {handles_name}[_i] = require{go_name}Handle(_v) }}",
                    param.name
                ));
                prep.setup_lines.push(format!(
                    "{c_name} := (**C.{c_handle})(unsafe.Pointer(&{handles_name}[0]))"
                ));
                prep.args.push(c_name);
            }
            IrTypeKind::Reference => render_reference_arg(&mut prep, &param.ty, &param.name, index),
            IrTypeKind::Pointer => render_pointer_arg(&mut prep, &param.ty, &param.name, index),
            IrTypeKind::ExternStructReference => {
                render_extern_struct_arg(&mut prep, &param.ty, &param.name, index, true)
            }
            IrTypeKind::ExternStructPointer => {
                render_extern_struct_arg(&mut prep, &param.ty, &param.name, index, false)
            }
            IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue => {
                render_model_arg(config, &mut prep, &param.ty, &param.name, index)
            }
            _ => prep.args.push(render_c_arg(&param.ty, &param.name)),
        }
    }

    prep
}

fn render_param_list(config: &PipelineContext, params: &[&ir_norm::IrParam]) -> String {
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

fn render_call_prep(config: &PipelineContext, params: &[&ir_norm::IrParam]) -> RenderedCallPrep {
    let mut prep = RenderedCallPrep::default();

    for (index, param) in params.iter().enumerate() {
        match param.ty.kind {
            IrTypeKind::String | IrTypeKind::CString => {
                let c_name = format!("cArg{index}");
                prep.setup_lines
                    .push(format!("{c_name} := C.CString({})", param.name));
                prep.defer_lines
                    .push(format!("defer C.free(unsafe.Pointer({c_name}))"));
                prep.args.push(c_name);
            }
            IrTypeKind::FixedByteArray => {
                let c_name = format!("cArg{index}");
                prep.setup_lines.push(format!(
                    "{c_name} := (*C.uint8_t)(unsafe.Pointer(&{}[0]))",
                    param.name
                ));
                prep.args.push(c_name);
            }
            IrTypeKind::FixedArray => {
                let c_name = format!("cArg{index}");
                let elem = ir_norm::fixed_array_elem_type(&param.ty.cpp_type).unwrap_or("int32_t");
                let c_elem = primitive_cgo_cast_type(elem).unwrap_or("C.int32_t");
                prep.setup_lines.push(format!(
                    "{c_name} := (*{c_elem})(unsafe.Pointer(&{}[0]))",
                    param.name
                ));
                prep.args.push(c_name);
            }
            IrTypeKind::FixedModelArray => {
                let c_handle = param.ty.handle.as_deref().unwrap_or("");
                let elem_cpp =
                    ir_norm::fixed_array_elem_type(&param.ty.cpp_type).unwrap_or("");
                let go_name = go_export_name(&flatten_qualified_cpp_name(elem_cpp));
                let handles_name = format!("cHandles{index}");
                let c_name = format!("cArg{index}");
                prep.setup_lines.push(format!(
                    "{handles_name} := make([]*C.{c_handle}, len({}))",
                    param.name
                ));
                prep.setup_lines.push(format!(
                    "for _i, _v := range {} {{ {handles_name}[_i] = require{go_name}Handle(_v) }}",
                    param.name
                ));
                prep.setup_lines.push(format!(
                    "{c_name} := (**C.{c_handle})(unsafe.Pointer(&{handles_name}[0]))"
                ));
                prep.args.push(c_name);
            }
            IrTypeKind::Reference => render_reference_arg(&mut prep, &param.ty, &param.name, index),
            IrTypeKind::Pointer => render_pointer_arg(&mut prep, &param.ty, &param.name, index),
            IrTypeKind::ExternStructReference => {
                render_extern_struct_arg(&mut prep, &param.ty, &param.name, index, true)
            }
            IrTypeKind::ExternStructPointer => {
                render_extern_struct_arg(&mut prep, &param.ty, &param.name, index, false)
            }
            IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue => {
                render_model_arg(config, &mut prep, &param.ty, &param.name, index)
            }
            _ => prep.args.push(render_c_arg(&param.ty, &param.name)),
        }
    }

    prep
}

fn render_model_handle_arg(config: &PipelineContext, ty: &IrType, name: &str) -> Option<String> {
    let projection = config.known_model_projection(&ty.cpp_type)?;
    let handle_arg = if ty.kind == IrTypeKind::ModelPointer {
        format!("optional{}Handle({})", projection.go_name, name)
    } else {
        format!("require{}Handle({})", projection.go_name, name)
    };
    // When the C function's expected handle type differs from the model projection's
    // handle type (e.g., UCIDHandle* vs _UCIDHandle*), cast via unsafe.Pointer.
    if let Some(expected_handle) = &ty.handle {
        if *expected_handle != projection.handle_name {
            return Some(format!(
                "(*C.{expected_handle})(unsafe.Pointer({handle_arg}))"
            ));
        }
    }
    Some(handle_arg)
}

/// Returns an expression for `raw` cast to the projection's handle type,
/// inserting an unsafe.Pointer cast when the C return type's handle differs
/// from the projection's stored handle type.
fn cast_raw_to_projection_handle(
    config: &PipelineContext,
    returns: &IrType,
    raw_expr: &str,
) -> String {
    if let Some(projection) = config.known_model_projection(&returns.cpp_type) {
        if let Some(expected_handle) = &returns.handle {
            if *expected_handle != projection.handle_name {
                return format!(
                    "(*C.{})(unsafe.Pointer({}))",
                    projection.handle_name, raw_expr
                );
            }
        }
    }
    raw_expr.to_string()
}

fn render_pointer_arg(prep: &mut RenderedCallPrep, ty: &IrType, name: &str, index: usize) {
    let c_name = format!("cArg{index}");
    let base_cpp = ty.cpp_type.trim_end_matches('*').trim();
    let c_type = primitive_cgo_cast_type(base_cpp)
        .or_else(|| primitive_cgo_cast_type(ty.c_type.trim_end_matches('*').trim()))
        .unwrap_or("C.int");
    prep.setup_lines
        .push(format!("{c_name} := (*{c_type})(unsafe.Pointer({name}))"));
    prep.args.push(c_name);
}

fn render_extern_struct_arg(
    prep: &mut RenderedCallPrep,
    ty: &IrType,
    name: &str,
    index: usize,
    require_non_nil: bool,
) {
    let c_name = format!("cArg{index}");
    let go_type = extern_struct_go_type(ty).expect("external struct params must be prefiltered");
    if require_non_nil {
        prep.setup_lines.push(format!("if {name} == nil {{"));
        prep.setup_lines
            .push(format!("    panic(\"{name} reference is nil\")"));
        prep.setup_lines.push("}".to_string());
    }
    prep.setup_lines
        .push(format!("{c_name} := ({go_type})(unsafe.Pointer({name}))"));
    prep.args.push(c_name);
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

fn has_string_params<'a>(mut params: impl Iterator<Item = &'a ir_norm::IrParam>) -> bool {
    params.any(|param| matches!(param.ty.kind, IrTypeKind::String | IrTypeKind::CString))
}

fn has_pointer_params<'a>(mut params: impl Iterator<Item = &'a ir_norm::IrParam>) -> bool {
    params.any(|param| {
        matches!(
            param.ty.kind,
            IrTypeKind::Pointer
                | IrTypeKind::ExternStructPointer
                | IrTypeKind::ExternStructReference
        )
    })
}

fn has_byte_array_params<'a>(mut params: impl Iterator<Item = &'a ir_norm::IrParam>) -> bool {
    params.any(|param| param.ty.kind == IrTypeKind::FixedByteArray)
}

fn has_void_model_params<'a>(mut params: impl Iterator<Item = &'a ir_norm::IrParam>) -> bool {
    params.any(|param| {
        matches!(
            param.ty.kind,
            IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue
        ) && base_model_cpp_type(&param.ty.cpp_type) == "void"
    })
}

fn render_model_arg(
    config: &PipelineContext,
    prep: &mut RenderedCallPrep,
    ty: &IrType,
    name: &str,
    index: usize,
) {
    if let Some(handle_arg) = render_model_handle_arg(config, ty, name) {
        prep.args.push(handle_arg);
        return;
    }
    // void model params: the Go type is unsafe.Pointer, which has no .ptr field.
    // Cast directly to *C.<handle> instead.
    if base_model_cpp_type(&ty.cpp_type) == "void" {
        let handle = ty.handle.as_deref().unwrap_or("void");
        let c_name = format!("cArg{index}");
        prep.setup_lines.push(format!("var {c_name} *C.{handle}"));
        prep.setup_lines.push(format!("if {name} != nil {{"));
        prep.setup_lines
            .push(format!("    {c_name} = (*C.{handle})({name})"));
        prep.setup_lines.push("}".to_string());
        prep.args.push(c_name);
        return;
    }
    let handle = ty.handle.as_deref().unwrap_or("void");
    let c_name = format!("cArg{index}");
    prep.setup_lines.push(format!("var {c_name} *C.{handle}"));
    if ty.kind != IrTypeKind::ModelPointer {
        prep.setup_lines.push(format!("if {name} == nil {{"));
        prep.setup_lines
            .push("    panic(\"reference facade/model argument cannot be nil\")".to_string());
        prep.setup_lines.push("}".to_string());
    }
    prep.setup_lines.push(format!("if {name} != nil {{"));
    prep.setup_lines.push(format!("    {c_name} = {name}.ptr"));
    prep.setup_lines.push("}".to_string());
    prep.args.push(c_name);
}

fn has_callback_param<'a>(mut params: impl Iterator<Item = &'a ir_norm::IrParam>) -> bool {
    params.any(|param| param.ty.kind == IrTypeKind::Callback)
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

fn free_function_supported(config: &PipelineContext, function: &IrFunction) -> bool {
    go_return_supported(config, &function.returns)
        && function
            .params
            .iter()
            .all(|param| go_param_supported(config, &param.ty))
}

fn method_supported(config: &PipelineContext, function: &IrFunction) -> bool {
    go_return_supported(config, &function.returns)
        && function
            .params
            .iter()
            .skip(1)
            .all(|param| go_param_supported(config, &param.ty))
}

fn go_param_supported(config: &PipelineContext, ty: &IrType) -> bool {
    go_param_type(config, ty).is_some()
}

fn go_param_type(config: &PipelineContext, ty: &IrType) -> Option<String> {
    match ty.kind {
        IrTypeKind::String | IrTypeKind::CString => Some("string".to_string()),
        IrTypeKind::FixedByteArray => Some("[]byte".to_string()),
        IrTypeKind::FixedArray => {
            let elem = ir_norm::fixed_array_elem_type(&ty.cpp_type)?;
            Some(format!("[]{}", primitive_go_type(elem).unwrap_or("int32")))
        }
        IrTypeKind::FixedModelArray => {
            let go_name = go_model_return_type(config, ty);
            Some(format!("[]*{go_name}"))
        }
        IrTypeKind::Primitive => go_type_for_ir(ty).map(str::to_string),
        IrTypeKind::Reference => go_type_for_reference(ty).map(|go_type| format!("*{go_type}")),
        IrTypeKind::Pointer => {
            let base = ty.cpp_type.trim_end_matches('*').trim();
            primitive_go_type(base)
                .or_else(|| primitive_go_type(ty.c_type.trim_end_matches('*').trim()))
                .map(|go_type| format!("*{go_type}"))
        }
        IrTypeKind::ExternStructPointer | IrTypeKind::ExternStructReference => {
            extern_struct_go_type(ty)
        }
        IrTypeKind::Callback => Some(leaf_cpp_name(&ty.cpp_type)),
        IrTypeKind::ModelReference | IrTypeKind::ModelPointer | IrTypeKind::ModelValue => {
            if base_model_cpp_type(&ty.cpp_type) == "void" {
                return Some("unsafe.Pointer".to_string());
            }
            config
                .known_model_projection(&ty.cpp_type)
                .map(|projection| format!("*{}", projection.go_name))
                .or_else(|| {
                    ty.handle
                        .as_deref()
                        .and_then(|h| h.strip_suffix("Handle"))
                        .map(|base| format!("*{}", go_export_name(base)))
                })
        }
        _ => None,
    }
}

fn go_return_supported(_config: &PipelineContext, ty: &IrType) -> bool {
    ty.kind == IrTypeKind::Void
        || matches!(
            ty.kind,
            IrTypeKind::String
                | IrTypeKind::CString
                | IrTypeKind::FixedByteArray
                | IrTypeKind::FixedArray
                | IrTypeKind::FixedModelArray
        )
        || (ty.kind == IrTypeKind::Primitive && go_type_for_ir(ty).is_some())
        || (ty.kind == IrTypeKind::Pointer && go_pointer_return_type(ty).is_some())
        || matches!(ty.kind, IrTypeKind::ModelPointer | IrTypeKind::ModelView)
        || ty.kind == IrTypeKind::ModelValue
}

fn go_pointer_return_type(ty: &IrType) -> Option<String> {
    if ty.kind != IrTypeKind::Pointer {
        return None;
    }
    let base = ty.cpp_type.trim_end_matches('*').trim();
    primitive_go_type(base)
        .or_else(|| primitive_go_type(ty.c_type.trim_end_matches('*').trim()))
        .map(|go_type| format!("*{go_type}"))
}

fn go_model_return_type(config: &PipelineContext, ty: &IrType) -> String {
    if base_model_cpp_type(&ty.cpp_type) == "void" {
        return "unsafe.Pointer".to_string();
    }
    config
        .known_model_projection(&ty.cpp_type)
        .map(|projection| projection.go_name.clone())
        .unwrap_or_else(|| {
            ty.handle
                .as_deref()
                .and_then(|h| h.strip_suffix("Handle"))
                .map(|base| go_export_name(base))
                .unwrap_or_else(|| flatten_qualified_cpp_name(&base_model_cpp_type(&ty.cpp_type)))
        })
}

fn is_model_wrapper_return(ty: &IrType) -> bool {
    matches!(
        ty.kind,
        IrTypeKind::ModelPointer | IrTypeKind::ModelView | IrTypeKind::ModelValue
    )
}

/// Returns the Go return type signature string (without surrounding parens for single values).
/// e.g. `""` for void, `"(string, error)"` for string, `"([]*Foo, error)"` for FixedModelArray.
fn go_return_sig(config: &PipelineContext, ty: &IrType) -> String {
    match ty.kind {
        IrTypeKind::Void => String::new(),
        IrTypeKind::String | IrTypeKind::CString => "(string, error)".to_string(),
        IrTypeKind::FixedByteArray => "([]byte, error)".to_string(),
        IrTypeKind::FixedArray => {
            let elem = ir_norm::fixed_array_elem_type(&ty.cpp_type).unwrap_or("int32_t");
            let go_elem = primitive_go_type(elem).unwrap_or("int32");
            format!("([]{go_elem}, error)")
        }
        IrTypeKind::FixedModelArray => {
            let go_name = go_model_return_type(config, ty);
            format!("([]*{go_name}, error)")
        }
        IrTypeKind::Pointer => go_pointer_return_type(ty).unwrap_or_default(),
        _ if is_model_wrapper_return(ty) => {
            let model_ret = go_model_return_type(config, ty);
            if model_ret == "unsafe.Pointer" {
                "unsafe.Pointer".to_string()
            } else {
                format!("*{model_ret}")
            }
        }
        _ => go_type_for_ir(ty).unwrap_or("int32").to_string(),
    }
}

/// Returns the nil/zero return statement used inside the receiver-nil guard block.
fn go_nil_return_stmt(ty: &IrType) -> String {
    match ty.kind {
        IrTypeKind::Void => "return".to_string(),
        IrTypeKind::String | IrTypeKind::CString => {
            "return \"\", errors.New(\"facade receiver is nil\")".to_string()
        }
        IrTypeKind::FixedByteArray | IrTypeKind::FixedArray | IrTypeKind::FixedModelArray => {
            "return nil, errors.New(\"facade receiver is nil\")".to_string()
        }
        IrTypeKind::Pointer => "return nil".to_string(),
        _ if is_model_wrapper_return(ty) => "return nil".to_string(),
        _ => format!(
            "return {}",
            zero_value_for_go_type(go_type_for_ir(ty).unwrap_or("int32"))
        ),
    }
}

/// Renders the function body from the C call onwards (call, post_call, nil-check, return).
/// Does NOT include setup/defer lines or the closing `}`.
fn render_go_call_return(
    config: &PipelineContext,
    function: &IrFunction,
    call: &str,
    post_call_lines: &[String],
) -> String {
    let ty = &function.returns;
    match ty.kind {
        IrTypeKind::Void => {
            let mut out = format!("    {call}\n");
            out.push_str(&indented_lines(post_call_lines));
            out
        }
        IrTypeKind::String => {
            let mut out = format!("    raw := {call}\n");
            out.push_str(&indented_lines(post_call_lines));
            out.push_str(&format!(
                "    if raw == nil {{\n        return \"\", errors.New(\"wrapper returned nil string\")\n    }}\n    defer C.{}_string_free(raw)\n    return C.GoString(raw), nil\n",
                config.naming.prefix
            ));
            out
        }
        IrTypeKind::CString => {
            let mut out = format!("    raw := {call}\n");
            out.push_str(&indented_lines(post_call_lines));
            out.push_str(
                "    if raw == nil {\n        return \"\", errors.New(\"wrapper returned nil string\")\n    }\n    return C.GoString(raw), nil\n",
            );
            out
        }
        IrTypeKind::FixedByteArray => {
            let n = ir_norm::byte_array_length(&ty.cpp_type).unwrap_or(0);
            let mut out = format!("    raw := {call}\n");
            out.push_str(&indented_lines(post_call_lines));
            out.push_str(&format!(
                "    if raw == nil {{\n        return nil, errors.New(\"wrapper returned nil byte array\")\n    }}\n    defer C.{prefix}_byte_array_free(raw)\n    return C.GoBytes(unsafe.Pointer(raw), C.int({n})), nil\n",
                prefix = config.naming.prefix
            ));
            out
        }
        IrTypeKind::FixedArray => {
            let n = ir_norm::fixed_array_length(&ty.cpp_type).unwrap_or(0);
            let elem = ir_norm::fixed_array_elem_type(&ty.cpp_type).unwrap_or("int32_t");
            let go_elem = primitive_go_type(elem).unwrap_or("int32");
            let c_elem = primitive_cgo_cast_type(elem).unwrap_or("C.int32_t");
            let mut out = format!("    raw := {call}\n");
            out.push_str(&indented_lines(post_call_lines));
            out.push_str(&format!(
                "    if raw == nil {{\n        return nil, errors.New(\"wrapper returned nil array\")\n    }}\n    defer C.{prefix}_array_free(unsafe.Pointer(raw))\n    cSlice := (*[{n}]{c_elem})(unsafe.Pointer(raw))\n    result := make([]{go_elem}, {n})\n    for i := range result {{\n        result[i] = {go_elem}(cSlice[i])\n    }}\n    return result, nil\n",
                prefix = config.naming.prefix
            ));
            out
        }
        IrTypeKind::FixedModelArray => {
            let n = ir_norm::fixed_array_length(&ty.cpp_type).unwrap_or(0);
            let go_name = go_model_return_type(config, ty);
            let c_handle = ty.handle.as_deref().unwrap_or("");
            let mut out = format!("    raw := {call}\n");
            out.push_str(&indented_lines(post_call_lines));
            out.push_str(&format!(
                "    if raw == nil {{\n        return nil, errors.New(\"wrapper returned nil model array\")\n    }}\n    defer C.free(unsafe.Pointer(raw))\n    cSlice := (*[{n}]*C.{c_handle})(unsafe.Pointer(raw))\n    result := make([]*{go_name}, {n})\n    for i := range result {{\n        result[i] = &{go_name}{{ptr: cSlice[i]}}\n    }}\n    return result, nil\n"
            ));
            out
        }
        IrTypeKind::Pointer => {
            let go_type = go_pointer_return_type(ty).unwrap();
            let mut out = format!("    raw := {call}\n");
            out.push_str(&indented_lines(post_call_lines));
            out.push_str(&format!("    return ({go_type})(unsafe.Pointer(raw))\n"));
            out
        }
        _ if is_model_wrapper_return(ty) => {
            let go_name = go_model_return_type(config, ty);
            let mut out = format!("    raw := {call}\n");
            out.push_str(&indented_lines(post_call_lines));
            if go_name == "unsafe.Pointer" {
                out.push_str("    return unsafe.Pointer(raw)\n");
            } else {
                let ptr_expr = cast_raw_to_projection_handle(config, ty, "raw");
                out.push_str(&format!(
                    "    if raw == nil {{\n        return nil\n    }}\n    return &{go_name}{{ptr: {ptr_expr}}}\n"
                ));
            }
            out
        }
        _ => {
            let go_type = go_type_for_ir(ty).unwrap();
            let mut out = String::new();
            if go_type == "bool" {
                out.push_str(&format!("    result := {call}\n"));
                out.push_str(&indented_lines(post_call_lines));
                out.push_str("    return bool(result)\n");
            } else {
                out.push_str(&format!("    return {go_type}({call})\n"));
            }
            out
        }
    }
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
    match ty.kind {
        IrTypeKind::String | IrTypeKind::CString => Some("string"),
        IrTypeKind::Primitive => {
            primitive_go_type(&ty.cpp_type).or_else(|| primitive_go_type(&ty.c_type))
        }
        _ => None,
    }
}

fn go_type_for_reference(ty: &IrType) -> Option<&'static str> {
    if ty.kind != IrTypeKind::Reference {
        return None;
    }
    primitive_go_type(&ty.cpp_type).or_else(|| primitive_go_type(&ty.c_type))
}

fn cgo_cast_type(ty: &IrType) -> &'static str {
    primitive_cgo_cast_type(&ty.cpp_type)
        .or_else(|| primitive_cgo_cast_type(&ty.c_type))
        .unwrap_or_else(|| {
            panic!(
                "unsupported type in cgo_cast_type: cpp_type={:?}, c_type={:?}",
                ty.cpp_type, ty.c_type
            )
        })
}

pub fn primitive_go_type_pub(value: &str) -> Option<&'static str> {
    primitive_go_type(value)
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
        "int" => Some("int32"),
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
        "unsignedlong" => Some("C.ulong"),
        "unsignedlonglong" => Some("C.ulonglong"),
        "int" => Some("C.int"),
        "long" => Some("C.long"),
        "longlong" | "signedlonglong" => Some("C.longlong"),
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
    for (index, segment) in value
        .split('_')
        .filter(|segment| !segment.is_empty())
        .enumerate()
    {
        if index > 0
            && segment.chars().next().is_some_and(|ch| ch.is_ascii_digit())
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
        // No params: const version gets the clean name, non-const gets "Mut"
        if function.is_const != Some(true) {
            suffix = "Mut".to_string();
        }
    } else {
        // Has params: append "Const" to distinguish const overloads
        if function.is_const == Some(true) {
            suffix.push_str("Const");
        }
    }
    suffix
}

fn go_overload_token(ty: &IrType) -> String {
    match ty.kind {
        IrTypeKind::Callback => format!("{}Callback", go_export_name(&leaf_cpp_name(&ty.cpp_type))),
        IrTypeKind::String | IrTypeKind::CString => string_overload_token(ty),
        IrTypeKind::Primitive => primitive_overload_token(ty),
        IrTypeKind::ExternStructReference => extern_struct_overload_token(ty, "Ref"),
        IrTypeKind::ExternStructPointer => extern_struct_overload_token(ty, "Ptr"),
        IrTypeKind::ModelReference => format!(
            "{}Ref",
            go_export_name(&flatten_qualified_cpp_name(&base_model_cpp_type(
                &ty.cpp_type
            )))
        ),
        IrTypeKind::ModelPointer => model_pointer_overload_token(ty),
        IrTypeKind::ModelValue => format!(
            "{}Value",
            go_export_name(&flatten_qualified_cpp_name(&base_model_cpp_type(
                &ty.cpp_type
            )))
        ),
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

fn extern_struct_overload_token(ty: &IrType, suffix: &str) -> String {
    let base = base_model_cpp_type(&ty.c_type);
    let tag = base.strip_prefix("struct ").unwrap_or(&base);
    format!("{}{}", go_export_name(&sanitize_go_token(tag)), suffix)
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
    format!(
        "go_{}_cb{}",
        sanitize_go_token(&usage.function.name),
        usage.param_index
    )
}

fn callback_cgo_param_type(ty: &IrType) -> &'static str {
    match ty.kind {
        IrTypeKind::String | IrTypeKind::CString => "*C.char",
        _ => cgo_cast_type_from_c_type(&ty.c_type),
    }
}

fn callback_cgo_return_type(ty: &IrType) -> &'static str {
    cgo_cast_type_from_c_type(&ty.c_type)
}

fn render_callback_go_arg(ty: &IrType, name: &str) -> String {
    match ty.kind {
        IrTypeKind::String | IrTypeKind::CString => format!("C.GoString({name})"),
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
        "unsignedlonglong" => "C.ulonglong",
        "longlong" | "signedlonglong" => "C.longlong",
        "ulong" | "unsignedlong" => "C.ulong",
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

fn extern_struct_go_type(ty: &IrType) -> Option<String> {
    let base = base_model_cpp_type(&ty.c_type);
    let tag = base.strip_prefix("struct ")?;
    Some(format!("*C.struct_{}", sanitize_go_token(tag)))
}

fn ir_uses_struct_timeval(functions: &[&IrFunction], classes: &[AnalyzedFacadeClass<'_>]) -> bool {
    functions
        .iter()
        .flat_map(|function| {
            std::iter::once(&function.returns).chain(function.params.iter().map(|param| &param.ty))
        })
        .chain(classes.iter().flat_map(|class| {
            class
                .constructor
                .iter()
                .flat_map(|ctor| {
                    std::iter::once(&ctor.returns)
                        .chain(ctor.params.iter().map(|param| &param.ty))
                        .collect::<Vec<_>>()
                })
                .chain(std::iter::once(&class.destructor.returns))
                .chain(class.destructor.params.iter().map(|param| &param.ty))
                .chain(class.methods.iter().flat_map(|function| {
                    std::iter::once(&function.returns)
                        .chain(function.params.iter().map(|param| &param.ty))
                }))
        }))
        .any(|ty| {
            matches!(
                ty.kind,
                IrTypeKind::ExternStructReference | IrTypeKind::ExternStructPointer
            ) && base_model_cpp_type(&ty.c_type) == "struct timeval"
        })
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
    use crate::{
        config::Config,
        domain::model_projection::{ModelProjection, ModelProjectionField},
        ir::IrParam,
        pipeline::context::PipelineContext,
    };

    fn test_context_with_known_model() -> PipelineContext {
        PipelineContext::new(Config::default()).with_known_model_projections(vec![
            ModelProjection {
                cpp_type: "ThingModel".to_string(),
                handle_name: "ThingModelHandle".to_string(),
                go_name: "ThingModel".to_string(),
                constructor_symbol: "cgowrap_ThingModel_new".to_string(),
                destructor_symbol: "cgowrap_ThingModel_delete".to_string(),
                fields: vec![ModelProjectionField {
                    go_name: "Value".to_string(),
                    go_type: "int".to_string(),
                    getter_symbol: "cgowrap_ThingModel_GetValue".to_string(),
                    setter_symbol: "cgowrap_ThingModel_SetValue".to_string(),
                    return_kind: IrTypeKind::Primitive,
                }],
            },
        ])
    }

    fn primitive_type(cpp_type: &str, c_type: &str) -> IrType {
        IrType {
            kind: IrTypeKind::Primitive,
            cpp_type: cpp_type.to_string(),
            c_type: c_type.to_string(),
            handle: None,
        }
    }

    fn model_type(kind: IrTypeKind, cpp_type: &str) -> IrType {
        IrType {
            kind,
            cpp_type: cpp_type.to_string(),
            c_type: format!("{cpp_type}Handle*"),
            handle: Some(format!("{cpp_type}Handle")),
        }
    }

    fn reference_type(cpp_type: &str, c_type: &str) -> IrType {
        IrType {
            kind: IrTypeKind::Reference,
            cpp_type: cpp_type.to_string(),
            c_type: c_type.to_string(),
            handle: None,
        }
    }

    #[test]
    fn method_supports_known_model_reference_params() {
        let config = test_context_with_known_model();
        let function = IrFunction {
            name: "cgowrap_Api_GetThing".to_string(),
            kind: IrFunctionKind::Method,
            cpp_name: "Api::GetThing".to_string(),
            method_of: Some("Api".to_string()),
            owner_cpp_type: Some("Api".to_string()),
            is_const: Some(false),
            field_accessor: None,
            returns: primitive_type("bool", "bool"),
            params: vec![
                IrParam {
                    name: "self".to_string(),
                    ty: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "Api".to_string(),
                        c_type: "ApiHandle*".to_string(),
                        handle: Some("ApiHandle".to_string()),
                    },
                },
                IrParam {
                    name: "out".to_string(),
                    ty: model_type(IrTypeKind::ModelReference, "ThingModel"),
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
        let config = test_context_with_known_model();
        let function = IrFunction {
            name: "cgowrap_Api_GetThing".to_string(),
            kind: IrFunctionKind::Method,
            cpp_name: "Api::GetThing".to_string(),
            method_of: Some("Api".to_string()),
            owner_cpp_type: Some("Api".to_string()),
            is_const: Some(false),
            field_accessor: None,
            returns: primitive_type("bool", "bool"),
            params: vec![
                IrParam {
                    name: "self".to_string(),
                    ty: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "Api".to_string(),
                        c_type: "ApiHandle*".to_string(),
                        handle: Some("ApiHandle".to_string()),
                    },
                },
                IrParam {
                    name: "value".to_string(),
                    ty: model_type(IrTypeKind::ModelReference, "UnknownThing"),
                },
            ],
        };

        assert!(method_supported(&config, &function));
    }

    #[test]
    fn method_supports_primitive_reference_and_known_model_params() {
        let config = test_context_with_known_model();
        let function = IrFunction {
            name: "cgowrap_Api_NextThing".to_string(),
            kind: IrFunctionKind::Method,
            cpp_name: "Api::NextThing".to_string(),
            method_of: Some("Api".to_string()),
            owner_cpp_type: Some("Api".to_string()),
            is_const: Some(false),
            field_accessor: None,
            returns: primitive_type("bool", "bool"),
            params: vec![
                IrParam {
                    name: "self".to_string(),
                    ty: IrType {
                        kind: IrTypeKind::Opaque,
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
                    ty: model_type(IrTypeKind::ModelReference, "ThingModel"),
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
            go_overload_token(&model_type(IrTypeKind::ModelReference, "ThingModel")),
            "ThingModelRef"
        );
        assert_eq!(
            go_overload_token(&model_type(IrTypeKind::ModelPointer, "ThingModel")),
            "ThingModelPtr"
        );
        assert_eq!(
            go_overload_token(&IrType {
                kind: IrTypeKind::ModelPointer,
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
                kind: IrTypeKind::CString,
                cpp_type: "NPCSTR".to_string(),
                c_type: "const char*".to_string(),
                handle: None,
            }),
            "NPCSTR"
        );
        assert_eq!(
            go_overload_token(&IrType {
                kind: IrTypeKind::String,
                cpp_type: "NPSTR".to_string(),
                c_type: "char*".to_string(),
                handle: None,
            }),
            "NPSTR"
        );
    }

    #[test]
    fn go_export_name_capitalizes_lowercase_first_letter() {
        assert_eq!(go_export_name("myApi"), "MyApi");
        assert_eq!(go_export_name("thingModel"), "ThingModel");
        assert_eq!(go_export_name("iApiClient"), "IApiClient");
        assert_eq!(go_export_name("UserRecord"), "UserRecord");
    }

    #[test]
    fn render_go_facade_uses_capitalized_struct_name_for_lowercase_cpp_class() {
        use crate::codegen::ir_norm::{IrModule, OpaqueType, SupportMetadata};

        let handle_name = "myApiHandle".to_string();
        let self_param = IrParam {
            name: "self".to_string(),
            ty: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "myApi".to_string(),
                c_type: "myApiHandle*".to_string(),
                handle: Some(handle_name.clone()),
            },
        };
        let ir = IrModule {
            version: 1,
            module: "cgowrap".to_string(),
            source_headers: vec![],
            opaque_types: vec![OpaqueType {
                name: handle_name.clone(),
                cpp_type: "myApi".to_string(),
            }],
            functions: vec![
                IrFunction {
                    name: "cgowrap_myApi_new".to_string(),
                    kind: IrFunctionKind::Constructor,
                    cpp_name: "myApi".to_string(),
                    method_of: None,
                    owner_cpp_type: Some("myApi".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Opaque,
                        cpp_type: "myApi".to_string(),
                        c_type: "myApiHandle*".to_string(),
                        handle: Some(handle_name.clone()),
                    },
                    params: vec![],
                },
                IrFunction {
                    name: "cgowrap_myApi_delete".to_string(),
                    kind: IrFunctionKind::Destructor,
                    cpp_name: "myApi".to_string(),
                    method_of: None,
                    owner_cpp_type: Some("myApi".to_string()),
                    is_const: None,
                    field_accessor: None,
                    returns: IrType {
                        kind: IrTypeKind::Void,
                        cpp_type: "void".to_string(),
                        c_type: "void".to_string(),
                        handle: None,
                    },
                    params: vec![self_param.clone()],
                },
                IrFunction {
                    name: "cgowrap_myApi_IsReady".to_string(),
                    kind: IrFunctionKind::Method,
                    cpp_name: "myApi::IsReady".to_string(),
                    method_of: Some("myApi".to_string()),
                    owner_cpp_type: Some("myApi".to_string()),
                    is_const: Some(true),
                    field_accessor: None,
                    returns: primitive_type("bool", "bool"),
                    params: vec![self_param],
                },
            ],
            enums: vec![],
            callbacks: vec![],
            support: SupportMetadata {
                parser_backend: "test".to_string(),
                notes: vec![],
                skipped_declarations: vec![],
            },
        };

        let config = PipelineContext::new(Config::default());
        let files = render_go_facade(&config, &ir, &BTreeSet::new()).unwrap();
        assert!(!files.is_empty(), "expected at least one Go file");
        let contents = &files[0].contents;
        assert!(
            contents.contains("type MyApi struct {"),
            "expected 'type MyApi struct {{' but got:\n{contents}"
        );
        assert!(
            contents.contains("func NewMyApi()"),
            "expected 'func NewMyApi()' but got:\n{contents}"
        );
    }

    #[test]
    fn model_view_return_is_supported() {
        let ty = model_type(IrTypeKind::ModelView, "ThingModel");
        let config = test_context_with_known_model();
        assert!(go_return_supported(&config, &ty));
    }

    #[test]
    fn model_view_return_renders_wrap_pattern() {
        let config = test_context_with_known_model();
        let self_param = IrParam {
            name: "self".to_string(),
            ty: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "Api".to_string(),
                c_type: "ApiHandle*".to_string(),
                handle: Some("ApiHandle".to_string()),
            },
        };
        let void_type = IrType {
            kind: IrTypeKind::Void,
            cpp_type: "void".to_string(),
            c_type: "void".to_string(),
            handle: None,
        };
        let constructor = IrFunction {
            name: "cgowrap_Api_new".to_string(),
            kind: IrFunctionKind::Constructor,
            cpp_name: "Api".to_string(),
            method_of: None,
            owner_cpp_type: Some("Api".to_string()),
            is_const: None,
            field_accessor: None,
            returns: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: "Api*".to_string(),
                c_type: "ApiHandle*".to_string(),
                handle: Some("ApiHandle".to_string()),
            },
            params: vec![],
        };
        let destructor = IrFunction {
            name: "cgowrap_Api_delete".to_string(),
            kind: IrFunctionKind::Destructor,
            cpp_name: "Api".to_string(),
            method_of: None,
            owner_cpp_type: Some("Api".to_string()),
            is_const: None,
            field_accessor: None,
            returns: void_type,
            params: vec![self_param.clone()],
        };
        let function = IrFunction {
            name: "cgowrap_Api_GetThing".to_string(),
            kind: IrFunctionKind::Method,
            cpp_name: "Api::GetThing".to_string(),
            method_of: Some("Api".to_string()),
            owner_cpp_type: Some("Api".to_string()),
            is_const: Some(false),
            field_accessor: None,
            returns: model_type(IrTypeKind::ModelView, "ThingModel"),
            params: vec![self_param],
        };

        assert!(method_supported(&config, &function));

        let class = AnalyzedFacadeClass {
            go_name: "Api".to_string(),
            handle_name: "ApiHandle".to_string(),
            constructor: Some(&constructor),
            destructor: &destructor,
            methods: vec![&function],
        };
        let code = render_general_api_method(&config, &class, &function);
        assert!(
            code.contains("*ThingModel"),
            "expected return type *ThingModel but got:\n{code}"
        );
        assert!(
            code.contains("return nil"),
            "expected nil check but got:\n{code}"
        );
        assert!(
            code.contains("&ThingModel{ptr: raw}"),
            "expected &ThingModel{{ptr: raw}} but got:\n{code}"
        );
    }

    #[test]
    fn model_view_return_uses_leaf_name_for_unknown_model() {
        let config = test_context_with_known_model();
        let ty = model_type(IrTypeKind::ModelView, "UnknownClass");
        let go_name = go_model_return_type(&config, &ty);
        assert_eq!(go_name, "UnknownClass");
    }
}
