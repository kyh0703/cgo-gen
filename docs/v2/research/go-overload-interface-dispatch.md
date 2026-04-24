# Go Overload Interface Dispatch Review

## Current Path

- Raw wrapper names are made unique before rendering by grouping identical IR function names and appending `__<type signature>` in `assign_unique_function_symbols`.
- The raw suffix is derived from supported IR parameter kinds, with class method receiver parameters skipped and constness appended for methods.
- Go facade names stay clean for non-overloaded declarations. They only receive Go-facing overload tokens when `has_disambiguated_raw_overload_suffix()` proves the raw symbol has a real overload suffix.
- The current Go-facing typed exports are deterministic and compile-time checked, for example `SetFlagBool` or `SetFlagInt32`.

## Dispatcher Option

A SWIG-like Go facade dispatcher would expose one generated method or function per overload group:

```go
func (w *Widget) Set(args ...interface{}) int32 {
    switch len(args) {
    case 1:
        switch v := args[0].(type) {
        case int32:
            return w.SetInt32(v)
        case bool:
            return w.SetBool(v)
        }
    }
    panic("no matching overload for Widget.Set")
}
```

This can be layered over the existing suffixed exports, but it should not replace them in the first slice.

## Safe Cases

- Overload groups where arity differs after dropping the receiver.
- Overload groups where every argument position maps to distinct Go runtime types, such as `int32` vs `string` vs `bool`.
- Overload groups where all overloads have the same return shape. The dispatcher can preserve the existing return type only when every candidate returns the same Go signature.
- Convenience-only APIs where runtime dispatch failure is acceptable and the direct typed methods remain available.

## Ambiguous Or Unsuitable Cases

- Primitive aliases can collapse to the same Go runtime type. For example two C++ typedefs may intentionally stay distinct in overload suffixes but both become `int32` or `int64` at runtime.
- Enums currently render as Go value types through `go_value_type`; dispatching by `interface{}` may not distinguish enum aliases from primitive values if they project to the same Go type.
- Model reference and model pointer parameters can both project to the same `*GoModel` type, while the current suffix can still represent `Ref` vs `Ptr` semantics and nil rules.
- Pointer and reference primitive parameters both become pointer-shaped Go values in several cases, so nil and typed nil values become hard to route correctly.
- Callback typedefs are nominally named in Go but runtime dispatch has to preserve callback bridge state indexing and should avoid merging unrelated callback typedefs.
- Return-only overload differences cannot be expressed by Go call syntax, and an `interface{}` return would erase the current generated return contract.
- Variadic `...interface{}` removes compile-time arity and type checking for the public call site.

## Recommendation

Do not replace the current suffix-based typed exports with a SWIG-style dispatcher.

The best direction is additive and conservative:

- Keep suffixed typed exports as the stable, documented, compile-time checked API.
- Optionally generate an unsuffixed dispatcher only for overload groups that are unambiguous after Go type projection.
- Skip dispatcher generation for ambiguous groups instead of weakening the direct API.
- Prefer dispatcher methods that return the same type as all candidates; if return signatures differ, do not generate a dispatcher for that group.
- Treat dispatcher generation as opt-in or explicitly documented convenience output, because it changes the ergonomics and failure mode from compile-time to runtime.

## First Implementation Slice

If this is implemented later, keep the first slice small:

- Add overload grouping in `src/codegen/go_facade.rs` after supported free functions and class methods are collected.
- Compute a dispatcher eligibility key from Go arity, Go parameter type, and Go return signature, not from C++ spelling alone.
- Render dispatcher wrappers that call the existing suffixed typed exports.
- Leave raw C ABI symbols and existing `go_overload_suffix()` behavior unchanged.
- Add tests in `tests/overload_collisions.rs` for:
  - one eligible overloaded free function dispatcher,
  - one eligible overloaded method dispatcher,
  - one ambiguous projected overload group that keeps only suffixed exports.

## Validation

- `cargo test --test overload_collisions` passed before this review note.
- `rg -n "go_overload_suffix|has_disambiguated_raw_overload_suffix|go_method_export_name|go_facade_export_name" src/codegen/go_facade.rs` confirms the current Go overload naming path.
- `rg -n "Recommendation|Ambiguous|First implementation slice" docs/v2/research/go-overload-interface-dispatch.md` should find the decision anchors for follow-up planning.
