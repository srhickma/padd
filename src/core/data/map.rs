/// Continuously Encoded Hash Map: Keys are integers which index values directly in a vector.
/// Efficient when the key space is very dense (e.g. when storing half a bijection).
///
/// # Type Parameters
///
/// * `V` - the type of the values stored in the map.
///
/// # Fields
///
/// * `vector` - the underlying vector which stores the values of the map, and is indexed by CEHashMap keys.
pub struct CEHashMap<V: Default> {
    vector: Vec<Option<V>>,
}

#[allow(dead_code)]
impl<V: Default> CEHashMap<V> {
    /// Returns a new empty CEHashMap.
    pub fn new() -> CEHashMap<V> {
        CEHashMap { vector: Vec::new() }
    }

    /// Inserts a value into the map at the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - the key at which to insert the value in the map.
    /// * `value` - the value to be inserted into the map.
    pub fn insert(&mut self, key: usize, value: V) {
        while self.vector.len() <= key {
            self.vector.push(None);
        }
        self.vector[key] = Some(value);
    }

    /// Returns a reference to the value stored in the map at the given key, or `None` if no such value is stored.
    ///
    /// # Arguments
    ///
    /// * `key` - the key to look up in the map.
    pub fn get(&self, key: usize) -> Option<&V> {
        match self.vector.get(key) {
            None => None,
            Some(opt) => opt.as_ref(),
        }
    }

    /// Returns a _mutable_ reference to the value stored in the map at the given key, or `None` if no such value is stored.
    ///
    /// # Arguments
    ///
    /// * `key` - the key to look up in the map.
    pub fn get_mut(&mut self, key: usize) -> Option<&mut V> {
        match self.vector.get_mut(key) {
            None => None,
            Some(opt) => opt.as_mut(),
        }
    }

    /// Returns `true` if there is a value stored in the map under the given key, and `false` otherwise.
    ///
    /// # Arguments
    ///
    /// * `key` - the key to look up in the map.
    pub fn contains(&self, key: usize) -> bool {
        self.get(key).is_some()
    }

    /// Returns the size of the map (i.e. the number of values stored).
    pub fn size(&self) -> usize {
        self.vector.len()
    }

    /// Returns an iterator over the values of the map, which starts at low keys and iterates to high keys.
    pub fn iter(&self) -> CEHashMapIterator<V> {
        CEHashMapIterator {
            map: &self,
            index: 0,
        }
    }
}

/// Continuously Encoded Hash Map Iterator: An iterator over the values of a CEHashMap.
///
/// # Type Parameters
///
/// * `V` - the type of the values stored in the map.
///
/// # Fields
///
/// * `map` - a reference to the map being iterated over.
/// * `index` - the index (key) of the current element in the map.
pub struct CEHashMapIterator<'scope, V: Default + 'scope> {
    map: &'scope CEHashMap<V>,
    index: usize,
}

impl<'scope, V: Default + 'scope> Iterator for CEHashMapIterator<'scope, V> {
    type Item = &'scope V;

    /// Returns a reference to the current value in the iteration, or `None` if the iteration is complete.
    /// The current value is then set to the next value in the iteration.
    fn next(&mut self) -> Option<&'scope V> {
        while self.index < self.map.size() {
            let entry = self.map.get(self.index);
            self.index += 1;
            if entry.is_some() {
                return entry;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_fwd() {
        //setup
        let mut map: CEHashMap<usize> = CEHashMap::new();

        //exercise
        for i in 0..100 {
            map.insert(i, i * i);
        }

        //verify
        for i in 0..100 {
            assert_eq!(*map.get(i).unwrap(), i * i);
        }
    }

    #[test]
    fn sequence_rev() {
        //setup
        let mut map: CEHashMap<usize> = CEHashMap::new();

        //exercise
        for i in (0..100).rev() {
            map.insert(i, i * i);
        }

        //verify
        for i in 0..100 {
            assert_eq!(*map.get(i).unwrap(), i * i);
        }
    }

    #[test]
    fn insert_beyond_bounds() {
        //setup
        let mut map: CEHashMap<usize> = CEHashMap::new();

        //exercise
        map.insert(2, 2);
        map.insert(1, 1);

        //verify
        assert_eq!(*map.get(2).unwrap(), 2);
        assert_eq!(*map.get(1).unwrap(), 1);
    }

    #[test]
    fn empty_get() {
        //setup
        let map: CEHashMap<usize> = CEHashMap::new();

        //exercise
        let res = map.get(0);

        //verify
        assert!(res.is_none());
    }

    #[test]
    fn contains() {
        //setup
        let mut map: CEHashMap<usize> = CEHashMap::new();

        //exercise
        map.insert(2, 4);
        map.insert(1, 5);

        //verify
        assert!(!map.contains(0));
        assert!(map.contains(1));
        assert!(map.contains(2));
        assert!(!map.contains(3));
        assert!(!map.contains(4));
    }

    #[test]
    fn size() {
        //setup
        let mut map: CEHashMap<usize> = CEHashMap::new();

        //exercise/verify
        assert_eq!(map.size(), 0);
        map.insert(2, 0);
        assert_eq!(map.size(), 3);
        map.insert(1, 0);
        assert_eq!(map.size(), 3);
        map.insert(50, 0);
        assert_eq!(map.size(), 51);
    }

    #[test]
    fn iterator() {
        //setup
        let mut map: CEHashMap<usize> = CEHashMap::new();

        //exercise/verify
        {
            let mut iter = map.iter();
            assert_eq!(iter.next(), None);
        }

        map.insert(2, 7);
        map.insert(1, 8);
        map.insert(50, 9);

        let mut iter = map.iter();
        assert_eq!(iter.next(), Some(&8));
        assert_eq!(iter.next(), Some(&7));
        assert_eq!(iter.next(), Some(&9));
        assert_eq!(iter.next(), None);
    }
}
