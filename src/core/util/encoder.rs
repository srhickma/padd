use {
    core::data::Data,
    std::{collections::HashMap, fmt},
};

pub struct Encoder<T: Default + Data> {
    encoder: HashMap<T, usize>,
    decoder: Vec<T>,
}

impl<T: Default + Data> Encoder<T> {
    pub fn new() -> Self {
        Encoder {
            encoder: HashMap::new(),
            decoder: Vec::new(),
        }
    }

    pub fn encode(&mut self, val: &T) -> usize {
        if self.encoder.contains_key(val) {
            self.encoder[val]
        } else {
            let key = self.decoder.len();
            self.decoder.push(val.clone());
            self.encoder.insert(val.clone(), key);
            key
        }
    }

    pub fn decode(&self, cipher: usize) -> Option<&T> {
        self.decoder.get(cipher)
    }
}

impl<T: Default + Data> fmt::Debug for Encoder<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Encoder")
    }
}

//TODO(shane) add tests here

