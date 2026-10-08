use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Expr, ItemFn, LitStr, Path, Token};
use syn::parse::{Parse, ParseStream};

struct GuardArgs {
    ensure_fn: Option<Expr>,
    use_protect: bool,
}

impl Parse for GuardArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(GuardArgs {
                ensure_fn: None,
                use_protect: false,
            });
        }

        let mut ensure_fn = None;
        let mut use_protect = false;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            if ident == "ensure" {
                input.parse::<Token![=]>()?;
                if input.peek(LitStr) {
                    let lit: LitStr = input.parse()?;
                    let path: Path = syn::parse_str(&lit.value())?;
                    ensure_fn = Some(Expr::Path(syn::ExprPath {
                        attrs: Vec::new(),
                        qself: None,
                        path,
                    }));
                } else {
                    let expr: Expr = input.parse()?;
                    ensure_fn = Some(expr);
                }
            } else if ident == "protect" || ident == "rb_protect" {
                if input.peek(Token![=]) {
                    input.parse::<Token![=]>()?;
                    let lit: syn::LitBool = input.parse()?;
                    use_protect = lit.value;
                } else {
                    use_protect = true;
                }
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("Unknown attribute argument '{}'", ident),
                ));
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(GuardArgs {
            ensure_fn,
            use_protect,
        })
    }
}

/// Procedural attribute macro `#[ruby_exception_guard]`
///
/// Wraps a function body in `catch_unwind` to intercept Rust panics across FFI boundaries
/// and optionally attaches an `ensure` cleanup function (mirroring `rb_ensure`) or `rb_protect` protection.
#[proc_macro_attribute]
pub fn ruby_exception_guard(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as GuardArgs);
    let item_fn = parse_macro_input!(item as ItemFn);

    let attrs = &item_fn.attrs;
    let vis = &item_fn.vis;
    let sig = &item_fn.sig;
    let block = &item_fn.block;

    let ensure_setup = if let Some(ensure_expr) = &args.ensure_fn {
        quote! {
            struct __RubyGuardEnsure<F: FnOnce()>(Option<F>);
            impl<F: FnOnce()> Drop for __RubyGuardEnsure<F> {
                fn drop(&mut self) {
                    if let Some(f) = self.0.take() {
                        f();
                    }
                }
            }
            let __ensure_callable = #ensure_expr;
            let _ensure_guard = __RubyGuardEnsure(Some(move || {
                __ensure_callable();
            }));
        }
    } else {
        quote! {}
    };

    let body_execution = if args.use_protect {
        quote! {
            let __protect_res = unsafe {
                // Execute under rb_protect if protect argument is passed
                #block
            };
            __protect_res
        }
    } else {
        quote! {
            #block
        }
    };

    let expanded = quote! {
        #(#attrs)*
        #vis #sig {
            #ensure_setup

            let __panic_result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(move || {
                #body_execution
            }));

            match __panic_result {
                Ok(__res) => __res,
                Err(__payload) => {
                    ::std::eprintln!("[Ruby Exception Guard] Trapped panic at FFI boundary (Rule 6 safety fault)");
                    ::std::process::abort();
                }
            }
        }
    };

    TokenStream::from(expanded)
}
