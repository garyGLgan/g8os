use super::vga_buffer;
use crate::println;
use alloc::{boxed::Box, string::String};
use conquer_once::spin::OnceCell;
use core::fmt::{Arguments, Error, Write};
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{stream::Stream, stream::StreamExt, task::AtomicWaker};

static LOG_MSG_QUEUE: OnceCell<ArrayQueue<LogMsg>> = OnceCell::uninit();
static LOG_WAKER: AtomicWaker = AtomicWaker::new();
static mut IS_STARTED: bool = false;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum LogLevel {
    ERROR = 0,
    WARN,
    INFO,
    DEBUG,
}

pub struct LogMsg {
    level: LogLevel,
    msg: String,
}

impl LogMsg {
    fn new(log_level: LogLevel, msg: String) -> Self {
        LogMsg {
            level: log_level,
            msg,
        }
    }

    fn print(&self) {
        use x86_64::instructions::interrupts;

        interrupts::without_interrupts(|| match self.level {
            LogLevel::ERROR => vga_buffer::WRITER.lock().error(self.msg.as_ref()),
            LogLevel::WARN => vga_buffer::WRITER.lock().warn(self.msg.as_ref()),
            LogLevel::DEBUG => vga_buffer::WRITER.lock().debug(self.msg.as_ref()),
            LogLevel::INFO => vga_buffer::WRITER.lock().info(self.msg.as_ref()),
        });
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
    type Item = LogMsg;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<LogMsg>> {
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

pub fn _log(level: LogLevel, args: Arguments) {
    if !is_started() {
        return;
    }
    fn write<W: Write>(f: &mut W, args: Arguments) -> Result<(), Error> {
        f.write_fmt(args)
    };

    let mut buf = String::new();
    write(&mut buf, args).unwrap();
    let log_msg = LogMsg::new(level, buf);
    

    if let Ok(queue) = LOG_MSG_QUEUE.try_get() {
        if let Err(_) = queue.push(log_msg) {
            println!("WRNING: log msg queue full; drop log message");
        }else {
            LOG_WAKER.wake();
        }
    } else {
        println!("WRING: log msg queue uninitialized");
    }
}

pub fn is_started() -> bool {
    unsafe{
        IS_STARTED
    }
}

pub fn init() {
    LOG_MSG_QUEUE
        .try_init_once(|| ArrayQueue::new(1000))
        .expect("LogMsgStream::new should only the called once");
    unsafe{
        IS_STARTED = true;
    }
}

pub async fn print_log() {
    println!("print_log");
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
