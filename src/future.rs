/// Simplified version of Rust's `core::future::Future` trait, used to
/// get a feel for the architecture of an async runtime.
pub trait MicroFuture {
    type Output;
    fn poll(&mut self, task_id: usize) -> MicroPoll<Self::Output>;
}

/// Same as `core::task::Poll`
/// Redefined without all of the attribute clutter.
pub enum MicroPoll<T> {
    Pending,
    Ready(T),
}
