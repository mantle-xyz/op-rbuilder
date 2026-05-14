use reth_node_api::PayloadBuilderError;
use reth_tasks::TaskExecutor;

/// Extension trait for [`TaskExecutor`] that adds a helper for awaiting the result of
/// a blocking closure from async code.
///
/// In reth v1.9, [`TaskExecutor::spawn_blocking`] takes a `Future`, not a closure.
/// We use [`tokio::task::spawn_blocking`] directly for closure execution.
pub(crate) trait RuntimeExt {
    /// Spawn a blocking closure on a blocking thread and await its result.
    fn run_blocking_task<T, F>(
        &self,
        task: F,
    ) -> impl std::future::Future<Output = Result<T, PayloadBuilderError>> + Send
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, PayloadBuilderError> + Send + 'static;
}

impl RuntimeExt for TaskExecutor {
    fn run_blocking_task<T, F>(
        &self,
        task: F,
    ) -> impl std::future::Future<Output = Result<T, PayloadBuilderError>> + Send
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, PayloadBuilderError> + Send + 'static,
    {
        let handle = tokio::task::spawn_blocking(task);
        async move {
            handle
                .await
                .map_err(|e| PayloadBuilderError::Other(Box::new(e)))?
        }
    }
}
