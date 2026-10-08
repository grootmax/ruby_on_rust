extern crate proc_macro;
use proc_macro::TokenStream;

fn quote_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn find_fn_keyword(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i + 2 <= len {
        if &bytes[i..i + 2] == b"fn" {
            let prev_ok = i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_');
            let next_ok = i + 2 == len || (!bytes[i + 2].is_ascii_alphanumeric() && bytes[i + 2] != b'_');
            if prev_ok && next_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn parse_and_rewrite_fn(item_str: &str) -> Result<String, String> {
    let fn_pos = find_fn_keyword(item_str).ok_or_else(|| "Could not find 'fn' keyword".to_string())?;

    // Find the opening brace of the body
    let brace_pos = item_str.find('{').ok_or_else(|| "Missing opening brace '{' for function body".to_string())?;

    let header_and_sig = item_str[..brace_pos].trim();
    let body = &item_str[brace_pos..];

    // Extract function name after 'fn'
    let after_fn = item_str[fn_pos + 2..].trim_start();
    let fn_name_end = after_fn
        .find(|c: char| c == '(' || c == '<' || c.is_whitespace())
        .ok_or_else(|| "Invalid function signature".to_string())?;
    let fn_name = after_fn[..fn_name_end].trim();

    // Extract parameters inside (...)
    let param_start = item_str.find('(').ok_or_else(|| "Missing '(' in function parameters".to_string())?;
    let param_end = item_str[param_start..]
        .find(')')
        .ok_or_else(|| "Missing ')' in function parameters".to_string())?
        + param_start;
    let params_str = &item_str[param_start + 1..param_end];

    // Extract return type after ')'
    let after_params = item_str[param_end + 1..brace_pos].trim();
    let ret_type = if let Some(arrow_pos) = after_params.find("->") {
        after_params[arrow_pos + 2..].trim()
    } else {
        "()"
    };

    // Parse parameter names
    let mut param_names = Vec::new();
    if !params_str.trim().is_empty() {
        for p in params_str.split(',') {
            let p = p.trim();
            if p.is_empty() {
                continue;
            }
            if let Some(colon_pos) = p.find(':') {
                let name_part = p[..colon_pos].trim();
                let name = name_part.split_whitespace().last().unwrap_or(name_part);
                param_names.push(name);
            }
        }
    }

    let args_call = param_names.join(", ");
    let inner_fn_name = format!("__{}_inner", fn_name);

    let expanded = format!(
        r#"{header_and_sig} {{
            #[inline(always)]
            fn {inner_fn_name}({params_str}) -> {ret_type} {body}

            unsafe {{
                crate::ffi::boundary::execute_boundary(|| {{
                    {inner_fn_name}({args_call})
                }})
            }}
        }}"#,
        header_and_sig = header_and_sig,
        params_str = params_str,
        ret_type = ret_type,
        body = body,
        inner_fn_name = inner_fn_name,
        args_call = args_call,
    );

    Ok(expanded)
}

fn parse_and_rewrite_struct(item_str: &str) -> Result<String, String> {
    let struct_pos = item_str.find("struct ").ok_or_else(|| "Could not find 'struct' keyword".to_string())?;
    let after_struct = item_str[struct_pos + 7..].trim_start();
    let name_end = after_struct
        .find(|c: char| c == '{' || c == '<' || c == '(' || c.is_whitespace())
        .ok_or_else(|| "Invalid struct declaration".to_string())?;
    let struct_name = after_struct[..name_end].trim();

    let has_lifetime = after_struct.starts_with(&format!("{}<'", struct_name)) || after_struct.contains("<'");
    let impl_generics = if has_lifetime { "<'a>" } else { "" };

    let mut setters = String::new();
    if let Some(brace_start) = item_str.find('{') {
        if let Some(brace_end) = item_str.rfind('}') {
            let fields_str = &item_str[brace_start + 1..brace_end];
            for line in fields_str.split(&[';', ','][..]) {
                let line = line.trim();
                if line.is_empty() || line.starts_with("//") {
                    continue;
                }
                if let Some(colon) = line.find(':') {
                    let field_name_part = line[..colon].trim();
                    let field_name = field_name_part.split_whitespace().last().unwrap_or(field_name_part);

                    if !field_name.is_empty() {
                        let setter_name = format!("set_{}", field_name);
                        setters.push_str(&format!(
                            r#"
    pub fn {setter_name}(&mut self, vm: &crate::vm::RubyVm<'a>, val: crate::ffi::value::VALUE) {{
        crate::gc::write_barrier(&mut self.{field_name}, vm, val);
    }}
"#,
                            setter_name = setter_name,
                            field_name = field_name,
                        ));
                    }
                }
            }
        }
    }

    let expanded = format!(
        r#"{item_str}

impl{impl_generics} {struct_name}{impl_generics} {{
{setters}
}}
"#,
        item_str = item_str,
        impl_generics = impl_generics,
        struct_name = struct_name,
        setters = setters,
    );

    Ok(expanded)
}

#[proc_macro_attribute]
pub fn ruby_exception_boundary(attr: TokenStream, item: TokenStream) -> TokenStream {
    let _ = attr;
    let item_str = item.to_string();

    if !item_str.contains("fn ") && !item_str.contains("fn\n") {
        return "compile_error!(\"#[ruby_exception_boundary] can only be applied to functions\");"
            .parse()
            .unwrap();
    }

    match parse_and_rewrite_fn(&item_str) {
        Ok(expanded) => match expanded.parse() {
            Ok(ts) => ts,
            Err(_) => item,
        },
        Err(err_msg) => {
            format!("compile_error!({});", quote_string(&err_msg))
                .parse()
                .unwrap()
        }
    }
}

#[proc_macro_attribute]
pub fn ruby_gc_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
    let _ = attr;
    let item_str = item.to_string();

    if !item_str.contains("struct ") && !item_str.contains("struct\n") {
        return "compile_error!(\"#[ruby_gc_struct] can only be applied to structs\");"
            .parse()
            .unwrap();
    }

    match parse_and_rewrite_struct(&item_str) {
        Ok(expanded) => match expanded.parse() {
            Ok(ts) => ts,
            Err(_) => item,
        },
        Err(err_msg) => {
            format!("compile_error!({});", quote_string(&err_msg))
                .parse()
                .unwrap()
        }
    }
}

#[proc_macro_attribute]
pub fn gc_write_barrier(attr: TokenStream, item: TokenStream) -> TokenStream {
    ruby_gc_struct(attr, item)
}
