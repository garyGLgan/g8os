use crate::input;
use crate::util::Flag;
use conquer_once::spin::OnceCell;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{
    stream::{Stream, StreamExt},
    task::AtomicWaker,
};
use lazy_static::lazy_static;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static SCANCODE_WAKER: AtomicWaker = AtomicWaker::new();
static IS_STARTED: Mutex<Flag> = Mutex::new(Flag::new());

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        ScancodeStream { _private: () }
    }
}

lazy_static! {
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = Mutex::new(
        Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore)
    );
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("Scancode queue is not initialized");

        if let Ok(b) = queue.pop() {
            return Poll::Ready(Some(b));
        }

        SCANCODE_WAKER.register(cx.waker());
        match queue.pop() {
            Ok(b) => {
                SCANCODE_WAKER.take();
                Poll::Ready(Some(b))
            }
            Err(crossbeam_queue::PopError) => Poll::Pending,
        }
    }
}

pub fn add_scancode(scancode: u8) {
    if IS_STARTED.lock().get() {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue is not initialized");
        if let Err(_) = queue.push(scancode) {
            panic!("scancode queue is full, drop key");
        } else {
            SCANCODE_WAKER.wake();
        }
    }
}

pub async fn handle_input() {
    let mut scancode_steam = ScancodeStream::new();

    while let (Some(scancode)) = scancode_steam.next().await {
        let mut keyboard = KEYBOARD.lock();
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => input!("{}", character),
                    DecodedKey::RawKey(key) => input!("{:?}", key),
                }
            }
        }
    }
}

pub fn init() {
    SCANCODE_QUEUE
        .try_init_once(|| ArrayQueue::new(100))
        .expect("ScancodeStream cannot only be initlized once");
    IS_STARTED.lock().on();
}
