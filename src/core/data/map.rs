pub struct CEHashMap<V: Default> {
    vector: Vec<V>
}

#[allow(dead_code)]
impl<V: Default> CEHashMap<V> {
    pub fn new() -> CEHashMap<V> {
        CEHashMap {
            vector: Vec::new()
        }
    }

    pub fn insert(&mut self, key: usize, value: V) {
        while self.vector.len() <= key {
            self.vector.push(V::default());
        }
        self.vector.insert(key, value)
    }

    pub fn get(&self, key: usize) -> Option<&V> {
        self.vector.get(key)
    }

    pub fn get_mut(&mut self, key: usize) -> Option<&mut V> {
        self.vector.get_mut(key)
    }

    pub fn contains(&self, key: usize) -> bool {
        self.get(key).is_some()
    }

    pub fn size(&self) -> usize {
        self.vector.len()
    }
}
