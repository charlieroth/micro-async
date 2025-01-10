use core::cell::Cell;

pub struct Sender<'a, T> {
    channel: &'a Channel<T>,
}

impl<T> Sender<'_, T> {
    pub fn send(&self, item: T) {
        self.channel.send(item);
    }
}

pub struct Receiver<'a, T> {
    channel: &'a Channel<T>,
}

impl<T> Receiver<'_, T> {
    pub fn receive(&self) -> Option<T> {
        self.channel.receive()
    }
}

pub struct Channel<T> {
    item: Cell<Option<T>>,
}

impl<T> Channel<T> {
    /// Creates a new `Channel` instance. This is a simple
    /// wrapper around a `cell::Cell` for a minimal API similar
    /// to a regular channel implementation
    pub fn new() -> Self {
        Self {
            item: Cell::new(None),
        }
    }

    pub fn get_sender(&self) -> Sender<T> {
        Sender { channel: &self }
    }

    pub fn get_receiver(&self) -> Receiver<T> {
        Receiver { channel: &self }
    }

    pub fn send(&self, item: T) {
        self.item.replace(Some(item));
    }

    pub fn receive(&self) -> Option<T> {
        self.item.take()
    }
}
