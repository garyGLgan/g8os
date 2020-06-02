use super::vga_buffer;
use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::{task::AtomicWaker, stream::Stream, stream::StreamExt};
use core::{pin::Pin, task::{Poll, Context}};
use crate::println;
use core::fmt::{Write, Arguments, Error};
use alloc::{boxed::Box, string::String};


static LOG_MSG_QUEUE: OnceCell<ArrayQueue<LogMsg>> = OnceCell::uninit();
static LOG_WAKER: AtomicWaker = AtomicWaker::new();

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum LogLevel{
    ERROR = 0,
    WARN,
    INFO,
    DEBUG,
}

pub struct LogMsg<'a> {
    level: LogLevel,
    msg: Box<&'a str>,
}

pub struct LogMsgStream{
    _private: (),
}

impl LogMsgStream {
    pub fn new() -> Self {
        LOG_MSG_QUEUE.try_init_once(|| ArrayQueue::new(1000))
            .expect("LogMsgStream::new should only the called once");
        LogMsgStream { _private: () }
    }
}

impl<'a>  Stream for LogMsgStream {
    type Item<'a> = LogMsg<'a>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<LogMsg>> {
        let queue = LOG_MSG_QUEUE.try_get().expect("Log message queue no initialized");
        if let Ok(log_msg) = queue.pop() {
            return Poll::Ready(Some(log_msg));
        }

        LOG_WAKER.register(&cx.waker());
        match queue.pop() {
            Ok(log_msg) => {
                LOG_WAKER.take();
                Poll::Ready(Some(log_msg))
            },
            Err(crossbeam_queue::PopError) => Poll::Pending,
        }
    }
}

impl<'a> LogMsg<'a> {
    fn print(&self) {
        use x86_64::instructions::interrupts;
        
        interrupts::without_interrupts(|| {
            match self.level {
                LogLevel::ERROR => vga_buffer::WRITER.lock().error(self.msg.as_ref()),
                LogLevel::WARN => vga_buffer::WRITER.lock().warn(self.msg.as_ref()),
                LogLevel::DEBUG => vga_buffer::WRITER.lock().debug(self.msg.as_ref()),
                LogLevel::INFO => vga_buffer::WRITER.lock().info(self.msg.as_ref()),
            }
        });
    }
}

fn log(level: LogLevel, args: Arguments) {
    fn write<W: Write>( f: &mut W, args: Arguments) -> Result<(), Error> {
        f.write_fmt(args)
    };

    let mut buf = String::new();
    write(&mut buf, args).unwrap();

    if let Ok(queue) = LOG_MSG_QUEUE.try_get() {
        let log_msg = LogMsg{
            level: level,
            msg: Box::new(&buf[..]),
        };
        if let Err(_) = LOG_MSG_QUEUE.push(log_msg) {
            println!("WRNING: log msg queue full; drop log message");
        }
    }else {
        println!("WRING: log msg queue uninitialized");
    }

}

pub async fn print_log() {
    let mut log_msgs = LogMsgStream::new();

    while let Some(log_msg) = log_msgs.next().await {
        log_msg.print();
    }
}
