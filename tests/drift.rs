#[cfg(feature = "watch")]
mod drift {
    use lockedenv::load;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };

    /// Watcher fires the callback when an env var changes.
    #[test]
    fn watcher_detects_change() {
        std::env::set_var("DRIFT_CHANGE", "1");
        let _config = load! { DRIFT_CHANGE: String };

        let seen = Arc::new(AtomicBool::new(false));
        let seen2 = seen.clone();

        let _handle = lockedenv::watch!(
            keys = ["DRIFT_CHANGE"],
            interval_ms = 10,
            on_drift = move |key: &str, _old: &str, _new: &str| {
                if key == "DRIFT_CHANGE" {
                    seen2.store(true, Ordering::SeqCst);
                }
            }
        );

        // Let the watcher take its initial snapshot, then mutate.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::env::set_var("DRIFT_CHANGE", "2");
        std::thread::sleep(std::time::Duration::from_millis(50));

        assert!(seen.load(Ordering::SeqCst), "watcher should detect the changed variable");
    }

    /// Watcher detects removal of an env var.
    #[test]
    fn watcher_detects_removal() {
        std::env::set_var("DRIFT_REMOVAL", "present");
        let _config = load! { DRIFT_REMOVAL: String };

        let removed = Arc::new(AtomicBool::new(false));
        let removed2 = removed.clone();

        let _handle = lockedenv::watch!(
            keys = ["DRIFT_REMOVAL"],
            interval_ms = 10,
            on_drift = move |key: &str, _old: &str, new: &str| {
                if key == "DRIFT_REMOVAL" && new == "<removed>" {
                    removed2.store(true, Ordering::SeqCst);
                }
            }
        );

        std::thread::sleep(std::time::Duration::from_millis(20));
        std::env::remove_var("DRIFT_REMOVAL");
        std::thread::sleep(std::time::Duration::from_millis(50));

        assert!(removed.load(Ordering::SeqCst), "watcher should fire when a variable is removed");
    }

    /// After the `WatchHandle` is dropped, no further callbacks fire.
    #[test]
    fn watcher_stops_on_drop() {
        std::env::set_var("DRIFT_STOP_TEST", "initial");
        let _config = load! { DRIFT_STOP_TEST: String };

        let call_count = Arc::new(AtomicUsize::new(0));
        let cc2 = call_count.clone();

        let handle = lockedenv::watch!(
            keys = ["DRIFT_STOP_TEST"],
            interval_ms = 10,
            on_drift = move |key: &str, _old: &str, _new: &str| {
                if key == "DRIFT_STOP_TEST" {
                    cc2.fetch_add(1, Ordering::SeqCst);
                }
            }
        );

        // Wait for the watcher to snap the initial state, then stop it.
        std::thread::sleep(std::time::Duration::from_millis(30));
        drop(handle); // sends stop signal to background thread
        std::thread::sleep(std::time::Duration::from_millis(20)); // let thread exit

        let count_before = call_count.load(Ordering::SeqCst);

        // Change the var after the handle is gone — no callback should fire.
        std::env::set_var("DRIFT_STOP_TEST", "after_drop");
        std::thread::sleep(std::time::Duration::from_millis(50));

        assert_eq!(
            call_count.load(Ordering::SeqCst),
            count_before,
            "watcher must not fire after the handle is dropped",
        );
    }

    /// Multiple env var changes are all reported.
    #[test]
    fn watcher_reports_multiple_changes() {
        std::env::set_var("DRIFT_MULTI_A", "0");
        std::env::set_var("DRIFT_MULTI_B", "0");
        let _cfg = load! { DRIFT_MULTI_A: u32, DRIFT_MULTI_B: u32 };

        let count = Arc::new(AtomicUsize::new(0));
        let c2 = count.clone();

        let _handle = lockedenv::watch!(
            keys = ["DRIFT_MULTI_A", "DRIFT_MULTI_B"],
            interval_ms = 10,
            on_drift = move |_key: &str, _old: &str, _new: &str| {
                c2.fetch_add(1, Ordering::SeqCst);
            }
        );

        std::thread::sleep(std::time::Duration::from_millis(20));
        std::env::set_var("DRIFT_MULTI_A", "1");
        std::env::set_var("DRIFT_MULTI_B", "1");
        std::thread::sleep(std::time::Duration::from_millis(60));

        assert!(
            count.load(Ordering::SeqCst) >= 2,
            "expected at least 2 drift callbacks, got {}",
            count.load(Ordering::SeqCst),
        );
    }

    /// Watcher detects a newly added variable (was absent at snapshot time).
    #[test]
    fn watcher_detects_addition() {
        std::env::remove_var("DRIFT_ADDITION");

        let seen = Arc::new(AtomicBool::new(false));
        let seen2 = seen.clone();

        let _handle = lockedenv::watch!(
            keys = ["DRIFT_ADDITION"],
            interval_ms = 10,
            on_drift = move |key: &str, old: &str, _new: &str| {
                if key == "DRIFT_ADDITION" && old == "<missing>" {
                    seen2.store(true, Ordering::SeqCst);
                }
            }
        );

        std::thread::sleep(std::time::Duration::from_millis(20));
        std::env::set_var("DRIFT_ADDITION", "appeared");
        std::thread::sleep(std::time::Duration::from_millis(50));

        assert!(seen.load(Ordering::SeqCst), "watcher should detect a newly added variable");
    }

    /// Watcher with empty key list never fires any callbacks.
    #[test]
    fn watcher_empty_keys_never_fires() {
        let count = Arc::new(AtomicUsize::new(0));
        let c2 = count.clone();

        let _handle = lockedenv::watch!(
            keys = [],
            interval_ms = 10,
            on_drift = move |_key: &str, _old: &str, _new: &str| {
                c2.fetch_add(1, Ordering::SeqCst);
            }
        );

        // Mutate something unrelated — the watcher should not fire.
        std::env::set_var("UNRELATED_VAR_XYZ", "value");
        std::thread::sleep(std::time::Duration::from_millis(50));

        assert_eq!(
            count.load(Ordering::SeqCst),
            0,
            "watcher with no keys must never fire",
        );
    }
}
