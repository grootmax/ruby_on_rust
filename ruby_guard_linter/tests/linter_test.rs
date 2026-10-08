#[cfg(test)]
mod tests {
    use ruby_guard_linter::lint_source;

    #[test]
    fn test_unprotected_drop_variable_across_ffi_call_triggers_rule_6() {
        let code = r#"
            fn unprotected_example() {
                let vec_var = Vec::new();
                unsafe { extern_c_call(); }
            }
        "#;

        let diagnostics = lint_source(code).expect("Parsing code failed");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("Rule 6 Violation"));
        assert_eq!(diagnostics[0].var_name, "vec_var");
    }

    #[test]
    fn test_guarded_function_allows_drop_variables() {
        let code = r#"
            #[ruby_exception_guard]
            fn guarded_example() {
                let vec_var = Vec::new();
                unsafe { extern_c_call(); }
            }
        "#;

        let diagnostics = lint_source(code).expect("Parsing code failed");
        assert_eq!(diagnostics.len(), 0);
    }
}
