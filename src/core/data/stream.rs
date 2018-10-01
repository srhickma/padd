use std::collections::LinkedList;

pub struct StreamSource<'g, T: 'g + Clone> {
    consumers: LinkedList<StreamBuffer<T>>,
    getter: &'g mut FnMut() -> Option<T>,
}

impl<'g, T: 'g + Clone> StreamSource<'g, T> {
    //TODO reduce visibility

    pub fn observe(getter: &'g mut FnMut() -> Option<T>) -> StreamSource<'g, T> {
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

    pub fn replay(&mut self) -> &mut Self {
        let mut val: Option<T> = self.buffer.outgoing_buffer.pop_back();
        while val.is_some() {
            self.buffer.incoming_buffer.push_back(val.unwrap());
            val = self.buffer.outgoing_buffer.pop_back();
        }
        self
    }
}

pub struct StreamConsumer<'s, 'a: 's, 'g: 'a + 's, T: 'g + 'a + 's + Clone> {
    stream: &'s mut Stream<'a, 'g, T>,
    on_consume: Box<FnMut(&LinkedList<T>) + 's>,
    block: bool, //TODO see if we can remove this via splitting
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
        self.stream.replay();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_iterator() {
        //setup
        let input = "erlerlkjrf4093452ri2309u0ur045thejhefkj".to_string();
        let mut iter = input.chars();

        let mut getter = || {
            iter.next()
        };

        let mut source = StreamSource::observe(&mut getter);
        let mut stream = source.split();

        let mut res = String::new();

        //exercise
        loop {
            match stream.pull() {
                None => break,
                Some(c) => res.push(c)
            }
        }

        //verify
        assert_eq!(res, "erlerlkjrf4093452ri2309u0ur045thejhefkj");
    }

    #[test]
    fn as_iterator_consumed() {
        //setup
        let input = "erlerlkjrf4093452ri2309u0ur045thejhefkj".to_string();
        let mut iter = input.chars();

        let mut getter = || {
            iter.next()
        };

        let mut source = StreamSource::observe(&mut getter);
        let mut base = source.split();

        let mut res_pulled = String::new();
        let mut res_consumed = String::new();

        //exercise
        {
            let mut stream = base.consumer(Box::new(|list: &LinkedList<char>| {
                assert_eq!(list.len(), 1);
                res_consumed.push(*list.front().unwrap());
            }));

            loop {
                match stream.pull() {
                    None => break,
                    Some(c) => res_pulled.push(c)
                }
                stream.consume();
            }
        }

        //verify
        assert_eq!(res_pulled, "erlerlkjrf4093452ri2309u0ur045thejhefkj");
        assert_eq!(res_consumed, "erlerlkjrf4093452ri2309u0ur045thejhefkj");
    }

    #[test]
    fn unconsumed_replay() {
        //setup
        let input = "abcdef".to_string();
        let mut iter = input.chars();

        let mut getter = || {
            iter.next()
        };

        let mut source = StreamSource::observe(&mut getter);
        let mut stream = source.split();

        let mut res = String::new();

        //exercise
        for _ in 0..3 {
            match stream.pull() {
                None => break,
                Some(c) => res.push(c)
            }
        }
        stream.replay();
        loop {
            match stream.pull() {
                None => break,
                Some(c) => res.push(c)
            }
        }

        //verify
        assert_eq!(res, "abcabcdef");
    }

    #[test]
    fn consumed_replay() {
        //setup
        let input = "abcdef".to_string();
        let mut iter = input.chars();

        let mut getter = || {
            iter.next()
        };

        let mut source = StreamSource::observe(&mut getter);
        let mut base = source.split();

        let mut res_pulled = String::new();
        let mut res_consumed = String::new();

        //exercise
        {
            let mut stream = base.consumer(Box::new(|list: &LinkedList<char>| {
                for c in list {
                    res_consumed.push(*c);
                }
            }));

            for _ in 0..3 {
                match stream.pull() {
                    None => break,
                    Some(c) => res_pulled.push(c)
                }
            }
            stream.consume();
            loop {
                match stream.pull() {
                    None => break,
                    Some(c) => res_pulled.push(c)
                }
            }
            stream.replay();
            loop {
                match stream.pull() {
                    None => break,
                    Some(c) => res_pulled.push(c)
                }
            }
            stream.consume();
        }

        //verify
        assert_eq!(res_pulled, "abcdefdef");
        assert_eq!(res_consumed, "abcdef");
    }

    #[test]
    fn consumer_block() {
        //setup
        let input = "abcdef".to_string();
        let mut iter = input.chars();

        let mut getter = || {
            iter.next()
        };

        let mut source = StreamSource::observe(&mut getter);
        let mut base = source.split();

        let mut res_pulled = String::new();
        let mut res_consumed = String::new();

        //exercise
        {
            let mut stream = base.consumer(Box::new(|list: &LinkedList<char>| {
                for c in list {
                    res_consumed.push(*c);
                }
            }));

            stream.block();
            loop {
                match stream.pull() {
                    None => break,
                    Some(c) => res_pulled.push(c)
                }
            }
            stream.consume().replay();
            loop {
                match stream.pull() {
                    None => break,
                    Some(c) => res_pulled.push(c)
                }
            }
            stream.unblock().consume().replay();
            loop {
                match stream.pull() {
                    None => break,
                    Some(c) => res_pulled.push(c)
                }
            }
        }

        //verify
        assert_eq!(res_pulled, "abcdefabcdef");
        assert_eq!(res_consumed, "abcdef");
    }

    #[test]
    fn advance() {
        //setup
        let input = "abcdef".to_string();
        let mut iter = input.chars();

        let mut getter = || {
            iter.next()
        };

        let mut source = StreamSource::observe(&mut getter);
        let mut stream = source.split();

        let mut res = String::new();

        //exercise
        loop {
            match stream.pull() {
                None => break,
                Some(c) => res.push(c)
            }
        }
        stream.replay().advance().advance().advance();
        loop {
            match stream.pull() {
                None => break,
                Some(c) => res.push(c)
            }
        }

        //verify
        assert_eq!(res, "abcdefdef");
    }

    #[test]
    fn has_next() {
        //setup
        let input = "abcdef".to_string();
        let mut iter = input.chars();

        let mut getter = || {
            iter.next()
        };

        let mut source = StreamSource::observe(&mut getter);
        let mut base = source.split();
        let mut stream = base.consumer(Box::new(|_| {}));

        let mut res = String::new();

        //exercise
        for _ in 0..3 {
            assert!(stream.has_next());
            res.push(stream.pull().unwrap())
        }
        stream.consume();
        while stream.has_next() {
            res.push(stream.pull().unwrap())
        }
        stream.replay();
        while stream.has_next() {
            res.push(stream.pull().unwrap())
        }
        stream.consume();

        //verify
        assert_eq!(res, "abcdefdef");
    }
}
