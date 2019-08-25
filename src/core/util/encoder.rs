use {
    core::data::Data,
    std::{collections::HashMap, fmt},
};

/// Encoder: Used to encode and decode structs as `usize`.
///
/// # Type Parameters
///
/// * `T` - The type to be encoded from and decoded to.
///
/// # Fields
///
/// * `encoder` - the mapping from decoded to encoded objects.
/// * `decoder` - the mapping from encoded to decoded objects, where the keys of the mapping are
/// the vector indices.
pub struct Encoder<T: Data> {
    encoder: HashMap<T, usize>,
    decoder: Vec<T>,
}

impl<T: Data> Encoder<T> {
    /// Returns a new encoder.
    pub fn new() -> Self {
        Encoder {
            encoder: HashMap::new(),
            decoder: Vec::new(),
        }
    }

    /// Returns the encoded representation of an object.
    ///
    /// # Parameters
    ///
    /// * `val` - the object to encode.
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

    /// Returns the decoded representation of a cipher, or `None` if the cipher text does
    /// not correspond to an encoded object.
    ///
    /// # Parameters
    ///
    /// * `cipher_text` - the encoded value to decode.
    pub fn decode(&self, cipher_text: usize) -> Option<&T> {
        self.decoder.get(cipher_text)
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
