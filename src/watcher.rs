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
///
/// Only the variables listed in `keys` are monitored.  On each tick the
/// watcher reads exactly those keys via [`std::env::var`] — O(watched vars)
/// instead of O(all env vars) — making it safe even in processes that have
/// hundreds of environment variables.
///
/// `"<removed>"` is passed as `new` when a variable disappears;
/// `"<missing>"` is passed as `old` when a variable is newly added.
///
/// Returns a `WatchHandle`; panics if the thread cannot be spawned.
#[allow(clippy::missing_panics_doc)]
pub fn start(
    keys: Vec<String>,
    interval: Duration,
    mut on_drift: impl FnMut(&str, &str, &str) + Send + 'static,
) -> WatchHandle {
    let (tx, rx) = mpsc::sync_channel::<()>(1);

    std::thread::Builder::new()
        .name("lockedenv-watcher".into())
        .spawn(move || {
            // Snapshot only the watched keys — avoids iterating all env vars.
            let mut snapshot: HashMap<String, String> = keys
                .iter()
                .filter_map(|k| std::env::var(k).ok().map(|v| (k.clone(), v)))
                .collect();

            loop {
                // Block until a stop signal arrives or the interval elapses.
                match rx.recv_timeout(interval) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }

                // Check only the watched keys — never touches unrelated vars.
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    for key in &keys {
                        let current = std::env::var(key).ok();
                        match (snapshot.get(key.as_str()), current.as_deref()) {
                            (Some(old), Some(new)) if old != new => on_drift(key, old, new),
                            (Some(old), None) => on_drift(key, old, "<removed>"),
                            (None, Some(new)) => on_drift(key, "<missing>", new),
                            _ => {}
                        }
                    }
                }));

                if result.is_err() {
                    #[cfg(feature = "tracing")]
                    tracing::error!("lockedenv watcher: on_drift callback panicked");
                }

                // Always advance the snapshot — even after a panic — so the
                // same set of drifts is not re-reported on the next tick.
                for key in &keys {
                    match std::env::var(key).ok() {
                        Some(v) => {
                            snapshot.insert(key.clone(), v);
                        }
                        None => {
                            snapshot.remove(key.as_str());
                        }
                    }
                }
            }
        })
        .expect("failed to spawn lockedenv-watcher thread");

    WatchHandle { tx }
}
