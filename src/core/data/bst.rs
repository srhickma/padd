pub trait BinarySearchTree<Key: Ord, Value> {
    fn insert(&mut self, key: Key, value: Value);
    fn search(&self, key: &Key) -> Option<&Value>;
    fn search_mut(&mut self, key: &Key) -> Option<&mut Value>;
}

pub struct AVLTree<Key: Ord, Value> {
    nodes: Vec<AVLTreeNode<Key, Value>>,
    root_index: Option<usize>,
}

impl<Key: Ord, Value> AVLTree<Key, Value> {
    pub fn new() -> Self {
        AVLTree {
            nodes: Vec::new(),
            root_index: None,
        }
    }

    fn node(&self, index: usize) -> &AVLTreeNode<Key, Value> {
        &self.nodes[index]
    }

    fn node_mut(&mut self, index: usize) -> &mut AVLTreeNode<Key, Value> {
        &mut self.nodes[index]
    }

    fn add_node(&mut self, key: Key, value: Value, parent: Option<usize>) -> usize {
        let index = self.nodes.len();
        self.nodes.push(AVLTreeNode::new(key, value, parent, index));
        index
    }

    // TODO(shane) can we simplify these match statements?
    fn search_path(
        &self,
        node: &AVLTreeNode<Key, Value>,
        key: &Key
    ) -> (bool, usize) {
        if *key == node.key {
            (true, node.index)
        } else if *key < node.key {
            match node.left {
                None => (false, node.index),
                Some(_) => self.search_path(self.node(node.left.unwrap()), key),
            }
        } else {
            match node.right {
                None => (false, node.index),
                Some(_) => self.search_path(self.node(node.right.unwrap()), key),
            }
        }
    }
}

impl<Key: Ord, Value> BinarySearchTree<Key, Value> for AVLTree<Key, Value> {
    fn insert(&mut self, key: Key, value: Value) {
        match self.root_index {
            None => self.root_index = Some(self.add_node(key, value, None)),
            Some(root_index) => {
                let (ok, last_node_index) = self.search_path(self.node(root_index), &key);
                if !ok {
                    let left_child = key < self.node(last_node_index).key;
                    let new_node_index = self.add_node(key, value, Some(last_node_index));
                    let parent = self.node_mut(last_node_index);

                    if left_child {
                        parent.left = Some(new_node_index);
                    } else {
                        parent.right = Some(new_node_index);
                    }

                    // TODO(shane) update balances.
                    // TODO(shane) rotate if imbalanced.
                }
            },
        }
    }

    fn search(&self, key: &Key) -> Option<&Value> {
        match self.root_index {
            Some(root_index) => {
                let (ok, last_node_index) = self.search_path(self.node(root_index), key);
                if ok {
                    Some(&self.node(last_node_index).value)
                } else {
                    None
                }
            },
            None => None,
        }
    }

    fn search_mut(&mut self, key: &Key) -> Option<&mut Value> {
        match self.root_index {
            Some(root_index) => {
                let (ok, last_node_index) = self.search_path(self.node(root_index), key);
                if ok {
                    Some(&mut self.node_mut(last_node_index).value)
                } else {
                    None
                }
            },
            None => None,
        }
    }
}

#[derive(Debug)]
struct AVLTreeNode<Key: Ord, Value> {
    key: Key,
    value: Value,
    left: Option<usize>,
    right: Option<usize>,
    parent: Option<usize>,
    balance: i8,
    index: usize,
}

