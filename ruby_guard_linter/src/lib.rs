use std::path::Path;
use syn::visit::Visit;
use syn::{ExprCall, ItemFn, Local, Type};

pub struct Diagnostic {
    pub line: usize,
    pub var_name: String,
    pub type_name: String,
    pub message: String,
}

#[derive(Default)]
pub struct LinterVisitor {
    pub diagnostics: Vec<Diagnostic>,
    in_guarded_fn: bool,
    active_drop_vars: Vec<(String, String, usize)>, // (name, type, line)
    current_line: usize,
}

impl LinterVisitor {
    pub fn new() -> Self {
        Self::default()
    }

    fn is_drop_type(&self, ty: &Type) -> bool {
        let type_str = quote::quote!(#ty).to_string();
        type_str.contains("Vec")
            || type_str.contains("String")
            || type_str.contains("Box")
            || type_str.contains("Rc")
            || type_str.contains("Arc")
            || type_str.contains("MutexGuard")
            || type_str.contains("RefCell")
            || type_str.contains("Guard")
            || type_str.contains("Drop")
    }

    fn has_guard_attr(item: &ItemFn) -> bool {
        for attr in &item.attrs {
            let path_str = quote::quote!(#attr).to_string();
            if path_str.contains("ruby_exception_guard") {
                return true;
            }
        }
        false
    }
}

impl<'ast> Visit<'ast> for LinterVisitor {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let outer_guarded = self.in_guarded_fn;
        let outer_drop_vars = std::mem::take(&mut self.active_drop_vars);

        self.in_guarded_fn = Self::has_guard_attr(node);

        syn::visit::visit_item_fn(self, node);

        self.in_guarded_fn = outer_guarded;
        self.active_drop_vars = outer_drop_vars;
    }

    fn visit_local(&mut self, node: &'ast Local) {
        self.current_line += 1;
        if let syn::Pat::Ident(pat_ident) = &node.pat {
            let var_name = pat_ident.ident.to_string();
            let mut type_name = "Unknown".to_string();
            let mut is_drop = false;

            if let Some(init) = &node.init {
                let expr = &init.expr;
                let expr_str = quote::quote!(#expr).to_string();
                if expr_str.contains("Vec ::")
                    || expr_str.contains("vec !")
                    || expr_str.contains("String ::")
                    || expr_str.contains("Box ::")
                {
                    type_name = expr_str;
                    is_drop = true;
                }
            }

            if let syn::Pat::Type(pat_type) = &node.pat {
                type_name = quote::quote!(#pat_type).to_string();
                if self.is_drop_type(&pat_type.ty) {
                    is_drop = true;
                }
            }

            if is_drop {
                self.active_drop_vars.push((var_name, type_name, self.current_line));
            }
        }

        syn::visit::visit_local(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        self.current_line += 1;
        let call_str = quote::quote!(#node).to_string();
        let is_ffi_call =
            call_str.contains("extern") || call_str.contains("rb_") || call_str.contains("ffi");

        if is_ffi_call && !self.in_guarded_fn {
            for (var_name, type_name, var_line) in &self.active_drop_vars {
                self.diagnostics.push(Diagnostic {
                    line: *var_line,
                    var_name: var_name.clone(),
                    type_name: type_name.clone(),
                    message: format!(
                        "Rule 6 Violation: Local variable '{}' (type implementing Drop: '{}') is active across unprotected C FFI call '{}'. Reference priority.md Rule 6.",
                        var_name, type_name, call_str
                    ),
                });
            }
        }

        syn::visit::visit_expr_call(self, node);
    }
}

pub fn lint_source(source_code: &str) -> Result<Vec<Diagnostic>, syn::Error> {
    let syntax_tree = syn::parse_file(source_code)?;
    let mut visitor = LinterVisitor::new();
    visitor.visit_file(&syntax_tree);
    Ok(visitor.diagnostics)
}

pub fn lint_file<P: AsRef<Path>>(path: P) -> Result<Vec<Diagnostic>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    Ok(lint_source(&content)?)
}
