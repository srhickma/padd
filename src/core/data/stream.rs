use std::collections::LinkedList;
use std::rc::Rc;
use std::mem;

pub struct StreamSource<'c, 'g: 'c, T: 'g + 'c + Clone> {
    consumers: LinkedList<StreamConsumer<'c, T>>,
    getter: &'g Fn() -> Option<T>,
}

impl<'c, 'g: 'c, T: 'g + 'c + Clone> StreamSource<'c, 'g, T> {
    //TODO reduce visibility

    pub fn observe(getter: &'g Fn() -> Option<T>) -> StreamSource<'c, 'g, T> {
        StreamSource {
            consumers: LinkedList::new(),
            getter,
        }
    }

    pub fn split<'a>(&'a mut self) -> ReadDrivenStream<'a, 'c, 'g, T> {
        self.consumers.push_front(StreamConsumer {
            incoming_buffer: LinkedList::new(),
            outgoing_buffer: LinkedList::new(),
            on_consume: None,
            block: false,
        });

        self.back().unwrap()
    }

    pub fn detach_front<'a>(&'a mut self) {
        self.consumers.pop_front();
    }

    pub fn detach_back<'a>(&'a mut self) -> Option<ReadDrivenStream<'a, 'c, 'g, T>> {
        self.consumers.pop_front();
        self.back()
    }

    pub fn back<'a>(&'a mut self) -> Option<ReadDrivenStream<'a, 'c, 'g, T>> {
        Some(ReadDrivenStream {
            source: self,
            data: match self.consumers.back_mut() {
                None => return None,
                Some(consumer) => consumer
            },
        })
    }

    pub fn pull(&mut self) {
        match (self.getter)() {
            None => {},
            Some(val) => for consumer in &mut self.consumers {
                consumer.push(&val)
            }
        }
    }
}

pub struct StreamConsumer<'c, T: 'c + Clone> {
    incoming_buffer: LinkedList<T>,
    outgoing_buffer: LinkedList<T>,
    on_consume: Option<&'c mut  FnMut(&LinkedList<T>)>,
    block: bool,
}

impl<'c, T: 'c + Clone> StreamConsumer<'c, T> {
    fn push(&mut self, val: &T) {
        self.incoming_buffer.push_front(val.clone())
    }
}

pub struct ReadDrivenStream<'a, 'c: 'a, 'g: 'c + 'a, T: 'g + 'c + 'a + Clone> {
    data: &'a mut StreamConsumer<'c, T>,
    source: *mut StreamSource<'c, 'g, T>,
}

impl<'a, 'c: 'a, 'g: 'c + 'a, T: 'g + 'c + 'a + Clone> ReadDrivenStream<'a, 'c, 'g, T> {
    //TODO write builder function
    //TODO swap front/back for top/bottom?? tail/head??

    pub fn split(&mut self) -> Self {
        unsafe {
            (&mut *self.source).split()
        }
    }

    pub fn detach_front(&mut self) {
        unsafe {
            (&mut *self.source).detach_front()
        }
    }

    pub fn detach_back(&mut self) -> Option<Self> {
        unsafe {
            (&mut *self.source).detach_back()
        }
    }

    pub fn on_consume(&mut self, on_consume: Option<&'c mut FnMut(&LinkedList<T>)>) -> &mut Self {
        self.data.on_consume = on_consume;
        self
    }

    pub fn has_next(&mut self) -> bool {
        if self.data.incoming_buffer.is_empty() {
            self.source_pull();
            !self.data.incoming_buffer.is_empty()
        } else {
            true
        }
    }

    pub fn pull(&mut self) -> Option<T> {
        let val: T = match self.data.incoming_buffer.pop_back() {
            None => {
                self.source_pull();
                match self.data.incoming_buffer.pop_back() {
                    None => return None,
                    Some(val) => val
                }
            },
            Some(val) => val
        };

        self.data.outgoing_buffer.push_back(val.clone());

        Some(val)
    }

    fn source_pull(&mut self) {
        unsafe {
            (&mut *self.source).pull()
        }
    }

    pub fn advance(&mut self) -> &mut Self {
        match self.data.incoming_buffer.pop_back() {
            None => self,
            Some(val) => {
                self.data.outgoing_buffer.push_back(val.clone());
                self
            }
        }
    }

    pub fn replay(&mut self) -> &mut Self {
        let mut val: Option<T> = self.data.outgoing_buffer.pop_front();
        while val.is_some() {
            self.data.incoming_buffer.push_back(val.unwrap());
            val = self.data.outgoing_buffer.pop_front();
        }
        self
    }

    pub fn consume(&mut self) -> &mut Self {
        if self.data.block {
            self.replay();
            self.data.block = false;
        } else {
            (self.data.on_consume.unwrap())(&self.data.outgoing_buffer);
            self.data.outgoing_buffer.clear();
        }
        self
    }

    pub fn block(&mut self) -> &mut Self {
        self.data.block = true;
        self
    }

    pub fn unblock(&mut self) -> &mut Self {
        self.data.block = false;
        self
    }
}
