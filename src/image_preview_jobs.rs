use std::sync::{mpsc, Arc, Mutex, OnceLock};

type Job = Box<dyn FnOnce() + Send + 'static>;

struct PreviewExecutor {
    sender: mpsc::SyncSender<Job>,
}

impl PreviewExecutor {
    fn new(workers: usize, capacity: usize) -> Option<Self> {
        let (sender, receiver) = mpsc::sync_channel::<Job>(capacity);
        let receiver = Arc::new(Mutex::new(receiver));
        let mut started = 0;
        for index in 0..workers {
            let receiver = receiver.clone();
            if std::thread::Builder::new()
                .name(format!("image-preview-{index}"))
                .spawn(move || loop {
                    let next = receiver.lock().unwrap_or_else(|p| p.into_inner()).recv();
                    let Ok(job) = next else {
                        break;
                    };
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
                })
                .is_ok()
            {
                started += 1;
            }
        }
        (started > 0).then_some(Self { sender })
    }

    fn submit(&self, job: Job) -> bool {
        self.sender.try_send(job).is_ok()
    }
}

pub(crate) fn submit(job: impl FnOnce() + Send + 'static) -> bool {
    static EXECUTOR: OnceLock<Option<PreviewExecutor>> = OnceLock::new();
    EXECUTOR
        .get_or_init(|| PreviewExecutor::new(2, 8))
        .as_ref()
        .map(|executor| executor.submit(Box::new(job)))
        .unwrap_or(false)
}

pub(crate) fn submit_paste(job: impl FnOnce() + Send + 'static) -> bool {
    static EXECUTOR: OnceLock<Option<PreviewExecutor>> = OnceLock::new();
    EXECUTOR
        .get_or_init(|| PreviewExecutor::new(1, 4))
        .as_ref()
        .map(|executor| executor.submit(Box::new(job)))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn preview_jobs_have_bounded_workers_and_pending_capacity() {
        let executor = PreviewExecutor::new(2, 3).unwrap();
        let (started, observed) = mpsc::channel();
        let barrier = Arc::new(std::sync::Barrier::new(3));
        for _ in 0..2 {
            let (started, barrier) = (started.clone(), barrier.clone());
            assert!(executor.submit(Box::new(move || {
                started.send(()).unwrap();
                barrier.wait();
            })));
        }
        for _ in 0..2 {
            observed.recv_timeout(Duration::from_secs(2)).unwrap();
        }
        for _ in 0..3 {
            assert!(executor.submit(Box::new(|| {})));
        }
        assert!(!executor.submit(Box::new(|| {})));
        barrier.wait();
    }

    #[test]
    fn a_failed_preview_does_not_stop_later_jobs() {
        let executor = PreviewExecutor::new(1, 3).unwrap();
        let (sent, received) = mpsc::channel();
        assert!(executor.submit(Box::new(|| panic!("preview fixture"))));
        assert!(executor.submit(Box::new(move || {
            let _ = sent.send(42);
        })));
        assert_eq!(received.recv_timeout(Duration::from_secs(2)).unwrap(), 42);
    }
}
