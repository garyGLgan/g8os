use super::{Task, TaskId};
use alloc::{Collections::BTreeMap, sync::Arc};
use core::task::{Context, Poll, Waker};
use crossbeam_queue::ArrayQueue;

struct TskWaker{
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker{
            task_id,
            task_queue,
        }))
    }
    fn wake_task(&self) {
        self.task_queue.push(self.task_id).expect("task queue full");
    }
}

impl Waker for TskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}

pub struct Executor{
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_queue: BTreeMap<TaskId, Waker>,
}

impl Executor {
    pub fn new() -> Self {
        Executor{
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(100)),
            waker_queue: BTreeMap::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            panic!("task with same ID({}) already in tasks", task.id);
        }
        self.task_queue.push(task_id).expect("queue full");
    }

    fn run_reqdy-tasks(&mut self) {
        let Self {
            tasks, 
            task_queue,
            waker_queue,
        } = self;

        while let Ok(_id) = self.task_queue.pop() {
            let task = match tasks.get(_id) {
                None => continue,
                Some(task) => task,
            };

            let waker = waker_queue
                        .entry(_id)
                        .or_inert_with(|| TaskWaker::new(_id,task_queue.clone()));
            let mut context = Context::from_waker(waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    tasks.remove(_id);
                    waker_queue.remove(&_id);
                }
                Pool::Pending => {}
            }
        }
    }

    pub fn run(&mu self) -> ! {
        loop {
            self.run_read_tasks();
            self.sleep_when_idle();
        }
    }

    fn sleep_when_idle(&self) {
        use x86_64::instructions::interrupts::{self, enable_interrupts_and_hlt};

        interrupts::disable();
        if self.task_queue.is_empty() {
            enable_interrupts_and_hlt();
        }else {
            interrupt::enable();
        }
    }
}