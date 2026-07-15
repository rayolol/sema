#[derive(Default)]
pub struct CallVisitor {
    pub calls: Vec<Vec<String>>,
}

impl CallVisitor {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'ast> syn::visit::Visit<'ast> for CallVisitor {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(ep) = &*node.func {
            let path: Vec<String> = ep
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect();
            self.calls.push(path);
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        self.calls.push(vec![node.method.to_string()]);
        syn::visit::visit_expr_method_call(self, node);
    }
}
