fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = std::path::Path::new(&manifest).join("Cargo.toml");

    let ws = sema::analysis(sema::Config {
        manifest_path: path,
        target_dir: "OUT_DIR".into(),
    })
    .unwrap();

    ws.emit_rerun_directives();

    println!("cargo:warning=============================================================");
    println!("cargo:warning=SEMA ANALYSIS RESULTS");
    println!("cargo:warning=============================================================");

    // Structs with detailed analysis
    let structs = ws.structs().collect();
    println!("cargo:warning=Found {} struct(s):", structs.len());
    for s in structs {
        println!(
            "cargo:warning=  STRUCT {} at {}",
            s.node.ident, s.module_path
        );

        // Show fields
        if let syn::Fields::Named(named) = &s.node.fields {
            println!("cargo:warning=    fields:");
            for f in &named.named {
                if let Some(ident) = &f.ident {
                    println!("cargo:warning=      - {}", ident);
                }
            }
        }

        // Show implementations
        let impls = s.impls(&ws);
        if !impls.is_empty() {
            println!("cargo:warning=    implementations ({}):", impls.len());
            for impl_ in impls {
                if let Some(trait_) = &impl_.trait_ {
                    println!(
                        "cargo:warning=      - impl {} for {}",
                        trait_.written, s.node.ident
                    );
                } else {
                    println!("cargo:warning=      - impl {}", s.node.ident);
                }
            }
        }

        // Show methods
        let methods = s.methods(&ws);
        if !methods.is_empty() {
            println!("cargo:warning=    methods ({}):", methods.len());
            for method in methods {
                let async_str = if method.is_async { " async" } else { "" };
                let unsafe_str = if method.is_unsafe { " unsafe" } else { "" };
                println!(
                    "cargo:warning=      - fn{}{}{}({})",
                    async_str, unsafe_str, method.name, method.params
                );
            }
        }

        // Show trait implementations
        let trait_impls = s.trait_impls(&ws);
        if !trait_impls.is_empty() {
            println!("cargo:warning=    implements traits:");
            for trait_ in trait_impls {
                println!("cargo:warning=      - {}", trait_.node.ident);
            }
        }
    }

    // Enums
    let enums = ws.enums().collect();
    println!("cargo:warning=Found {} enum(s):", enums.len());
    for e in enums {
        println!("cargo:warning=  ENUM {} at {}", e.node.ident, e.module_path);
        for v in &e.node.variants {
            println!("cargo:warning=    variant: {}", v.ident);
        }
    }

    // Traits with implementations
    let traits = ws.traits().collect();
    println!("cargo:warning=Found {} trait(s):", traits.len());
    for t in traits {
        println!(
            "cargo:warning=  TRAIT {} at {}",
            t.node.ident, t.module_path
        );

        // Show methods in trait
        for item in &t.node.items {
            if let syn::TraitItem::Fn(method) = item {
                println!("cargo:warning=    method: {}", method.sig.ident);
            }
        }

        // Show implementations of this trait
        let impls = t.impl_blocks(&ws);
        if !impls.is_empty() {
            println!("cargo:warning=    implemented by ({}):", impls.len());
            for impl_ in impls {
                println!(
                    "cargo:warning=      - impl {} for {}",
                    t.node.ident, impl_.self_ty.written
                );
            }
        }
    }

    // Impls
    let impls = ws.impls().collect();
    println!("cargo:warning=Found {} impl(s):", impls.len());
    for i in impls {
        println!(
            "cargo:warning=  IMPL {} for {} at {}",
            i.trait_.as_ref().map(|t| t.written.as_str()).unwrap_or("?"),
            i.self_ty.written,
            i.module_path
        );

        // Show methods in this impl
        for item in &i.node.items {
            if let syn::ImplItem::Fn(method) = item {
                let async_str = if method.sig.asyncness.is_some() {
                    "async "
                } else {
                    ""
                };
                let unsafe_str = if method.sig.unsafety.is_some() {
                    "unsafe "
                } else {
                    ""
                };
                println!(
                    "cargo:warning=    method: fn {}{}{}({})",
                    async_str,
                    unsafe_str,
                    method.sig.ident,
                    method.sig.inputs.len()
                );
            }
        }
    }

    println!("cargo:warning=============================================================");
}
