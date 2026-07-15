# sema

Semantic analysis tool for Rust codebases, designed to power proc-macro code generation by querying structs, enums, traits, impls, and functions with full type relationships resolved.

## How it works

`sema` walks a src/ directory and parses every file using `syn` AST nodes for downstream manipulation, and indexes the relationships (impl→struct, impl→trait) by id so they can be queried by name.

All queries start from `Workspace` and return a chainable `Query<T>`:

```rust
// Structs
ws.structs()
  .named("Foo")                        // exact name match
  .public()                            // pub visibility only
  .in_module("motor")           // module path prefix
  .with_attribute("derive_thing")      // has #[derive_thing]
  .named_matching(|n| n.ends_with("State"))
  .collect()                           // → Vec<&ResolvedStruct>

// Enums, traits, impls, functions
ws.enums().public().collect()
ws.traits().named("Device").collect()
ws.impls().in_module("crate::motor").collect()
ws.functions().with_attribute("handler").collect()

// Cross-references
let s: &ResolvedStruct = ...;
s.methods(&ws)       // → Vec<MethodInfo>  (name, param count, async, unsafe)
s.impls(&ws)         // → Vec<&ResolvedImpl>
s.trait_impls(&ws)   // → Vec<&ResolvedTrait>

let t: &ResolvedTrait = ...;
t.impl_blocks(&ws)   // → Vec<&ResolvedImpl>

// Lookup by id
ws.struct_by_id(id)  // → Option<&ResolvedStruct>
ws.trait_by_id(id)   // → Option<&ResolvedTrait>
ws.by_id(id)         // → Option<&dyn SemaItem>  (any item kind)
```

## Resolved types

Every item carries its `syn` AST node, so you have full access to fields, generics, attributes, and method signatures without re-parsing:

```rust
// ResolvedStruct
s.id            // ItemId — stable hash of module_path::name
s.module_path   // "crate::motor"  (empty string for crate root items)
s.node          // syn::ItemStruct — fields, generics, attrs, vis
s.file          // PathBuf to the source file

// ResolvedImpl
i.self_ty.written    // "MotorConfig"
i.self_ty.resolved   // Option<ItemId> — links to the struct/enum
i.trait_.written     // "Device"  (if trait impl)
i.trait_.resolved    // Option<ItemId> — links to the trait

// ResolvedTrait
t.node   // syn::ItemTrait — all method signatures
```

However the limitations are: 

- analyzes exactly one crate target (the file passed as entry, analysis() always uses src/lib.rs); other targets in the same package (e.g. src/bin/*.rs) aren't walked.
- `module_path` is empty for items defined at the crate root (`lib.rs`).

- use-statement resolution isn't implemented. ItemRef.resolved can come back None for a trait/type referenced via use rather than its full path, even though .written and the item's own discovery are unaffected.

- Path resolution only handles a single trailing segment past a type/module (no multi-hop associated-item paths).

- Type disambiguation for impls is name-based" (no type inference at all, purely name/path matching)