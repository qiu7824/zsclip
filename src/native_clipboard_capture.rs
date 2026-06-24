use crate::app_core::ClipboardHost;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeClipboardCaptureResult {
    pub(crate) inserted: bool,
    pub(crate) item_id: Option<i64>,
    pub(crate) reason: &'static str,
}

impl NativeClipboardCaptureResult {
    fn from_db(outcome: crate::db_runtime::NativeClipboardInsertOutcome) -> Self {
        Self {
            inserted: outcome.inserted,
            item_id: outcome.item_id,
            reason: outcome.reason,
        }
    }

    fn ignored(reason: &'static str) -> Self {
        Self {
            inserted: false,
            item_id: None,
            reason,
        }
    }
}

pub(crate) struct NativeClipboardCaptureService;

impl NativeClipboardCaptureService {
    pub(crate) fn capture_current<H: ClipboardHost>(
        category: i64,
        source_app: &str,
    ) -> NativeClipboardCaptureResult {
        if H::should_ignore_capture_by_named_format() {
            return NativeClipboardCaptureResult::ignored("ignored_self_write");
        }

        if let Some(paths) = H::read_file_paths().filter(|paths| !paths.is_empty()) {
            return crate::db_runtime::insert_native_clipboard_file_paths(
                category, &paths, source_app,
            )
            .map(NativeClipboardCaptureResult::from_db)
            .unwrap_or_else(|_| NativeClipboardCaptureResult::ignored("db_error"));
        }

        if let Some((bytes, width, height)) = H::read_image_rgba() {
            return crate::db_runtime::insert_native_clipboard_image(
                category, &bytes, width, height, source_app,
            )
            .map(NativeClipboardCaptureResult::from_db)
            .unwrap_or_else(|_| NativeClipboardCaptureResult::ignored("db_error"));
        }

        if let Some(text) = H::read_text() {
            return crate::db_runtime::insert_native_clipboard_text(category, &text, source_app)
                .map(NativeClipboardCaptureResult::from_db)
                .unwrap_or_else(|_| NativeClipboardCaptureResult::ignored("db_error"));
        }

        NativeClipboardCaptureResult::ignored("empty_clipboard")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_core::ClipboardHost;
    use std::cell::RefCell;
    use std::sync::{Mutex, OnceLock};

    thread_local! {
        static TEXT: RefCell<Option<String>> = const { RefCell::new(None) };
        static SEQUENCE: RefCell<u32> = const { RefCell::new(1) };
        static IGNORE: RefCell<bool> = const { RefCell::new(false) };
    }

    struct TestClipboardHost;

    impl TestClipboardHost {
        fn set_text(text: &str) {
            TEXT.with(|slot| *slot.borrow_mut() = Some(text.to_string()));
            SEQUENCE.with(|slot| {
                let next = slot.borrow().saturating_add(1);
                *slot.borrow_mut() = next;
            });
            IGNORE.with(|slot| *slot.borrow_mut() = false);
        }
    }

    impl ClipboardHost for TestClipboardHost {
        fn read_text() -> Option<String> {
            TEXT.with(|slot| slot.borrow().clone())
        }

        fn write_text(text: &str) -> bool {
            Self::set_text(text);
            true
        }

        fn read_image_rgba() -> Option<(Vec<u8>, usize, usize)> {
            None
        }

        fn write_image_rgba(_bytes: &[u8], _width: usize, _height: usize) -> bool {
            false
        }

        fn read_file_paths() -> Option<Vec<String>> {
            None
        }

        fn write_file_paths(_paths: &[String]) -> bool {
            false
        }

        fn sequence_number() -> u32 {
            SEQUENCE.with(|slot| *slot.borrow())
        }

        fn write_text_ignored_by_monitors(text: &str) -> bool {
            Self::set_text(text);
            IGNORE.with(|slot| *slot.borrow_mut() = true);
            true
        }

        fn should_ignore_capture_by_named_format() -> bool {
            IGNORE.with(|slot| {
                let ignore = *slot.borrow();
                *slot.borrow_mut() = false;
                ignore
            })
        }
    }

    fn db_test_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("native clipboard capture DB test lock poisoned")
    }

    #[test]
    fn native_clipboard_capture_inserts_text_and_dedupes() {
        let _guard = db_test_guard();
        let text = format!(
            "native capture smoke text {:?}",
            std::time::SystemTime::now()
        );
        TestClipboardHost::set_text(&text);
        let first =
            NativeClipboardCaptureService::capture_current::<TestClipboardHost>(0, "test-host");
        assert!(first.inserted);
        assert!(first.item_id.is_some());

        TestClipboardHost::set_text(&text);
        let duplicate =
            NativeClipboardCaptureService::capture_current::<TestClipboardHost>(0, "test-host");
        assert!(!duplicate.inserted);
        assert_eq!(duplicate.reason, "duplicate");
        assert_eq!(duplicate.item_id, first.item_id);
    }
}
