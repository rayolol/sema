use std::path::Path;

// Manual smoke test: walk resolve's own source, resolve every call in every
// function, and report counts. Not a substitute for the tests in the lib
// crate -- there's no ground truth here to assert against (use-statement
// resolution isn't implemented yet, so plenty of real calls are expected to
// come back UNRESOLVED) -- but it's a quick sanity check that the whole
// pipeline runs end-to-end against real code without panicking.
fn main() -> anyhow::Result<()> {
    let src_dir = Path::new("src");
    let entry = src_dir.join("lib.rs");
    let db = resolve::Db::build(src_dir, &entry)?;

    let mut module_paths: Vec<&str> = db.items().map(|i| i.module_path.as_str()).collect();
    module_paths.sort_unstable();
    module_paths.dedup();
    println!("module paths: {module_paths:?}");

    let mut resolved = 0u32;
    let mut unresolved = 0u32;

    for item in db.items() {
        let resolve::ItemKind::Fn(f) = &item.kind else {
            continue;
        };

        let mut visitor = resolve::CallVisitor::new();
        syn::visit::Visit::visit_item_fn(&mut visitor, f);

        for call_path in visitor.calls {
            match db.resolve_path(item.parent, &call_path) {
                Some(_) => resolved += 1,
                None => unresolved += 1,
            }
        }
    }

    println!("resolved: {resolved}, unresolved: {unresolved}");
    Ok(())
}
