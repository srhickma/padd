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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn encode_decode() {
        //setup
        let mut encoder: Encoder<usize> = Encoder::new();
        let start = 10;
        let numbers: Vec<usize> = (start..(start + 40)).collect();

        //exercise/verify
        for number in &numbers {
            assert_eq!(encoder.encode(number), number - start);
        }

        for number in &numbers {
            assert_eq!(encoder.encode(number), number - start);
        }

        for i in 0..30 {
            assert_eq!(*encoder.decode(i).unwrap(), i + start);
        }
    }

    #[test]
    fn decode_non_existent() {
        //setup
        let encoder: Encoder<String> = Encoder::new();

        //exercise
        let res = encoder.decode(1);

        //verify
        assert_eq!(res, None);
    }

    #[test]
    fn fmt() {
        //setup
        let encoder: Encoder<String> = Encoder::new();

        //exercise
        let res = format!("{:?}", encoder);

        //verify
        assert_eq!(res, "Encoder");
    }
}
