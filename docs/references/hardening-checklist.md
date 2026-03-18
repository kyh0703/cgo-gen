# Hardening Checklist

## Must fix before wider adoption

- [x] Prevent false default-constructor generation when a class declares constructors but wrapper generation cannot safely synthesize a default constructor.
- [x] Add overload collision detection or deterministic overload-safe symbol naming.
- [ ] Catch C++ exceptions at every exported C ABI boundary and convert to an explicit failure policy.

## Should fix next

- [ ] Stop emitting environment-specific absolute include paths into generated `wrapper.cpp` unless explicitly requested.
- [ ] Harden `compile_commands.json` argument parsing for `-include`, response files, and other multi-arg compiler options.
- [ ] Wrap libclang resources in RAII types so `TranslationUnit` and `Index` are always disposed on early-return paths.

## Good follow-up improvements

- [ ] Add unsupported-feature diagnostics for templates, overloaded methods, and exception-bearing APIs.
- [ ] Add more compile-and-run smoke tests across multiple fixture shapes.
- [ ] Upgrade overload handling from collision detection to deterministic overload-safe wrapper naming.
