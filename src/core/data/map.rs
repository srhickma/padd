pub struct CEHashMap<V: Default> {
    vector: Vec<Option<V>>,
}

#[allow(dead_code)]
impl<V: Default> CEHashMap<V> {
    pub fn new() -> CEHashMap<V> {
        CEHashMap { vector: Vec::new() }
    }

    pub fn insert(&mut self, key: usize, value: V) {
        while self.vector.len() <= key {
            self.vector.push(None);
        }
        self.vector[key] = Some(value);
    }

    pub fn get(&self, key: usize) -> Option<&V> {
        match self.vector.get(key) {
            None => None,
            Some(opt) => opt.as_ref(),
        }
    }

    pub fn get_mut(&mut self, key: usize) -> Option<&mut V> {
        match self.vector.get_mut(key) {
            None => None,
            Some(opt) => opt.as_mut(),
        }
    }

    pub fn contains(&self, key: usize) -> bool {
        self.get(key).is_some()
    }

    pub fn size(&self) -> usize {
        self.vector.len()
    }

    pub fn iter(&self) -> CEHashMapIterator<V> {
        CEHashMapIterator {
            map: &self,
            index: 0,
        }
    }
}

pub struct CEHashMapIterator<'scope, V: Default + 'scope> {
    map: &'scope CEHashMap<V>,
    index: usize,
}

impl<'scope, V: Default + 'scope> Iterator for CEHashMapIterator<'scope, V> {
    type Item = &'scope V;
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
