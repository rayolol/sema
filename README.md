# sema

Semantic analysis tool for Rust codebases, built on rust-analyzer's backend. Designed for use in `build.rs` scripts to power proc-macro code generation — query structs, enums, traits, impls, and functions with full type relationships resolved.

## How it works

`sema` loads a Cargo workspace via `ra_ap_load_cargo`, walks the HIR module tree to collect all local items, converts them to `syn` AST nodes for downstream manipulation, and indexes the relationships (impl→struct, impl→trait) so they can be queried by name.

The pipeline is:

```
load_workspace_at()  →  analyse::collect()  →  bridge::build_workspace()  →  Workspace
      (RA/HIR)              (walk HIR)            (convert to syn + index)     (query API)
```

## Usage

Add to `build-dependencies` in your crate's `Cargo.toml`:

```toml
[build-dependencies]
sema = { path = "../sema" }
```

Then in `build.rs`:

```rust
fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = std::path::Path::new(&manifest).join("Cargo.toml");

    let ws = sema::analysis(sema::Config { manifest_path: path }).unwrap();

    // Tell Cargo to re-run this script if any source file changes
    ws.emit_rerun_directives();

    // Query the workspace
    for s in ws.structs().with_attribute("my_macro").collect() {
        println!("Found annotated struct: {}", s.node.ident);
    }
}
```

## Query API

All queries start from `Workspace` and return a chainable `Query<T>`:

```rust
// Structs
ws.structs()
  .named("Foo")                        // exact name match
  .public()                            // pub visibility only
  .in_module("crate::motor")           // module path prefix
  .with_attribute("derive_thing")      // has #[derive_thing]
  .named_matching(|n| n.ends_with("State"))
  .collect()                           // → Vec<&ResolvedStruct>

// Enums, traits, impls, functions — same combinators
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

## Limitations

- Only analyzes crates local to the workspace (`no_deps: true`). Std and external crate items are excluded.
- Must run inside a `build.rs` — requires `OUT_DIR` env var for the intermediate target directory.
- `module_path` is empty for items defined at the crate root (`lib.rs`).
- Type disambiguation for impls is name-based: if two types in the same workspace share a name, impl resolution may link to the wrong one.
- Proc-macro expansion is disabled (`ProcMacroServerChoice::None`) — items inside proc-macro-generated code won't appear.