impl<Key: Ord, Value> AVLTreeNode<Key, Value> {
    fn new(key: Key, value: Value, parent: Option<usize>, index: usize) -> Self {
        AVLTreeNode {
            key,
            value,
            left: None,
            right: None,
            parent,
            balance: 0,
            index,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avl_empty_search() {
        //setup
        let tree: AVLTree<usize, ()> = AVLTree::new();

        //exercise
        let node = tree.search(&4);

        //verify
        assert!(node.is_none());
    }

    #[test]
    fn avl_empty_search_mut() {
        //setup
        let mut tree: AVLTree<usize, ()> = AVLTree::new();

        //exercise
        let node = tree.search_mut(&4);

        //verify
        assert!(node.is_none());
    }

    #[test]
    fn avl_insert_search_simple() {
        //setup
        let mut tree: AVLTree<usize, &str> = AVLTree::new();

        //exercise
        tree.insert(2, "a");
        tree.insert(1, "b");
        tree.insert(3, "c");

        //verify
        assert_eq!(tree.search(&1), Some(&"b"));
        assert_eq!(tree.search(&2), Some(&"a"));
        assert_eq!(tree.search(&3), Some(&"c"));
    }

    #[test]
    fn avl_insert_search_complex() {
        //setup
        let mut tree: AVLTree<usize, usize> = AVLTree::new();

        //exercise/verify
        tree.insert(1, 1);
        tree.insert(2, 2);
        tree.insert(3, 3);
        tree.insert(4, 4);
        tree.insert(5, 5);
        tree.insert(6, 6);
        tree.insert(256, 256);
        tree.insert(0, 0);
        tree.insert(11, 11);
        tree.insert(7, 7);
        tree.insert(15, 15);
        tree.insert(14, 14);
        tree.insert(13, 13);
        tree.insert(12, 12);
        tree.insert(99, 99);
        tree.insert(100, 100);

        assert_eq!(tree.search(&1), Some(&1));
        assert_eq!(tree.search(&2), Some(&2));
        assert_eq!(tree.search(&3), Some(&3));
        assert_eq!(tree.search(&4), Some(&4));
        assert_eq!(tree.search(&5), Some(&5));
        assert_eq!(tree.search(&6), Some(&6));
        assert_eq!(tree.search(&256), Some(&256));
        assert_eq!(tree.search(&0), Some(&0));
        assert_eq!(tree.search(&11), Some(&11));
        assert_eq!(tree.search(&7), Some(&7));
        assert_eq!(tree.search(&15), Some(&15));
        assert_eq!(tree.search(&14), Some(&14));
        assert_eq!(tree.search(&13), Some(&13));
        assert_eq!(tree.search(&12), Some(&12));
        assert_eq!(tree.search(&99), Some(&99));
        assert_eq!(tree.search(&100), Some(&100));
    }

    #[test]
    fn avl_insert_search_complex_mut() {
        //setup
        let mut tree: AVLTree<usize, usize> = AVLTree::new();

        //exercise/verify
        tree.insert(1, 1);
        tree.insert(2, 2);
        tree.insert(3, 3);
        tree.insert(4, 4);
        tree.insert(5, 5);
        tree.insert(6, 6);
        tree.insert(256, 256);
        tree.insert(0, 0);
        tree.insert(11, 11);
        tree.insert(7, 7);
        tree.insert(15, 15);
        tree.insert(14, 14);
        tree.insert(13, 13);
        tree.insert(12, 12);
        tree.insert(99, 99);
        tree.insert(100, 100);

        assert_eq!(tree.search_mut(&1), Some(&mut 1));
        assert_eq!(tree.search_mut(&2), Some(&mut 2));
        assert_eq!(tree.search_mut(&3), Some(&mut 3));
        assert_eq!(tree.search_mut(&4), Some(&mut 4));
        assert_eq!(tree.search_mut(&5), Some(&mut 5));
        assert_eq!(tree.search_mut(&6), Some(&mut 6));
        assert_eq!(tree.search_mut(&256), Some(&mut 256));
        assert_eq!(tree.search_mut(&0), Some(&mut 0));
        assert_eq!(tree.search_mut(&11), Some(&mut 11));
        assert_eq!(tree.search_mut(&7), Some(&mut 7));
        assert_eq!(tree.search_mut(&15), Some(&mut 15));
        assert_eq!(tree.search_mut(&14), Some(&mut 14));
        assert_eq!(tree.search_mut(&13), Some(&mut 13));
        assert_eq!(tree.search_mut(&12), Some(&mut 12));
        assert_eq!(tree.search_mut(&99), Some(&mut 99));
        assert_eq!(tree.search_mut(&100), Some(&mut 100));
    }

    #[test]
    fn avl_insert_duplicates() {
        //setup
        let mut tree: AVLTree<usize, &str> = AVLTree::new();

        //exercise
        tree.insert(1, "a");
        tree.insert(1, "b");

        //verify
        assert_eq!(tree.search(&1), Some(&"a"));
    }

    #[test]
    fn avl_search_missing() {
        //setup
        let mut tree: AVLTree<usize, ()> = AVLTree::new();
        tree.insert(1, ());
        tree.insert(2, ());
        tree.insert(3, ());
        tree.insert(5, ());

        //exercise/verify
        assert_eq!(tree.search(&0), None);
        assert_eq!(tree.search(&4), None);
        assert_eq!(tree.search(&6), None);
    }

    #[test]
    fn avl_search_for_update() {
        //setup
        let mut tree: AVLTree<usize, &str> = AVLTree::new();
        tree.insert(1, "original text");

        //exercise
        *tree.search_mut(&1).unwrap() = "new text";

        //verify
        assert_eq!(tree.search(&1), Some(&"new text"));
    }
}
