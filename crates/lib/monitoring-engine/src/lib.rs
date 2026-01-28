//! Monitoring engine.
//!
//! This crate provides a small helper to spawn generic monitoring tasks
//! under supervision.

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

/// Workload update.
pub type WorkloadUpdate<Entry, Workload> =
    Update<Entry, <Workload as monitoring_core::Workload>::Update>;

/// Supervisor update carrying [`supervisor::SupervisorEvent`] values.
pub type SupervisorUpdate<Entry, Workload> = Update<
    Entry,
    supervisor::SupervisorEvent<
        core::convert::Infallible,
        <Workload as monitoring_core::Workload>::Error,
    >,
>;

/// Parameters for spawning monitors.
///
/// This type bundles borrowed inputs used by [`spawn_monitors`].
pub struct SpawnMonitorsParams<'a, WorkloadItem, RegisterState, WorkloadNotify, SupervisorNotify> {
    /// Slice of workload items to spawn.
    pub workload_items: &'a [WorkloadItem],

    /// Callback used to register each workload and produce an `EntryKey`.
    pub register_state: RegisterState,

    /// `JoinSet` to spawn tasks into.
    pub join_set: &'a mut tokio::task::JoinSet<()>,

    /// Notifier used to report monitored workload updates.
    pub workload_notify: WorkloadNotify,

    /// Notifier used to report supervisor events.
    pub supervisor_notify: SupervisorNotify,
}

/// Spawn monitor tasks for the provided configs.
pub fn spawn_monitors<
    Workload,
    Entry,
    RegisterState,
    WorkloadNotify,
    WorkloadNotifyFut,
    SupervisorNotify,
    SupervisorNotifyFut,
>(
    params: SpawnMonitorsParams<
        '_,
        Workload::Item,
        RegisterState,
        WorkloadNotify,
        SupervisorNotify,
    >,
) where
    Workload: monitoring_core::Workload,
    Entry: Clone + Send + 'static,
    RegisterState: for<'item> FnMut(&'item Workload::Item) -> Entry,
    WorkloadNotify:
        FnMut(Update<Entry, Workload::Update>) -> WorkloadNotifyFut + Clone + Send + Sync + 'static,
    WorkloadNotifyFut: Future<Output = ()> + Send,
    SupervisorNotify: FnMut(SupervisorUpdate<Entry, Workload>) -> SupervisorNotifyFut
        + Clone
        + Send
        + Sync
        + 'static,
    SupervisorNotifyFut: Future<Output = ()> + Send,
{
    let SpawnMonitorsParams {
        workload_items,
        join_set,
        workload_notify,
        supervisor_notify,
        mut register_state,
    } = params;

    for workload_item in workload_items {
        let entry = (register_state)(workload_item);

        let workload_item = workload_item.clone();

        let workload_notify = workload_notify.clone();
        let supervisor_notify = supervisor_notify.clone();

        join_set.spawn(async move {
            let entry = entry.clone();
            let workload_item = workload_item.clone();

            let work = {
                let entry = entry.clone();
                let workload_item = workload_item.clone();
                move || {
                    let workload_item = workload_item.clone();

                    let workload_notify = {
                        let entry = entry.clone();
                        let workload_notify = workload_notify.clone();
                        move |payload| {
                            let entry = entry.clone();
                            let mut workload_notify = workload_notify.clone();
                            async move {
                                (workload_notify)(WorkloadUpdate::<Entry, Workload> {
                                    entry,
                                    payload,
                                })
                                .await;
                            }
                        }
                    };

                    std::panic::AssertUnwindSafe(async move {
                        Workload::run(&workload_item, workload_notify).await
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
                    (supervisor_notify)(SupervisorUpdate::<Entry, Workload> {
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

    drop(workload_notify);
    drop(supervisor_notify);
}
