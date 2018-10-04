use std::collections::LinkedList;

pub struct StreamSource<'g, T: 'g + Clone> {
    buffers: LinkedList<StreamBuffer<T>>,
    getter: &'g mut FnMut() -> Option<T>,
}

impl<'g, T: 'g + Clone> StreamSource<'g, T> {
    pub fn observe(getter: &'g mut FnMut() -> Option<T>) -> StreamSource<'g, T> {
        StreamSource {
            buffers: LinkedList::new(),
            getter,
        }
    }

    pub fn split<'a>(&'a mut self) -> Stream<'a, 'g, T> {
        self.buffers.push_back(StreamBuffer::new());

        self.head().unwrap()
    }

    pub fn detach_tail<'a>(&'a mut self) {
        match self.buffers.pop_back() {
            None => {}
            Some(head) => {
                self.buffers = LinkedList::new();
                self.buffers.push_back(head);
            }
        };
    }

    pub fn detach_head<'a>(&'a mut self) -> Option<Stream<'a, 'g, T>> {
        self.buffers.pop_back();
        self.head()
    }

    pub fn head<'a>(&'a mut self) -> Option<Stream<'a, 'g, T>> {
        Some(Stream {
            source: self,
            buffer: match self.buffers.back_mut() {
                None => return None,
                Some(buffer) => buffer
            },
        })
    }

    fn pull(&mut self) {
        match (self.getter)() {
            None => {}
            Some(val) => for buffer in &mut self.buffers {
                buffer.push(&val)
            }
        }
    }
}

pub struct StreamBuffer<T: Clone> {
    incoming_buffer: LinkedList<T>,
    outgoing_buffer: LinkedList<T>,
}

impl<T: Clone> StreamBuffer<T> {
    fn new() -> Self {
        StreamBuffer {
            incoming_buffer: LinkedList::new(),
            outgoing_buffer: LinkedList::new(),
        }
    }

    fn push(&mut self, val: &T) {
        self.incoming_buffer.push_front(val.clone())
    }

    fn dequeue_incoming(&mut self) -> Option<T> {
        self.incoming_buffer.pop_back()
    }

    fn enqueue_outgoing(&mut self, val: &T) {
        self.outgoing_buffer.push_back(val.clone());
    }

    fn input_queued(&self) -> bool {
        self.incoming_buffer.is_empty()
    }

    fn advance(&mut self) {
        match self.incoming_buffer.pop_back() {
            None => {}
            Some(val) => self.outgoing_buffer.push_back(val.clone())
        };
    }

    fn replay(&mut self) {
        let mut val: Option<T> = self.outgoing_buffer.pop_back();
        while val.is_some() {
            self.incoming_buffer.push_back(val.unwrap());
            val = self.outgoing_buffer.pop_back();
        }
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

    pub fn detach_tail(&mut self) -> &mut Self {
        unsafe {
            (&mut *self.source).detach_tail()
        }
        self
    }

    pub fn detach_head(&mut self) -> Option<Self> {
        unsafe {
            (&mut *self.source).detach_head()
        }
    }

    pub fn consumer<'s>(&'s mut self, on_consume: Box<FnMut(&LinkedList<T>) + 's>) -> StreamConsumer<'s, 'a, 'g, T> {
        StreamConsumer::new(self, on_consume)
    }

    pub fn has_next(&mut self) -> bool {
        if self.buffer.input_queued() {
            self.source_pull();
            !self.buffer.input_queued()
        } else {
            true
        }
    }

    pub fn pull(&mut self) -> Option<T> {
        let val: T = match self.buffer.dequeue_incoming() {
            None => {
                self.source_pull();
                match self.buffer.dequeue_incoming() {
                    None => return None,
                    Some(val) => val
                }
            }
            Some(val) => val
        };

        self.buffer.enqueue_outgoing(&val);

        Some(val)
    }

    pub fn advance(&mut self) -> &mut Self {
        self.buffer.advance();
        self
    }

    pub fn replay(&mut self) -> &mut Self {
        self.buffer.replay();
        self
    }

    fn source_pull(&mut self) {
        unsafe {
            (&mut *self.source).pull()
        }
    }
}

pub struct StreamConsumer<'s, 'a: 's, 'g: 'a + 's, T: 'g + 'a + 's + Clone> {
    stream: &'s mut Stream<'a, 'g, T>,
    on_consume: Box<FnMut(&LinkedList<T>) + 's>,
    block: bool, //TODO see if we can remove this via splitting
}

