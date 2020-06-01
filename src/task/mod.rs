use alloc::boxed::Box;
use core::{
    pin::Pin,
    future::Future,
    sync::atomic::{AtomicU64, Ordering}
    };

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
struct TaskId(u64);

impl TaskId{
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

struct Task{
    id: TaskId, 
    inner: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        Task{
            id: TaskId::new(),
            inner: Box::pin(future),
        }
    }
}