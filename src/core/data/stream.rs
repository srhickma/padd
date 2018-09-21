use std::collections::LinkedList;

pub struct ReadDrivenStream<T: Clone> {
    incoming_buffer: LinkedList<T>,
    outgoing_buffer: LinkedList<T>,
    getter: fn() -> Option<T>,
    block: bool,
}

impl<T: Clone> ReadDrivenStream<T> {
    pub fn observe(getter: fn() -> Option<T>) -> ReadDrivenStream<T> {
        ReadDrivenStream {
            incoming_buffer: LinkedList::new(),
            outgoing_buffer: LinkedList::new(),
            getter,
            block: false,
        }
    }

    pub fn pull(&mut self) -> Option<T> {
        let val: T = match self.incoming_buffer.pop_back() {
            None => match (self.getter)() {
                None => { return None; }
                Some(val) => val
            },
            Some(val) => val
        };

        self.outgoing_buffer.push_back(val.clone());

        Some(val)
    }

    pub fn replay(&mut self) {
        let mut val: Option<T> = self.outgoing_buffer.pop_front();
        while val.is_some() {
            self.incoming_buffer.push_back(val.unwrap());
            val = self.outgoing_buffer.pop_front();
        }
    }

    pub fn consume(&mut self) {
        if self.block {
            self.replay();
            self.block = false;
        } else {
            self.outgoing_buffer.clear();
        }
    }

    pub fn block(&mut self) {
        self.block = true;
    }

    pub fn unblock(&mut self) {
        self.block = false;
    }
}