use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use crate::app_core::{
    ClipKind, ClipKindFilter, NativeHostClipListItemProjection, NATIVE_HOST_CLIP_ROW_CAPACITY,
};

struct NativeSearchRequest {
    generation: u64,
    category: i64,
    group_id: i64,
    kind_filter: ClipKindFilter,
    query: String,
    phrase_titles_enabled: bool,
    page_index: usize,
}

pub(crate) struct NativeSearchResult {
    pub generation: u64,
    pub items: Result<Vec<NativeHostClipListItemProjection>, String>,
    pub protection_revision: Option<String>,
    pub data_generation: u64,
    pub page_index: usize,
    pub has_more: bool,
}

#[derive(Default)]
struct NativeSearchQueue {
    pending: Option<NativeSearchRequest>,
    result: Option<NativeSearchResult>,
    stopped: bool,
}

struct NativeSearchShared {
    generation: Arc<AtomicU64>,
    queue: Mutex<NativeSearchQueue>,
    ready: Condvar,
}

/// Each native list owns one worker and at most one pending query/result.
/// Replacing a request interrupts SQLite and prevents an old result from publishing.
pub(crate) struct NativeSearchService {
    shared: Arc<NativeSearchShared>,
}

impl NativeSearchService {
    pub(crate) fn new() -> Self {
        Self::with_executor(execute_native_search)
    }

    fn with_executor(
        execute: impl Fn(&NativeSearchRequest, Arc<AtomicU64>) -> NativeSearchResult + Send + 'static,
    ) -> Self {
        let shared = Arc::new(NativeSearchShared {
            generation: Arc::new(AtomicU64::new(0)),
            queue: Mutex::new(NativeSearchQueue::default()),
            ready: Condvar::new(),
        });
        let worker = shared.clone();
        std::thread::spawn(move || loop {
            let request = {
                let Ok(mut queue) = worker.queue.lock() else {
                    return;
                };
                while queue.pending.is_none() && !queue.stopped {
                    queue = match worker.ready.wait(queue) {
                        Ok(queue) => queue,
                        Err(_) => return,
                    };
                }
                if queue.stopped {
                    return;
                }
                queue.pending.take().expect("pending search request")
            };
            let result = execute(&request, worker.generation.clone());
            let Ok(mut queue) = worker.queue.lock() else {
                return;
            };
            if !queue.stopped && worker.generation.load(Ordering::Acquire) == request.generation {
                queue.result = Some(result);
            }
        });
        Self { shared }
    }

    pub(crate) fn submit(
        &self,
        category: i64,
        group_id: i64,
        kind_filter: ClipKindFilter,
        query: String,
        phrase_titles_enabled: bool,
    ) -> u64 {
        self.submit_page(
            category,
            group_id,
            kind_filter,
            query,
            phrase_titles_enabled,
            0,
        )
    }

    pub(crate) fn submit_page(
        &self,
        category: i64,
        group_id: i64,
        kind_filter: ClipKindFilter,
        query: String,
        phrase_titles_enabled: bool,
        page_index: usize,
    ) -> u64 {
        let mut queue = self
            .shared
            .queue
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let generation = self
            .shared
            .generation
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1);
        queue.result = None;
        queue.pending = Some(NativeSearchRequest {
            generation,
            category,
            group_id,
            kind_filter,
            query,
            phrase_titles_enabled,
            page_index,
        });
        self.shared.ready.notify_one();
        generation
    }

    pub(crate) fn cancel(&self) {
        let mut queue = self
            .shared
            .queue
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.shared.generation.fetch_add(1, Ordering::AcqRel);
        queue.pending = None;
        queue.result = None;
    }

    pub(crate) fn try_latest_result(&self) -> Option<NativeSearchResult> {
        let mut result = self.shared.queue.lock().ok()?.result.take()?;
        if result.generation != self.shared.generation.load(Ordering::Acquire) {
            return None;
        }
        if result.data_generation != crate::db_runtime::current_app_data_generation()
            || result.protection_revision != crate::db_runtime::search_protection_revision().ok()
        {
            result.items = Err("Search data changed; retry the query".to_string());
        }
        Some(result)
    }
}

