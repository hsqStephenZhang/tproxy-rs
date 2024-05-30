use std::sync::atomic::AtomicI32;

static MARK: AtomicI32 = AtomicI32::new(0);

pub fn set_mark(val: i32) {
    MARK.store(val, std::sync::atomic::Ordering::SeqCst)
}

pub fn get_mark() -> i32 {
    MARK.load(std::sync::atomic::Ordering::SeqCst)
}
