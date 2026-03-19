//! Tokio runtime creation helpers.
//!
//! Wraps `tokio::runtime::Runtime::new()` with `anyhow::Result` error
//! handling and provides a convenience `block_on` function for running
//! a single future to completion.

/// Create a new multi-threaded Tokio runtime.
///
/// This is the standard runtime used by pleme-io applications for async
/// operations (MCP servers, daemon loops, network I/O).
pub fn create_runtime() -> anyhow::Result<tokio::runtime::Runtime> {
    tokio::runtime::Runtime::new()
        .map_err(|e| anyhow::anyhow!("failed to create tokio runtime: {e}"))
}

/// Create a Tokio runtime and block on a single future.
///
/// Convenience wrapper that combines [`create_runtime`] and
/// `Runtime::block_on`. The runtime is dropped after the future
/// completes.
pub fn block_on<F: std::future::Future>(f: F) -> anyhow::Result<F::Output> {
    let rt = create_runtime()?;
    Ok(rt.block_on(f))
}

/// Create a current-thread Tokio runtime.
///
/// Lighter weight than [`create_runtime`] — useful for simple CLI tools
/// that do not need multi-threaded scheduling.
pub fn create_current_thread_runtime() -> anyhow::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow::anyhow!("failed to create current-thread tokio runtime: {e}"))
}

/// Create a current-thread Tokio runtime and block on a single future.
///
/// Convenience wrapper combining [`create_current_thread_runtime`] and
/// `Runtime::block_on`.
pub fn block_on_current_thread<F: std::future::Future>(f: F) -> anyhow::Result<F::Output> {
    let rt = create_current_thread_runtime()?;
    Ok(rt.block_on(f))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_runtime_succeeds() {
        let rt = create_runtime();
        assert!(rt.is_ok());
    }

    #[test]
    fn block_on_executes_future() {
        let result = block_on(async { 42 });
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn block_on_returns_string() {
        let result = block_on(async { String::from("hello") });
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn block_on_with_async_block() {
        let result = block_on(async {
            let a = 10;
            let b = 20;
            a + b
        });
        assert_eq!(result.unwrap(), 30);
    }

    #[test]
    fn block_on_with_unit_future() {
        let result = block_on(async {});
        assert!(result.is_ok());
    }

    #[test]
    fn create_current_thread_runtime_succeeds() {
        let rt = create_current_thread_runtime();
        assert!(rt.is_ok());
    }

    #[test]
    fn block_on_current_thread_executes_future() {
        let result = block_on_current_thread(async { 99 });
        assert_eq!(result.unwrap(), 99);
    }

    #[test]
    fn block_on_current_thread_with_string() {
        let result = block_on_current_thread(async { String::from("current") });
        assert_eq!(result.unwrap(), "current");
    }

    #[test]
    fn runtime_can_spawn_tasks() {
        let rt = create_runtime().unwrap();
        let result = rt.block_on(async {
            let handle = tokio::spawn(async { 7 });
            handle.await.unwrap()
        });
        assert_eq!(result, 7);
    }

    #[test]
    fn runtime_can_run_async_computation() {
        let rt = create_runtime().unwrap();
        let result = rt.block_on(async {
            let a = 10;
            let b = tokio::spawn(async move { a * 2 }).await.unwrap();
            b
        });
        assert_eq!(result, 20);
    }

    #[test]
    fn multiple_runtimes_can_coexist() {
        let rt1 = create_runtime().unwrap();
        let rt2 = create_runtime().unwrap();
        let v1 = rt1.block_on(async { 1 });
        let v2 = rt2.block_on(async { 2 });
        assert_eq!(v1, 1);
        assert_eq!(v2, 2);
    }

    #[test]
    fn current_thread_runtime_can_spawn_local() {
        let rt = create_current_thread_runtime().unwrap();
        let result = rt.block_on(async {
            let local = tokio::task::spawn(async { 42 });
            local.await.unwrap()
        });
        assert_eq!(result, 42);
    }

    #[test]
    fn block_on_propagates_result_type() {
        let result: anyhow::Result<Result<i32, String>> = block_on(async { Ok(123) });
        assert_eq!(result.unwrap().unwrap(), 123);
    }

    #[test]
    fn block_on_with_error_result() {
        let result: anyhow::Result<Result<i32, &str>> = block_on(async { Err("fail") });
        assert!(result.unwrap().is_err());
    }
}
