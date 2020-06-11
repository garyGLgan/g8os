use super::vga_buffer;
use crate::{warn, println};
use crate::util::Flag;
use alloc::{boxed::Box, string::String};
use conquer_once::spin::OnceCell;
use core::{
    fmt::{Arguments, Error, Write},
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{stream::Stream, stream::StreamExt, task::AtomicWaker};
use spin::Mutex;

static LOG_MSG_QUEUE: OnceCell<ArrayQueue<ScrnOut>> = OnceCell::uninit();
static LOG_WAKER: AtomicWaker = AtomicWaker::new();
static IS_STARTED: Mutex<Flag> = Mutex::new(Flag::new());
pub static SYS_LOG_LEVEL: Mutex<SysLogLevel> = Mutex::new(SysLogLevel::new());

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum LogLevel {
    ERROR = 1,
    WARN = 2,
    INFO = 4,
    DEBUG = 8,
}

pub struct SysLogLevel(u8);

impl SysLogLevel {
    pub const fn new() -> Self {
        SysLogLevel(0)
    }

    fn on(&mut self, l: LogLevel) {
        self.0 |= (l as u8);
    }

    fn off(&mut self, l: LogLevel) {
        self.0 &= !(l as u8);
    }

    fn is_on(&mut self, l: LogLevel) -> bool {
        (self.0 & (l as u8)) > 0
    }

    pub fn conf(&mut self, cmd: &str) {
        match cmd {
            "LD1" => self.on(LogLevel::DEBUG),
            "LD0" => self.off(LogLevel::DEBUG),
            "LE1" => self.on(LogLevel::ERROR),
            "LE0" => self.off(LogLevel::ERROR),
            "LI1" => self.on(LogLevel::INFO),
            "LI0" => self.off(LogLevel::INFO),
            "LW1" => self.on(LogLevel::WARN),
            "LW0" => self.off(LogLevel::WARN),
            _ => warn!("unsupported command"),
        }
    }
}

#[derive(Debug)]
#[repr(u64)]
pub enum ScrnOut {
    LOG_MSG(LogLevel, String),
    INPUT_MSG(String),
}

impl ScrnOut {
    fn print(&self) {
        match self {
            Self::LOG_MSG(LogLevel::ERROR, msg) if SYS_LOG_LEVEL.lock().is_on(LogLevel::ERROR) => {
                vga_buffer::WRITER.lock().error(msg.as_ref())
            }
            Self::LOG_MSG(LogLevel::WARN, msg) if SYS_LOG_LEVEL.lock().is_on(LogLevel::WARN) => {
                vga_buffer::WRITER.lock().warn(msg.as_ref())
            }
            Self::LOG_MSG(LogLevel::DEBUG, msg) if SYS_LOG_LEVEL.lock().is_on(LogLevel::DEBUG) => {
                vga_buffer::WRITER.lock().debug(msg.as_ref())
            }
            Self::LOG_MSG(LogLevel::INFO, msg) if SYS_LOG_LEVEL.lock().is_on(LogLevel::INFO) => {
                vga_buffer::WRITER.lock().info(msg.as_ref())
            }
            Self::INPUT_MSG(msg) => vga_buffer::WRITER.lock().input(msg.as_ref()),
            Self::LOG_MSG(_, _) => (),
        }
    }
}

pub struct LogMsgStream {
    _private: (),
}

impl LogMsgStream {
    pub fn new() -> Self {
        LogMsgStream { _private: () }
    }
}

impl Stream for LogMsgStream {
    type Item = ScrnOut;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<ScrnOut>> {
        let queue = LOG_MSG_QUEUE
            .try_get()
            .expect("Log message queue no initialized");
        if let Ok(log_msg) = queue.pop() {
            return Poll::Ready(Some(log_msg));
        }

        LOG_WAKER.register(&cx.waker());
        match queue.pop() {
            Ok(log_msg) => {
                LOG_WAKER.take();
                Poll::Ready(Some(log_msg))
            }
            Err(crossbeam_queue::PopError) => Poll::Pending,
        }
    }
}

fn _fmt(args: Arguments) -> String {
    let mut buf = String::new();
    buf.write_fmt(args).unwrap();
    buf
}

fn push_msg(msg: ScrnOut) {
    if let Ok(queue) = LOG_MSG_QUEUE.try_get() {
        if let Err(_) = queue.push(msg) {
            println!("WRNING: log msg queue full; drop log message");
        } else {
            LOG_WAKER.wake();
        }
    } else {
        println!("WRING: log msg queue uninitialized");
    }
}

pub fn _log(level: LogLevel, args: Arguments) {
    if IS_STARTED.lock().get() {
        push_msg(ScrnOut::LOG_MSG(level, _fmt(args)));
    }
}

pub fn _input(args: Arguments) {
    if IS_STARTED.lock().get() {
        push_msg(ScrnOut::INPUT_MSG(_fmt(args)));
    }
}

pub fn init() {
    LOG_MSG_QUEUE
        .try_init_once(|| ArrayQueue::new(1000))
        .expect("LogMsgStream::new should only the called once");
    IS_STARTED.lock().on();
    SYS_LOG_LEVEL.lock().on(LogLevel::ERROR);
    SYS_LOG_LEVEL.lock().on(LogLevel::INFO);
    SYS_LOG_LEVEL.lock().on(LogLevel::WARN);
}

pub async fn print_log() {
    let mut log_msgs = LogMsgStream::new();

    while let Some(log_msg) = log_msgs.next().await {
        log_msg.print();
    }
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::console::sys_log::_log($crate::console::sys_log::LogLevel::INFO, format_args!("{}\n", format_args!($($arg)*))));
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ($crate::console::sys_log::_log($crate::console::sys_log::LogLevel::DEBUG, format_args!("{}\n", format_args!($($arg)*))));
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ($crate::console::sys_log::_log($crate::console::sys_log::LogLevel::WARN, format_args!("{}\n", format_args!($($arg)*))));
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::console::sys_log::_log($crate::console::sys_log::LogLevel::ERROR, format_args!("{}\n", format_args!($($arg)*))));
}

#[macro_export]
macro_rules! input {
    ($($arg:tt)*) => ($crate::console::sys_log::_input(format_args!($($arg)*)));
}
