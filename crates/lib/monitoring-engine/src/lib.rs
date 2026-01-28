//! Monitoring engine.
//!
//! This crate provides a small helper to spawn mailbox-monitoring tasks
//! (from `imap-monitor`) under supervision.

use std::sync::Arc;

/// Generic update produced by the engine.
///
/// `Entry` identifies which entry the update refers to and `Payload`
/// carries the actual update data.
#[derive(Debug)]
pub struct Update<Entry, Payload> {
    /// The entry this update belongs to.
    pub entry: Entry,

    /// The payload of the update.
    pub payload: Payload,
}

/// Mailbox update carrying [`imap_checker::MailboxCounts`].
pub type MailboxUpdate<Entry> = Update<Entry, imap_checker::MailboxCounts>;

/// Supervisor update carrying [`supervisor::SupervisorEvent`] values.
pub type SupervisorUpdate<Entry> = Update<
    Entry,
    supervisor::SupervisorEvent<core::convert::Infallible, imap_monitor::MonitorError>,
>;

/// Parameters for spawning monitors.
///
/// This type bundles borrowed inputs used by `spawn_monitors`.
pub struct SpawnMonitorsParams<'a, RegisterState, MailboxNotify, SupervisorNotify> {
    /// Slice of monitor configurations to spawn.
    pub mailboxes: &'a [config_bringup::data::Mailbox],

    /// Callback used to register each config and produce an `EntryKey`.
    pub register_state: RegisterState,

    /// `JoinSet` to spawn tasks into.
    pub join_set: &'a mut tokio::task::JoinSet<()>,

    /// Notofier used to report mailbox updates.
    pub mailbox_notify: MailboxNotify,

    /// Notifier used to report supervisor events.
    pub supervisor_notify: SupervisorNotify,
}

/// Spawn monitor tasks for the provided configs.
pub fn spawn_monitors<
    Entry,
    RegisterState,
    MailboxNotify,
    MailboxNotifyFut,
    SupervisorNotify,
    SupervisorNotifyFut,
>(
    params: SpawnMonitorsParams<'_, RegisterState, MailboxNotify, SupervisorNotify>,
) where
    Entry: Clone + Send + 'static,
    RegisterState: for<'c> FnMut(&'c config_bringup::data::Mailbox) -> Entry,
    MailboxNotify: FnMut(MailboxUpdate<Entry>) -> MailboxNotifyFut + Clone + Send + Sync + 'static,
    MailboxNotifyFut: Future<Output = ()> + Send,
    SupervisorNotify:
        FnMut(SupervisorUpdate<Entry>) -> SupervisorNotifyFut + Clone + Send + Sync + 'static,
    SupervisorNotifyFut: Future<Output = ()> + Send,
{
    let SpawnMonitorsParams {
        mailboxes,
        join_set,
        mailbox_notify,
        supervisor_notify,
        mut register_state,
    } = params;

    for config in mailboxes {
        let entry = (register_state)(config);

        let config = Arc::new(config.clone());

        let mailbox_notify = mailbox_notify.clone();
        let supervisor_notify = supervisor_notify.clone();

        join_set.spawn(async move {
            let entry = entry.clone();
            let work = {
                let entry = entry.clone();
                move || {
                    let config = Arc::clone(&config);

                    let mailbox_notify = {
                        let entry = entry.clone();
                        let mailbox_notify = mailbox_notify.clone();
                        move |counts| {
                            let entry = entry.clone();
                            let mut mailbox_notify = mailbox_notify.clone();
                            async move {
                                (mailbox_notify)(Update {
                                    entry,
                                    payload: counts,
                                })
                                .await;
                            }
                        }
                    };

                    std::panic::AssertUnwindSafe(async move {
                        let imap_session = config.server.to_imap_session_params();

                        let imap_monitor = imap_monitor::MonitorParams {
                            imap_session,
                            mailbox: &config.mailbox,
                            idle_timeout: config.idle_timeout,
                        };

                        imap_monitor::monitor(imap_monitor, mailbox_notify).await
                    })
                }
            };

            let retries_backoff = exp_backoff::State {
                factor: 2,
                max: core::time::Duration::from_secs(30),
                value: core::time::Duration::from_secs(1),
            };

            let supervisor_notify = move |event| {
                let entry = entry.clone();
                let mut supervisor_notify = supervisor_notify.clone();
                async move {
                    (supervisor_notify)(SupervisorUpdate {
                        entry,
                        payload: event,
                    })
                    .await;
                }
            };

            supervisor::run(supervisor::Params {
                work,
                notifier: supervisor_notify,
                sleep: tokio::time::sleep,
                retries_backoff,
            })
            .await
        });
    }

    drop(mailbox_notify);
    drop(supervisor_notify);
}
