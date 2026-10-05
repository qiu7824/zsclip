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
        Self::capture_current_with_html::<H>(category, source_app, || None)
    }

    pub(crate) fn capture_current_with_html<H: ClipboardHost>(
        category: i64,
        source_app: &str,
        read_html: impl FnOnce() -> Option<String>,
    ) -> NativeClipboardCaptureResult {
        let sequence = H::sequence_number();
        if H::should_ignore_capture_by_named_format() {
            return NativeClipboardCaptureResult::ignored("ignored_self_write");
        }

        if let Some(paths) = H::read_file_paths().filter(|paths| !paths.is_empty()) {
            if sequence != H::sequence_number() {
                return NativeClipboardCaptureResult::ignored("clipboard_changed");
            }
            return crate::db_runtime::insert_native_clipboard_file_paths(
                category, &paths, source_app,
            )
            .map(NativeClipboardCaptureResult::from_db)
            .unwrap_or_else(|_| NativeClipboardCaptureResult::ignored("db_error"));
        }

        // Office/browser selections can advertise an image as well as HTML.
        // Prefer their editable text+HTML representation over the bitmap fallback.
        let html = read_html();
        if let Some(html) = html.as_ref() {
            if let Some(text) = H::read_text() {
                if sequence != H::sequence_number() {
                    return NativeClipboardCaptureResult::ignored("clipboard_changed");
                }
                return crate::db_runtime::insert_native_clipboard_rich_text(
                    category, &text, html, source_app,
                )
                .map(NativeClipboardCaptureResult::from_db)
                .unwrap_or_else(|_| NativeClipboardCaptureResult::ignored("db_error"));
            }
        }

        if let Some((bytes, width, height)) = H::read_image_rgba() {
            if sequence != H::sequence_number() {
                return NativeClipboardCaptureResult::ignored("clipboard_changed");
            }
            return crate::db_runtime::insert_native_clipboard_image(
                category, &bytes, width, height, source_app,
            )
            .map(NativeClipboardCaptureResult::from_db)
            .unwrap_or_else(|_| NativeClipboardCaptureResult::ignored("db_error"));
        }

        if let Some(text) = H::read_text() {
            if sequence != H::sequence_number() {
                return NativeClipboardCaptureResult::ignored("clipboard_changed");
            }
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
        static IMAGE: RefCell<Option<(Vec<u8>, usize, usize)>> = const { RefCell::new(None) };
        static FILES: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
        static SEQUENCE: RefCell<u32> = const { RefCell::new(1) };
        static IGNORE: RefCell<bool> = const { RefCell::new(false) };
    }

    struct TestClipboardHost;

    impl TestClipboardHost {
        fn set_text(text: &str) {
            TEXT.with(|slot| *slot.borrow_mut() = Some(text.to_string()));
            IMAGE.with(|slot| *slot.borrow_mut() = None);
            FILES.with(|slot| *slot.borrow_mut() = None);
            Self::bump_sequence();
            IGNORE.with(|slot| *slot.borrow_mut() = false);
        }

        fn set_image(bytes: Vec<u8>, width: usize, height: usize) {
            TEXT.with(|slot| *slot.borrow_mut() = None);
            IMAGE.with(|slot| *slot.borrow_mut() = Some((bytes, width, height)));
            FILES.with(|slot| *slot.borrow_mut() = None);
            Self::bump_sequence();
            IGNORE.with(|slot| *slot.borrow_mut() = false);
        }

        fn set_files(paths: Vec<String>) {
            TEXT.with(|slot| *slot.borrow_mut() = Some(paths.join("\n")));
            IMAGE.with(|slot| *slot.borrow_mut() = Some((vec![1, 2, 3, 4], 1, 1)));
            FILES.with(|slot| *slot.borrow_mut() = Some(paths));
            Self::bump_sequence();
            IGNORE.with(|slot| *slot.borrow_mut() = false);
        }

        fn bump_sequence() {
            SEQUENCE.with(|slot| {
                let next = slot.borrow().saturating_add(1);
                *slot.borrow_mut() = next;
            });
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
            IMAGE.with(|slot| slot.borrow().clone())
        }

        fn write_image_rgba(_bytes: &[u8], _width: usize, _height: usize) -> bool {
            false
        }

        fn read_file_paths() -> Option<Vec<String>> {
            FILES.with(|slot| slot.borrow().clone())
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
        crate::db_runtime::with_test_db(|| {
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
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn native_clipboard_capture_inserts_files_before_other_payloads() {
        let _guard = db_test_guard();
        crate::db_runtime::with_test_db(|| {
            TestClipboardHost::set_files(vec![
                "/tmp/native-capture-a.txt".to_string(),
                "/tmp/native-capture-b.txt".to_string(),
            ]);
            let result =
                NativeClipboardCaptureService::capture_current::<TestClipboardHost>(0, "files");
            assert!(result.inserted);
            let item = crate::db_runtime::native_clip_item(result.item_id.unwrap())?.unwrap();
            assert_eq!(item.kind, crate::app_core::ClipKind::Files);
            assert_eq!(
                item.file_paths,
                Some(vec![
                    "/tmp/native-capture-a.txt".to_string(),
                    "/tmp/native-capture-b.txt".to_string()
                ])
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn native_clipboard_capture_inserts_images() {
        let _guard = db_test_guard();
        crate::db_runtime::with_test_db(|| {
            TestClipboardHost::set_image(vec![255, 0, 0, 255], 1, 1);
            let result =
                NativeClipboardCaptureService::capture_current::<TestClipboardHost>(0, "image");
            assert!(result.inserted);
            let item = crate::db_runtime::native_clip_item(result.item_id.unwrap())?.unwrap();
            assert_eq!(item.kind, crate::app_core::ClipKind::Image);
            assert_eq!(item.image_width, 1);
            assert_eq!(item.image_height, 1);
            assert_eq!(item.image_bytes, Some(vec![255, 0, 0, 255]));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn native_capture_prefers_editable_html_over_an_advertised_bitmap() {
        let _guard = db_test_guard();
        crate::db_runtime::with_test_protected_texts(&[], || crate::db_runtime::with_test_db(|| {
            TestClipboardHost::set_text("Cell A\tCell B\nRow two");
            IMAGE.with(|slot| *slot.borrow_mut() = Some((vec![255, 0, 0, 255], 1, 1)));
            let result = NativeClipboardCaptureService::capture_current_with_html::<TestClipboardHost>(0, "office", || Some("<table><tr><td>Cell A</td><td>Cell B</td></tr><tr><td colspan='2'>Row two</td></tr></table>".into()));
            assert!(result.inserted);
            let item = crate::db_runtime::native_clip_item(result.item_id.unwrap())?.unwrap();
            assert_eq!(item.kind, crate::app_core::ClipKind::Text);
            assert_eq!(item.text.as_deref(), Some("Cell A\tCell B\nRow two"));
            assert!(item.rich_text_html.unwrap().contains("<table>"));
            assert!(item.image_bytes.is_none());
            Ok(())
        })).unwrap();
    }

    #[test]
    fn native_capture_keeps_distinct_styles_and_deduplicates_normalized_html() {
        let _guard = db_test_guard();
        crate::db_runtime::with_test_protected_texts(&[], || crate::db_runtime::with_test_db(|| {
            TestClipboardHost::set_text("same body");
            let bold = "<b>same body</b>";
            let first = NativeClipboardCaptureService::capture_current_with_html::<TestClipboardHost>(0, "browser", || Some(bold.into()));
            TestClipboardHost::set_text("same body");
            let second = NativeClipboardCaptureService::capture_current_with_html::<TestClipboardHost>(0, "browser", || Some("<i>same body</i>".into()));
            assert!(first.inserted && second.inserted);
            assert_ne!(first.item_id, second.item_id);
            let normalized = crate::app_core::clipboard_html::normalize(bold).unwrap();
            TestClipboardHost::set_text("same body");
            let same = NativeClipboardCaptureService::capture_current_with_html::<TestClipboardHost>(0, "browser", || Some(normalized));
            assert!(!same.inserted);
            assert_eq!(same.reason, "duplicate");
            assert_eq!(same.item_id, first.item_id);
            Ok(())
        })).unwrap();
    }

    #[test]
    fn native_capture_rejects_a_clipboard_change_during_html_read() {
        let _guard = db_test_guard();
        crate::db_runtime::with_test_protected_texts(&[], || crate::db_runtime::with_test_db(|| {
            TestClipboardHost::set_text("original body");
            let result = NativeClipboardCaptureService::capture_current_with_html::<TestClipboardHost>(0, "browser", || {
                TestClipboardHost::set_text("new body");
                Some("<b>original body</b>".into())
            });
            assert!(!result.inserted);
            assert_eq!(result.reason, "clipboard_changed");
            let count = crate::db_runtime::with_db(|conn| conn.query_row("SELECT COUNT(*) FROM items", [], |row| row.get::<_, i64>(0)))?;
            assert_eq!(count, 0);
            Ok(())
        })).unwrap();
    }
}
