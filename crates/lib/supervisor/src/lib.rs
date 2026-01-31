//! Lightweight async harness for supervised task execution.

extern crate alloc;

use futures_util::FutureExt;
use std::future::Future;
use std::panic::UnwindSafe;
use std::time::{Duration, Instant};

/// The panic payload type alias.
type PanicPayload = alloc::boxed::Box<dyn core::any::Any + Send + 'static>;

/// Event sent to the notifier.
#[derive(Debug)]
pub enum SupervisorEvent<T, E> {
    /// The work is about to be invoked.
    Started,

    /// The work has completed without an error or panic.
    ///
    /// It won't be restarted.
    Done {
        /// The returned value.
        value: T,
    },

    /// The work returned an error.
    ///
    /// It may be restarted.
    Error {
        /// The error that was returned by the work future.
        error: E,

        /// The time to wait before the next attempt.
        next_retry_in: Duration,
    },

    /// The work panicked.
    ///
    /// It will be restarted.
    Panicked {
        /// The captured panic payload.
        panic_payload: PanicPayload,

        /// The time to wait before the next attempt.
        next_retry_in: Duration,
    },
}

/// Parameters for `run`. Generic over the work and notifier closure types
/// and their returned futures. Runs a single async work item and reports events.
pub struct Params<Work, Notifier, Sleep> {
    /// The work to run.
    pub work: Work,

    /// Notifier for events.
    pub notifier: Notifier,

    /// Sleep timer.
    pub sleep: Sleep,

    /// The exponential backoff configuration for the retries.
    pub retries_backoff: exp_backoff::State,

    /// Cooldown period for resetting backoff.
    pub cooldown: Duration,
}

/// Run once: notify start, run work, notify result.
pub async fn run<Work, WorkFut, Notifier, NotifierFut, Sleep, SleepFut, Value, Error>(
    mut params: Params<Work, Notifier, Sleep>,
) where
    Work: FnMut() -> WorkFut,
    WorkFut: Future<Output = Result<Value, Error>> + UnwindSafe,
    Notifier: FnMut(SupervisorEvent<Value, Error>) -> NotifierFut,
    NotifierFut: Future<Output = ()>,
    Sleep: FnMut(Duration) -> SleepFut,
    SleepFut: Future<Output = ()>,
{
    let initial_delay = params.retries_backoff.value;
    let mut last_reset = Instant::now();

    loop {
        (params.notifier)(SupervisorEvent::Started).await;

        // Run the work and catch panics coming from the future.
        let work_future = core::panic::AssertUnwindSafe(async { (params.work)().await });
        let result = work_future.catch_unwind().await;

        if last_reset.elapsed() >= params.cooldown {
            params.retries_backoff.value = initial_delay;
            last_reset = Instant::now();
        }
        let delay = params.retries_backoff.take_and_advance();

        let event = match result {
            Ok(Ok(value)) => {
                (params.notifier)(SupervisorEvent::Done { value }).await;
                return;
            }
            Ok(Err(error)) => SupervisorEvent::Error {
                error,
                next_retry_in: delay,
            },
            Err(panic_payload) => SupervisorEvent::Panicked {
                panic_payload,
                next_retry_in: delay,
            },
        };

        (params.notifier)(event).await;

        (params.sleep)(delay).await;
    }
}
