use std::collections::LinkedList;
use std::rc::Rc;
use std::mem;

pub struct StreamSource<'g, T: 'g + Clone> {
    consumers: LinkedList<StreamBuffer<T>>,
    getter: &'g Fn() -> Option<T>,
}

impl<'g, T: 'g + Clone> StreamSource<'g, T> {
    //TODO reduce visibility

    pub fn observe(getter: &'g Fn() -> Option<T>) -> StreamSource<'g, T> {
        StreamSource {
            consumers: LinkedList::new(),
            getter,
        }
    }

    pub fn split<'a>(&'a mut self) -> Stream<'a, 'g, T> {
        self.consumers.push_front(StreamBuffer {
            incoming_buffer: LinkedList::new(),
            outgoing_buffer: LinkedList::new()
        });

        self.back().unwrap()
    }

    pub fn detach_front<'a>(&'a mut self) {
        self.consumers.pop_front();
    }

    pub fn detach_back<'a>(&'a mut self) -> Option<Stream<'a, 'g, T>> {
        self.consumers.pop_front();
        self.back()
    }

    pub fn back<'a>(&'a mut self) -> Option<Stream<'a, 'g, T>> {
        Some(Stream {
            source: self,
            buffer: match self.consumers.back_mut() {
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

pub struct StreamBuffer<T: Clone> {
    incoming_buffer: LinkedList<T>,
    outgoing_buffer: LinkedList<T>,
}

impl<T: Clone> StreamBuffer<T> {
    fn push(&mut self, val: &T) {
        self.incoming_buffer.push_front(val.clone())
    }
}

pub struct Stream<'a, 'g: 'a, T: 'g + 'a + Clone> {
    buffer: &'a mut StreamBuffer<T>,
    source: *mut StreamSource<'g, T>,
}

impl<'a, 'g: 'a, T: 'g + 'a + Clone> Stream<'a, 'g, T> {
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

    pub fn consumer<'s>(&'s mut self, on_consume: Box<FnMut(&LinkedList<T>) + 's>) -> StreamConsumer<'s, 'a, 'g, T> {
        StreamConsumer {
            stream: self,
            on_consume,
            block: false
        }
    }

    pub fn has_next(&mut self) -> bool {
        if self.buffer.incoming_buffer.is_empty() {
            self.source_pull();
            !self.buffer.incoming_buffer.is_empty()
        } else {
            true
        }
    }

    pub fn pull(&mut self) -> Option<T> {
        let val: T = match self.buffer.incoming_buffer.pop_back() {
            None => {
                self.source_pull();
                match self.buffer.incoming_buffer.pop_back() {
                    None => return None,
                    Some(val) => val
                }
            },
            Some(val) => val
        };

        self.buffer.outgoing_buffer.push_back(val.clone());

        Some(val)
    }

    fn source_pull(&mut self) {
        unsafe {
            (&mut *self.source).pull()
        }
    }

    pub fn advance(&mut self) -> &mut Self {
        match self.buffer.incoming_buffer.pop_back() {
            None => self,
            Some(val) => {
                self.buffer.outgoing_buffer.push_back(val.clone());
                self
            }
        }
    }
}

pub struct StreamConsumer<'s, 'a: 's, 'g: 'a + 's, T: 'g + 'a + 's + Clone> {
    stream: &'s mut Stream<'a, 'g, T>,
    on_consume: Box<FnMut(&LinkedList<T>) + 's>,
    block: bool,
}

impl<'s, 'a: 's, 'g: 'a + 's, T: 'g + 'a + 's + Clone> StreamConsumer<'s, 'a, 'g, T> {
    pub fn has_next(&mut self) -> bool {
        self.stream.has_next()
    }

    pub fn pull(&mut self) -> Option<T> {
        self.stream.pull()
    }

    pub fn advance(&mut self) -> &mut Self {
        self.stream.advance();
        self
    }

    pub fn replay(&mut self) -> &mut Self {
        let mut val: Option<T> = self.stream.buffer.outgoing_buffer.pop_front();
        while val.is_some() {
            self.stream.buffer.incoming_buffer.push_back(val.unwrap());
            val = self.stream.buffer.outgoing_buffer.pop_front();
        }
        self
    }

    pub fn consume(&mut self) -> &mut Self {
        if self.block {
            self.replay();
            self.block = false;
        } else {
            (self.on_consume)(&self.stream.buffer.outgoing_buffer);
            self.stream.buffer.outgoing_buffer.clear();
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

//pub struct ReadDrivenStream<'a, 'g: 'a, T: 'g + 'a + Clone> {
//    data: &'a mut StreamBuffer<T>,
//    source: *mut StreamSource<'g, T>,
//}
//
//impl<'a, 'g: 'a, T: 'g + 'a + Clone> ReadDrivenStream<'a, 'g, T> {
//    //TODO write builder function
//    //TODO swap front/back for top/bottom?? tail/head??
//
//    pub fn split(&mut self) -> Self {
//        unsafe {
//            (&mut *self.source).split()
//        }
//    }
//
//    pub fn detach_front(&mut self) {
//        unsafe {
//            (&mut *self.source).detach_front()
//        }
//    }
//
//    pub fn detach_back(&mut self) -> Option<Self> {
//        unsafe {
//            (&mut *self.source).detach_back()
//        }
//    }
//
//    pub fn on_consume(&mut self, on_consume: Option<&'c mut FnMut(&LinkedList<T>)>) -> &mut Self {
//        self.data.on_consume = on_consume;
//        self
//    }
//
//    pub fn has_next(&mut self) -> bool {
//        if self.data.incoming_buffer.is_empty() {
//            self.source_pull();
//            !self.data.incoming_buffer.is_empty()
//        } else {
//            true
//        }
//    }
//
//    pub fn pull(&mut self) -> Option<T> {
//        let val: T = match self.data.incoming_buffer.pop_back() {
//            None => {
//                self.source_pull();
//                match self.data.incoming_buffer.pop_back() {
//                    None => return None,
//                    Some(val) => val
//                }
//            },
//            Some(val) => val
//        };
//
//        self.data.outgoing_buffer.push_back(val.clone());
//
//        Some(val)
//    }
//
//    fn source_pull(&mut self) {
//        unsafe {
//            (&mut *self.source).pull()
//        }
//    }
//
//    pub fn advance(&mut self) -> &mut Self {
//        match self.data.incoming_buffer.pop_back() {
//            None => self,
//            Some(val) => {
//                self.data.outgoing_buffer.push_back(val.clone());
//                self
//            }
//        }
//    }
//
//    pub fn replay(&mut self) -> &mut Self {
//        let mut val: Option<T> = self.data.outgoing_buffer.pop_front();
//        while val.is_some() {
//            self.data.incoming_buffer.push_back(val.unwrap());
//            val = self.data.outgoing_buffer.pop_front();
//        }
//        self
//    }
//
//    pub fn consume(&mut self) -> &mut Self {
//        if self.data.block {
//            self.replay();
//            self.data.block = false;
//        } else {
//            (self.data.on_consume.unwrap())(&self.data.outgoing_buffer);
//            self.data.outgoing_buffer.clear();
//        }
//        self
//    }
//
//    pub fn block(&mut self) -> &mut Self {
//        self.data.block = true;
//        self
//    }
//
//    pub fn unblock(&mut self) -> &mut Self {
//        self.data.block = false;
//        self
//    }
//}