fn execute_native_search(
    request: &NativeSearchRequest,
    generation: Arc<AtomicU64>,
) -> NativeSearchResult {
    let data_generation = crate::db_runtime::current_app_data_generation();
    let protection_revision = crate::db_runtime::search_protection_revision().ok();
    let mut items = if protection_revision.is_some() && data_generation & 1 == 0 {
        crate::db_runtime::native_clip_list_items_for_query_page_cancellable(
            request.category,
            request.group_id,
            request.kind_filter,
            &request.query,
            NATIVE_HOST_CLIP_ROW_CAPACITY + 1,
            request
                .page_index
                .saturating_mul(NATIVE_HOST_CLIP_ROW_CAPACITY),
            generation,
            request.generation,
        )
        .map_err(|error| error.to_string())
    } else {
        Err("Search is temporarily unavailable".to_string())
    };
    if !request.phrase_titles_enabled {
        if let Ok(items) = items.as_mut() {
            for item in items {
                if item.kind == ClipKind::Phrase {
                    item.title.clear();
                }
            }
        }
    }
    if data_generation != crate::db_runtime::current_app_data_generation()
        || protection_revision != crate::db_runtime::search_protection_revision().ok()
    {
        items = Err("Search data changed; retry the query".to_string());
    }
    let has_more = items
        .as_ref()
        .is_ok_and(|items| items.len() > NATIVE_HOST_CLIP_ROW_CAPACITY);
    if let Ok(items) = items.as_mut() {
        items.truncate(NATIVE_HOST_CLIP_ROW_CAPACITY);
    }
    NativeSearchResult {
        generation: request.generation,
        items,
        protection_revision,
        data_generation,
        page_index: request.page_index,
        has_more,
    }
}

impl Drop for NativeSearchService {
    fn drop(&mut self) {
        self.cancel();
        if let Ok(mut queue) = self.shared.queue.lock() {
            queue.stopped = true;
        }
        self.shared.ready.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    fn test_result(request: &NativeSearchRequest) -> NativeSearchResult {
        NativeSearchResult {
            generation: request.generation,
            items: Ok(Vec::new()),
            protection_revision: None,
            data_generation: 0,
            page_index: request.page_index,
            has_more: false,
        }
    }

    #[test]
    fn native_search_coalesces_pending_queries_and_never_publishes_old_work() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let service = NativeSearchService::with_executor(move |request, _| {
            started_tx.send(request.query.clone()).unwrap();
            if request.query == "first" {
                release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            }
            test_result(request)
        });
        service.submit(0, 0, ClipKindFilter::All, "first".into(), true);
        assert_eq!(
            started_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            "first"
        );
        service.submit(0, 0, ClipKindFilter::All, "middle".into(), true);
        let latest = service.submit(1, 2, ClipKindFilter::All, "latest".into(), false);
        release_tx.send(()).unwrap();
        assert_eq!(
            started_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            "latest"
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(result) = service.try_latest_result() {
                assert_eq!(result.generation, latest);
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(started_rx.try_recv().is_err());
    }

    #[test]
    fn dropping_native_search_service_cancels_in_flight_work_without_waiting() {
        let (started_tx, started_rx) = mpsc::channel();
        let (stopped_tx, stopped_rx) = mpsc::channel();
        let service = NativeSearchService::with_executor(move |request, generation| {
            started_tx.send(()).unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            while generation.load(Ordering::Acquire) == request.generation {
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
            stopped_tx.send(()).unwrap();
            test_result(request)
        });
        service.submit(0, 0, ClipKindFilter::All, "in flight".into(), true);
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        drop(service);
        stopped_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    }

    #[test]
    fn native_search_pages_expose_every_matching_record_without_duplicates() {
        crate::db_runtime::with_test_protected_texts(&[], || crate::db_runtime::with_test_db(|| {
            crate::db_runtime::with_db_mut(|conn| {
                let transaction = conn.transaction()?;
                for index in 0..130 {
                    transaction.execute("INSERT INTO items(category,kind,preview,text_data) VALUES(0,'text',?1,?1)", [format!("page needle {index}")])?;
                }
                transaction.commit()
            })?;
            let generation = Arc::new(AtomicU64::new(1));
            let mut ids = Vec::new();
            for page_index in 0..3 {
                let request = NativeSearchRequest { generation: 1, category: 0, group_id: 0, kind_filter: ClipKindFilter::All,
                    query: "needle".into(), phrase_titles_enabled: true, page_index };
                let result = execute_native_search(&request, generation.clone());
                assert_eq!(result.page_index, page_index);
                assert_eq!(result.has_more, page_index < 2);
                let items = result.items.unwrap();
                assert_eq!(items.len(), if page_index < 2 { 64 } else { 2 });
                ids.extend(items.into_iter().map(|item| item.id));
            }
            assert_eq!(ids.len(), 130);
            assert!(ids.windows(2).all(|pair| pair[0] > pair[1]));
            Ok(())
        })).unwrap();
    }
}