impl<'s, 'a: 's, 'g: 'a + 's, T: 'g + 'a + 's + Clone> StreamConsumer<'s, 'a, 'g, T> {
    fn new(stream: &'s mut Stream<'a, 'g, T>, on_consume: Box<FnMut(&LinkedList<T>) + 's>) -> Self {
        StreamConsumer {
            stream,
            on_consume,
            block: false,
        }
    }

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

    #[test]
    fn split_detach_head() {
        //setup
        let input = "abcdefghijklmnopqrstuvwxyz".to_string();
        let mut iter = input.chars();

        let mut getter = || {
            iter.next()
        };

        let mut source = StreamSource::observe(&mut getter);

        let mut base_res = String::new();
        let mut split_res = String::new();

        //exercise
        let mut base_stream = source.split();
        read_to(&mut base_stream, &mut base_res, 10);

        let mut split_stream = base_stream.split();
        read_to(&mut split_stream, &mut split_res, 5);

        read_all(&mut base_stream, &mut base_res);
        read_all(&mut split_stream, &mut split_res);

        //verify
        assert_eq!(base_res, "abcdefghijklmnopqrstuvwxyz");
        assert_eq!(split_res, "klmnopqrstuvwxyz");
    }

    #[test]
    fn split_detach_tail() {
        //setup
        let input = "abcdefghijklmnopqrstuvwxyz".to_string();
        let mut iter = input.chars();

        let mut getter = || {
            iter.next()
        };

        let mut source = StreamSource::observe(&mut getter);

        let mut stream1_res = String::new();
        let mut stream2_res = String::new();
        let mut stream3_res = String::new();

        //exercise
        let mut stream1 = source.split();
        read_to(&mut stream1, &mut stream1_res, 6);

        let mut stream2 = stream1.split();
        read_to(&mut stream2, &mut stream2_res, 6);

        stream2.detach_tail();

        let mut stream3 = stream2.split();
        read_to(&mut stream3, &mut stream3_res, 6);

        stream2 = stream3.detach_head().unwrap();
        read_all(&mut stream2, &mut stream2_res);

        //verify
        assert!(stream2.detach_head().is_none());

        assert_eq!(stream1_res, "abcdef");
        assert_eq!(stream2_res, "ghijklmnopqrstuvwxyz");
        assert_eq!(stream3_res, "mnopqr");
    }

    #[test]
    fn deep_split() {
        //setup
        let input = "abcdefghijklmnopqrstuvwxyz".to_string();
        let mut iter = input.chars();

        let mut getter = || {
            iter.next()
        };

        let mut source = StreamSource::observe(&mut getter);
        let mut stream = source.split();

        let mut res = String::new();

        //exercise
        for _ in 0..12 {
            read_to(&mut stream, &mut res, 1);
            stream = stream.split();
        }

        read_to(&mut stream, &mut res, 1);

        for _ in 0..12 {
            read_to(&mut stream, &mut res, 1);
            stream = stream.detach_head().unwrap();
        }

        read_to(&mut stream, &mut res, 1);

        //verify
        assert!(stream.detach_head().is_none());

        assert_eq!(res, "abcdefghijklmnmlkjihgfedcb");
    }

    fn read_to(stream: &mut Stream<char>, to: &mut String, n: usize) {
        for _ in 0..n {
            to.push(stream.pull().unwrap())
        }
    }

    fn read_all(stream: &mut Stream<char>, to: &mut String) {
        while stream.has_next() {
            to.push(stream.pull().unwrap())
        }
    }
}
