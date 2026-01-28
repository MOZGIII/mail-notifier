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
    type Error = imap_monitor::Error;

    async fn run<Notify, NotifyFut>(
        item: &Self::Item,
        notify: Notify,
    ) -> Result<core::convert::Infallible, Self::Error>
    where
        Notify: FnMut(Self::Update) -> NotifyFut + Send,
        NotifyFut: core::future::Future<Output = ()> + Send,
    {
        let params = setup(item);
        imap_monitor::monitor(params, notify).await
    }
}

/// Setup the workload.
fn setup(mailbox: &config_bringup::data::Mailbox) -> imap_monitor::Params<'_> {
    let config_bringup::data::Mailbox {
        server,
        mailbox,
        idle_timeout,
    } = mailbox;

    let config_bringup::data::Server {
        server_name: _,
        host,
        port,
        tls_mode,
        tls_server_name,
        auth,
    } = server.as_ref();

    let connect = imap_connect::Params {
        host,
        port: *port,
        tls_mode: *tls_mode,
        tls_server_name,
    };

    let auth = match auth {
        config_bringup::data::ServerAuth::Login { username, password } => {
            imap_auth::Params::Login { username, password }
        }
        config_bringup::data::ServerAuth::OAuth2Credentials { user, access_token } => {
            imap_auth::Params::OAuth2 { user, access_token }
        }
    };

    let session = imap_session::Params { connect, auth };

    imap_monitor::Params {
        session,
        mailbox,
        idle_timeout: *idle_timeout,
    }
}
