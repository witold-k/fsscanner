use fsscanner::threadpool::{ThreadPool, ThreadPoolError};
use std::sync::{Arc, Mutex};

#[test]
fn rejects_zero_workers() {
    assert!(matches!(
        ThreadPool::new(0),
        Err(ThreadPoolError::NoWorkers)
    ));
}

#[test]
fn join_waits_for_queued_jobs() {
    let value = Arc::new(Mutex::new(0usize));
    let pool = ThreadPool::new(2).unwrap();

    for _ in 0..8 {
        let value = Arc::clone(&value);
        pool.execute(move || {
            *value.lock().unwrap() += 1;
        })
        .unwrap();
    }

    pool.join().unwrap();
    assert_eq!(*value.lock().unwrap(), 8);
}

#[test]
fn join_reports_worker_panics() {
    let pool = ThreadPool::new(1).unwrap();
    pool.execute(|| panic!("worker panic")).unwrap();

    assert!(matches!(
        pool.join(),
        Err(ThreadPoolError::WorkerPanicked)
    ));
}
