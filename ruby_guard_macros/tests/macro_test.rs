#[cfg(test)]
mod tests {
    use ruby_guard_macros::ruby_exception_guard;
    use std::sync::atomic::{AtomicBool, Ordering};

    static CLEANUP_CALLED: AtomicBool = AtomicBool::new(false);

    fn my_cleanup() {
        CLEANUP_CALLED.store(true, Ordering::SeqCst);
    }

    #[ruby_exception_guard(ensure = "my_cleanup")]
    fn protected_function_with_ensure(x: i32) -> i32 {
        x + 10
    }

    #[test]
    fn test_ruby_exception_guard_execution_and_ensure() {
        CLEANUP_CALLED.store(false, Ordering::SeqCst);
        let res = protected_function_with_ensure(5);
        assert_eq!(res, 15);
        assert!(CLEANUP_CALLED.load(Ordering::SeqCst));
    }
}
