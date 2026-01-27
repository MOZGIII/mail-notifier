//! Monitoring core.

/// A monitoring workload.
pub trait Workload {
    /// An item that specified the workload.
    type Item: Clone + Send + Sync + 'static;

    /// An update from the workload.
    type Update: Send + Sync + 'static;

    /// An error that the workload may fail with.
    type Error: Send + Sync + 'static;

    /// Run the workload.
    fn run<Notify, NotifyFut>(
        item: &Self::Item,
        notify: Notify,
    ) -> impl std::future::Future<Output = Result<core::convert::Infallible, Self::Error>>
    + std::marker::Send
    where
        Notify: FnMut(Self::Update) -> NotifyFut + Send,
        NotifyFut: core::future::Future<Output = ()> + Send;
}
