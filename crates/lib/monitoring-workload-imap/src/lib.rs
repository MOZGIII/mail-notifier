//! Monitoring workload for IMAP mailbox monitoring.
//!
//! This crate provides a small helper to spawn generic monitoring tasks
//! under supervision.

use std::sync::Arc;

/// IMAP monitoring workload.
pub enum Mailbox {}

impl monitoring_core::Workload for Mailbox {
    type Item = Arc<config_bringup::data::Mailbox>;
    type Update = imap_checker::MailboxCounts;
    type Error = imap_monitor::MonitorError;

    async fn run<Notify, NotifyFut>(
        item: &Self::Item,
        notify: Notify,
    ) -> Result<core::convert::Infallible, Self::Error>
    where
        Notify: FnMut(Self::Update) -> NotifyFut + Send,
        NotifyFut: core::future::Future<Output = ()> + Send,
    {
        let imap_session = item.server.to_imap_session_params();

        let imap_monitor = imap_monitor::MonitorParams {
            imap_session,
            mailbox: &item.mailbox,
            idle_timeout: item.idle_timeout,
        };

        imap_monitor::monitor(imap_monitor, notify).await
    }
}
