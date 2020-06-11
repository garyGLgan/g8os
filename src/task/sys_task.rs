//! To avoid the dead look when allocator memory in heap, 
//! Dont' new any object in interrupt handler.
//! Wrapper all interrupt handler to SysTask
//! and then push it the system task queue, 
//! the corresponding function will be called by executor


use crate::{error, warn, debug, info, println, input, print, };
use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use spin::Mutex;
use futures_util::{stream::Stream, stream::StreamExt, task::AtomicWaker};
use core::{
    fmt::{Arguments, Error, Write},
    pin::Pin,
    task::{Context, Poll},
};
use crate::util::{Locked, Flag};
use lazy_static::lazy_static;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1, KeyCode};
use alloc::string::String;
use crate::console::sys_log;
use crate::drivers::pci;

static SYS_TASK_QUEUE: OnceCell<ArrayQueue<SysTask>> = OnceCell::uninit();
static SYS_TASK_WAKER: AtomicWaker = AtomicWaker::new();
static SYS_TASK_STARTED: Locked<Flag> = Locked::new(Flag::new());

lazy_static! {
    static ref KEYBOARD: Locked<Keyboard<layouts::Us104Key, ScancodeSet1>> = Locked::new(
        Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore)
    );

    static ref SYS_CMD: Locked<SysCmd> = Locked::new(SysCmd::new());
}

#[derive(Debug, Copy, Clone, Eq, PartialEq )]
#[repr(u64)]
pub enum SysTask {
    TIMMER(u64),
    KEY(u8),
}

impl SysTask{
    pub fn run(&self) {
        match self {
            Self::TIMMER(i) =>{
                if i == &6 {
                    info!("scan pci devices");
                    pci::scan_devices();
                }
            },
            Self::KEY(c) => {
                let mut keyboard = KEYBOARD.lock();
                if let Ok(Some(key_event)) = keyboard.add_byte(*c) {
                    if let Some(key) = keyboard.process_keyevent(key_event) {
                        match key {
                            DecodedKey::Unicode(character) => {
                                SYS_CMD.lock().input_char(character);
                                input!("{}", character);
                            },
                            DecodedKey::RawKey(key) => {
                                SYS_CMD.lock().input_byte(key);
                                input!("{:?}", key);
                            },
                        }
                    }
                }
            }
        }
    }
}

pub struct SysTaskStream {
    _private: (),
}

impl SysTaskStream {
    pub fn new() -> Self {
        SysTaskStream { _private: () }
    }
}

impl Stream for SysTaskStream {
    type Item =  SysTask;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<SysTask>> {
        let queue = SYS_TASK_QUEUE
                    .try_get()
                    .expect("The SYS_TASK_QUEUE not initialized");
        if let Ok(task) = queue.pop() {
            return Poll::Ready(Some(task));
        }

        SYS_TASK_WAKER.register(&cx.waker());
        match queue.pop() {
            Ok(task) => {
                Poll::Ready(Some(task))
            },
            Err(crossbeam_queue::PopError) => Poll::Pending,
        }
    }
}

pub fn init() {
    SYS_TASK_QUEUE
        .try_init_once(|| ArrayQueue::new(4096))
        .expect("SYS_TASK_QUEUE::new should only the called once");
    SYS_TASK_STARTED.lock().on();
}

pub async fn run_sys_task() {
    let mut sys_task_stream = SysTaskStream::new();
    
    while let Some(task) = sys_task_stream.next().await {
        task.run();
    }
}

pub fn add_sys_task(task: SysTask) {
    if SYS_TASK_STARTED.lock().get(){
        if let Ok(queue) = SYS_TASK_QUEUE.try_get() {
            if let Err(_) = queue.push(task) {
                println!("WRNING: SYS_TASK_QUEUE full; drop log message");
            } else {
                SYS_TASK_WAKER.wake();
            }
        } else {
            println!("WRING: SYS_TASK_QUEUE uninitialized");
        }
    }
}

#[derive(Debug)]
pub struct SysCmd {
    buf: String,
}

impl SysCmd {
    fn new() -> Self {
        SysCmd{
            buf: String::new(),
        }
    }

    fn write(&mut self, args: Arguments) -> Result<(), Error> {
        self.buf.write_fmt(args)
    }

    pub fn input_char(&mut self, c: char){
        match c {
            '\n' => self.run(),
            _ => self.write(format_args!("{}", c)).unwrap(),
        }
    }

    pub fn input_byte(&mut self, k: KeyCode){
        
        match k {
            KeyCode::Enter => self.run(),
            _ => self.write(format_args!("{:?}", k)).unwrap(),
        }
    }

    fn run(&mut self ) {
        let cmd = self.buf.as_ref();
        sys_log::SYS_LOG_LEVEL.lock().conf(cmd);
        self.buf.clear();
    }
}

