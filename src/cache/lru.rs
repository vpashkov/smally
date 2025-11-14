use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::ptr::NonNull;

pub struct LruCache<K, V> {
    capacity: usize,
    map: HashMap<K, NonNull<Node<K, V>>>,
    head: UnsafeCell<Option<NonNull<Node<K, V>>>>,
    tail: UnsafeCell<Option<NonNull<Node<K, V>>>>,
}

struct Node<K, V> {
    key: K,
    value: V,
    prev: Option<NonNull<Node<K, V>>>,
    next: Option<NonNull<Node<K, V>>>,
}

impl<K: Clone + Eq + Hash, V: Clone> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        LruCache {
            capacity,
            map: HashMap::new(),
            head: UnsafeCell::new(None),
            tail: UnsafeCell::new(None),
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let node_ptr = self.map.get(key).copied()?;

        // Move to front
        unsafe {
            self.detach(node_ptr);
            self.attach(node_ptr);
        }

        Some(unsafe { node_ptr.as_ref().value.clone() })
    }

    pub fn put(&mut self, key: K, value: V) {
        if let Some(&node_ptr) = self.map.get(&key) {
            unsafe {
                (*node_ptr.as_ptr()).value = value;
                self.detach(node_ptr);
                self.attach(node_ptr);
            }
            return;
        }

        let node = Box::new(Node {
            key: key.clone(),
            value,
            prev: None,
            next: None,
        });

        let node_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(node)) };
        self.map.insert(key, node_ptr);

        unsafe {
            self.attach(node_ptr);
        }

        if self.map.len() > self.capacity {
            self.pop_back();
        }
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    #[allow(dead_code)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    unsafe fn attach(&self, node: NonNull<Node<K, V>>) {
        let head_ptr = self.head.get();
        match *head_ptr {
            Some(head) => {
                (*node.as_ptr()).next = Some(head);
                (*node.as_ptr()).prev = None;
                (*head.as_ptr()).prev = Some(node);
                *head_ptr = Some(node);
            }
            None => {
                (*node.as_ptr()).prev = None;
                (*node.as_ptr()).next = None;
                *head_ptr = Some(node);
                *self.tail.get() = Some(node);
            }
        }
    }

    unsafe fn detach(&self, node: NonNull<Node<K, V>>) {
        match (*node.as_ptr()).prev {
            Some(prev) => {
                (*prev.as_ptr()).next = (*node.as_ptr()).next;
            }
            None => {
                *self.head.get() = (*node.as_ptr()).next;
            }
        }

        match (*node.as_ptr()).next {
            Some(next) => {
                (*next.as_ptr()).prev = (*node.as_ptr()).prev;
            }
            None => {
                *self.tail.get() = (*node.as_ptr()).prev;
            }
        }
    }

    fn pop_back(&mut self) {
        unsafe {
            if let Some(tail) = *self.tail.get() {
                let key = (*tail.as_ptr()).key.clone();
                self.detach(tail);
                self.map.remove(&key);
                let _ = Box::from_raw(tail.as_ptr());
            }
        }
    }
}

impl<K, V> Drop for LruCache<K, V> {
    fn drop(&mut self) {
        // Clear the map which owns all the nodes
        self.map.clear();
        // The UnsafeCells will be dropped automatically
    }
}

unsafe impl<K: Send, V: Send> Send for LruCache<K, V> {}
unsafe impl<K: Sync, V: Sync> Sync for LruCache<K, V> {}
