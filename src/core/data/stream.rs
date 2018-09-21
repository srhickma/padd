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
                None => return None,
                Some(val) => val
            },
            Some(val) => val
        };

        self.outgoing_buffer.push_back(val.clone());

        Some(val)
    }

    pub fn advance(&mut self) -> &mut Self {
        match self.incoming_buffer.pop_back() {
            None => self,
            Some(val) => {
                self.outgoing_buffer.push_back(val.clone());
                self
            }
        }
    }

    pub fn replay(&mut self) -> &mut Self {
        let mut val: Option<T> = self.outgoing_buffer.pop_front();
        while val.is_some() {
            self.incoming_buffer.push_back(val.unwrap());
            val = self.outgoing_buffer.pop_front();
        }
        self
    }

    pub fn consume(&mut self) -> &mut Self {
        if self.block {
            self.replay();
            self.block = false;
        } else {
            self.outgoing_buffer.clear();
        }
        self
    }

    pub fn block(&mut self) -> &mut Self {
        self.block = true;
        self
    }

    pub fn unblock(&mut self) -> &mut Self {
        self.block = false;
        self
    }
}