pub struct CEHashMap<V: Default> {
    vector: Vec<V>
}

impl<V: Default> CEHashMap<V> {
    pub fn insert(&mut self, key: usize, value: V) {
        while self.vector.len() <= key {
            self.vector.push(V::default());
        }
        self.vector.insert(key, value)
    }

    pub fn get(&self, key: usize) -> Option<&V> {
        self.vector.get(key)
    }

    pub fn contains(&self, key: usize) -> bool {
        self.get(key).is_some()
    }
}