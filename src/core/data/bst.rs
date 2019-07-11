use std::cell::RefCell;

pub trait BinarySearchTree<Key: Ord, Value, Node: BSTNode<Key, Value>> {
    fn insert(&mut self, key: Key, value: Value);
    fn root(&self) -> Option<&Node>;
    fn root_mut(&mut self) -> Option<&mut Node>;

    fn search<'scope>(&'scope self, key: &Key) -> Option<&'scope Value> where Node: 'scope {
        let (ok, last_node) = BinarySearchTree::search_path(self.root(), None, key);
        if ok {
            last_node.map(BSTNode::value)
        } else {
            None
        }
    }

    fn search_mut<'scope>(
        &'scope mut self,
        key: &Key
    ) -> Option<&'scope mut Value> where Node: 'scope {
        match self.root_mut() {
            Some(root_node) => {
                let (ok, last_node) = BinarySearchTree::search_path_mut(root_node, key);
                if ok {
                    Some(last_node.value_mut())
                } else {
                    None
                }
            },
            None => None,
        }
    }
}

impl<Key: Ord, Value, Node: BSTNode<Key, Value>> BinarySearchTree<Key, Value, Node> {
    fn search_path<'tree>(
        node_opt: Option<&'tree Node>,
        previous: Option<&'tree Node>,
        key: &Key
    ) -> (bool, Option<&'tree Node>) {
        match node_opt {
            None => (false, previous),
            Some(node) => {
                if *key == *node.key() {
                    (true, node_opt)
                } else if *key < *node.key() {
                    BinarySearchTree::search_path(node.left(), node_opt, key)
                } else {
                    BinarySearchTree::search_path(node.right(), node_opt, key)
                }
            }
        }
    }

    fn search_path_mut<'tree>(
        node: &'tree mut Node,
        key: &Key
    ) -> (bool, &'tree mut Node) {
        if *key == *node.key() {
            (true, node)
        } else if *key < *node.key() {
            match node.left() {
                None => (false, node),
                Some(_) => BinarySearchTree::search_path_mut(node.left_mut().unwrap(), key),
            }
        } else {
            match node.right() {
                None => (false, node),
                Some(_) => BinarySearchTree::search_path_mut(node.right_mut().unwrap(), key),
            }
        }
    }
}

pub trait BSTNode<Key: Ord, Value> {
    fn key(&self) -> &Key;
    fn value(&self) -> &Value;
    fn value_mut(&mut self) -> &mut Value;
    fn left(&self) -> Option<&Self>;
    fn left_mut(&mut self) -> Option<&mut Self>;
    fn right(&self) -> Option<&Self>;
    fn right_mut(&mut self) -> Option<&mut Self>;
    fn parent(&self) -> Option<&Self>;
    fn parent_mut(&mut self) -> Option<&mut Self>;
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
}

impl<Key: Ord, Value> BinarySearchTree<Key, Value, AVLTreeNode<Key, Value>> for AVLTree<Key, Value> {
    fn insert(&mut self, key: Key, value: Value) {
        match self.root {
            None => self.root = Some(AVLTreeNode::root(key, value)),
            Some(_) => {
                let (ok, last_node) = BinarySearchTree::search_path_mut(self.root.as_mut().unwrap(), &key);
                if !ok {
                    let parent: &mut AVLTreeNode<Key, Value> = last_node;
                    if key < *parent.key() {
                        parent.left = Some(Box::new(AVLTreeNode::root(key, value)));
                        parent.balance -= 1;
                    } else {
                        parent.right = Some(Box::new(AVLTreeNode::root(key, value)));
                        parent.balance += 1;
                    }

                    // TODO(shane) rotate if imbalanced.
                }
            },
        }
    }

    fn root(&self) -> Option<&AVLTreeNode<Key, Value>> {
        self.root.as_ref()
    }

    fn root_mut(&mut self) -> Option<&mut AVLTreeNode<Key, Value>> {
        self.root.as_mut()
    }
}

#[derive(Debug)]
struct AVLTreeNode<Key: Ord, Value> {
    key: Key,
    value: Value,
    left: Option<Box<AVLTreeNode<Key, Value>>>,
    right: Option<Box<AVLTreeNode<Key, Value>>>,
    parent: Option<Box<RefCell<AVLTreeNode<Key, Value>>>>,
    balance: i8,
    index: usize,
}

impl<Key: Ord, Value> AVLTreeNode<Key, Value> {
    fn root(key: Key, value: Value) -> Self {
        AVLTreeNode {
            key,
            value,
            left: None,
            right: None,
            parent: None,
            balance: 0,
        }
    }
}

impl<Key: Ord, Value> BSTNode<Key, Value> for AVLTreeNode<Key, Value> {
    fn key(&self) -> &Key {
        &self.key
    }

    fn value(&self) -> &Value {
        &self.value
    }

    fn value_mut(&mut self) -> &mut Value {
        &mut self.value
    }

    fn left(&self) -> Option<&AVLTreeNode<Key, Value>> {
        self.left.as_ref().map(|boxed_node| &**boxed_node)
    }

    fn left_mut(&mut self) -> Option<&mut AVLTreeNode<Key, Value>> {
        self.left.as_mut().map(|boxed_node| &mut **boxed_node)
    }

    fn right(&self) -> Option<&AVLTreeNode<Key, Value>> {
        self.right.as_ref().map(|boxed_node| &**boxed_node)
    }

    fn right_mut(&mut self) -> Option<&mut AVLTreeNode<Key, Value>> {
        self.right.as_mut().map(|boxed_node| &mut **boxed_node)
    }

    fn parent(&self) -> Option<&Self> {
        self.parent.as_ref().map(|rc_node| &**rc_node)
    }

    fn parent_mut(&mut self) -> Option<&mut Self> {
        self.parent.as_mut().map(|rc_node| &mut **rc_node)
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
