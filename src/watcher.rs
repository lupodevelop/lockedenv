//! Background watcher thread for detecting environment changes (feature `watch`).

use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;

/// Handle for a running watcher thread; drop or call `stop()` to end it.
#[must_use = "the watcher stops immediately when the handle is dropped"]
pub struct WatchHandle {
    tx: mpsc::SyncSender<()>,
}

impl WatchHandle {
    /// Stop the watcher explicitly (equivalent to dropping the handle).
    pub fn stop(self) {
        drop(self);
    }
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        // Ignoring send errors: the thread may have already exited.
        let _ = self.tx.try_send(());
    }
}

/// Start the watcher thread, calling `on_drift(key, old, new)` on changes.
/// Returns a `WatchHandle`; panics if the thread cannot be spawned.
#[allow(clippy::missing_panics_doc)]
pub fn start(
    interval: Duration,
    mut on_drift: impl FnMut(&str, &str, &str) + Send + 'static,
) -> WatchHandle {
    let (tx, rx) = mpsc::sync_channel::<()>(1);

    std::thread::Builder::new()
        .name("env-lock-watcher".into())
        .spawn(move || {
            let mut snapshot: HashMap<String, String> = std::env::vars().collect();

            loop {
                // Block until a stop signal arrives or the interval elapses.
                match rx.recv_timeout(interval) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }

                let current: HashMap<String, String> = std::env::vars().collect();

                // Detect drifts with a pure functional pipeline, then fire the
                // callback inside `catch_unwind` so a panicking consumer never
                // kills the watcher thread silently.
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    current
                        .iter()
                        .filter_map(|(k, v)| {
                            snapshot.get(k).map_or_else(
                                || Some((k.as_str(), "<missing>", v.as_str())),
                                |old| (old != v).then_some((k.as_str(), old.as_str(), v.as_str())),
                            )
                        })
                        .chain(
                            snapshot
                                .iter()
                                .filter(|(k, _)| !current.contains_key(k.as_str()))
                                .map(|(k, old)| (k.as_str(), old.as_str(), "<removed>")),
                        )
                        .for_each(|(k, old, new)| on_drift(k, old, new));
                }));

                if result.is_err() {
                    #[cfg(feature = "tracing")]
                    tracing::error!("env-lock watcher: on_drift callback panicked");
                }

                // Always advance the snapshot — even after a panic — so the
                // same set of drifts is not re-reported on the next tick.
                snapshot = current;
            }
        })
        .expect("failed to spawn env-lock-watcher thread");

    WatchHandle { tx }
}
