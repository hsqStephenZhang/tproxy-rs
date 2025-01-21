use std::sync::atomic::AtomicU32;

static MARK: AtomicU32 = AtomicU32::new(0);

pub fn set_mark(val: u32) {
    MARK.store(val, std::sync::atomic::Ordering::SeqCst)
}

pub fn get_mark() -> u32 {
    MARK.load(std::sync::atomic::Ordering::SeqCst)
}
