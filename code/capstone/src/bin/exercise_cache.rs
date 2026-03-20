// Problem 7: Exercise Cache — LRU Cache
// HashMap + index-based doubly-linked list, O(1) get/put.
// Run with: cargo run --bin exercise_cache

use std::collections::HashMap;

// --- Brute Force ---

struct LruBrute {
    capacity: usize,
    items: Vec<(String, String)>,
}

impl LruBrute {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            items: Vec::new(),
        }
    }

    fn get(&mut self, key: &str) -> Option<String> {
        if let Some(pos) = self.items.iter().position(|(k, _)| k == key) {
            let item = self.items.remove(pos);
            let value = item.1.clone();
            self.items.push(item);
            Some(value)
        } else {
            None
        }
    }

    fn put(&mut self, key: String, value: String) {
        if let Some(pos) = self.items.iter().position(|(k, _)| *k == key) {
            self.items.remove(pos);
        } else if self.items.len() == self.capacity {
            self.items.remove(0);
        }
        self.items.push((key, value));
    }
}

// --- Optimized: HashMap + index-based linked list ---

struct CacheEntry {
    key: String,
    value: String,
    prev: Option<usize>,
    next: Option<usize>,
}

struct LruCache {
    capacity: usize,
    map: HashMap<String, usize>,
    entries: Vec<CacheEntry>,
    head: Option<usize>,
    tail: Option<usize>,
    free: Vec<usize>,
}

impl LruCache {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: HashMap::new(),
            entries: Vec::new(),
            head: None,
            tail: None,
            free: Vec::new(),
        }
    }

    fn get(&mut self, key: &str) -> Option<String> {
        if let Some(&idx) = self.map.get(key) {
            self.move_to_head(idx);
            Some(self.entries[idx].value.clone())
        } else {
            None
        }
    }

    fn put(&mut self, key: String, value: String) {
        if let Some(&idx) = self.map.get(&key) {
            self.entries[idx].value = value;
            self.move_to_head(idx);
        } else {
            if self.map.len() == self.capacity {
                self.evict_tail();
            }
            let idx = self.allocate(CacheEntry {
                key: key.clone(),
                value,
                prev: None,
                next: self.head,
            });
            if let Some(old_head) = self.head {
                self.entries[old_head].prev = Some(idx);
            }
            self.head = Some(idx);
            if self.tail.is_none() {
                self.tail = Some(idx);
            }
            self.map.insert(key, idx);
        }
    }

    fn move_to_head(&mut self, idx: usize) {
        if self.head == Some(idx) {
            return;
        }

        let prev = self.entries[idx].prev;
        let next = self.entries[idx].next;
        if let Some(p) = prev {
            self.entries[p].next = next;
        }
        if let Some(n) = next {
            self.entries[n].prev = prev;
        }
        if self.tail == Some(idx) {
            self.tail = prev;
        }

        self.entries[idx].prev = None;
        self.entries[idx].next = self.head;
        if let Some(old_head) = self.head {
            self.entries[old_head].prev = Some(idx);
        }
        self.head = Some(idx);
    }

    fn evict_tail(&mut self) {
        if let Some(tail_idx) = self.tail {
            let key = self.entries[tail_idx].key.clone();
            self.tail = self.entries[tail_idx].prev;
            if let Some(new_tail) = self.tail {
                self.entries[new_tail].next = None;
            } else {
                self.head = None;
            }
            self.map.remove(&key);
            self.free.push(tail_idx);
        }
    }

    fn allocate(&mut self, entry: CacheEntry) -> usize {
        if let Some(idx) = self.free.pop() {
            self.entries[idx] = entry;
            idx
        } else {
            self.entries.push(entry);
            self.entries.len() - 1
        }
    }
}

fn main() {
    println!("=== Brute Force LRU ===");
    let mut brute = LruBrute::new(3);
    brute.put("Deadlift".into(), "Posterior chain".into());
    brute.put("Back Squat".into(), "King of legs".into());
    brute.put("Push Press".into(), "Overhead strength".into());
    assert!(brute.get("Deadlift").is_some());
    brute.put("Thruster".into(), "Full body".into());
    assert!(brute.get("Back Squat").is_none());
    println!("  All assertions passed!");

    println!("\n=== Optimized LRU ===");
    let mut cache = LruCache::new(3);
    cache.put("Deadlift".into(), "Posterior chain compound lift".into());
    cache.put("Back Squat".into(), "King of leg exercises".into());
    cache.put("Push Press".into(), "Overhead strength builder".into());

    assert!(cache.get("Deadlift").is_some());
    cache.put("Thruster".into(), "Full body metabolic exercise".into());

    assert!(cache.get("Back Squat").is_none());
    assert!(cache.get("Push Press").is_some());
    assert!(cache.get("Deadlift").is_some());
    assert!(cache.get("Thruster").is_some());

    println!("  All assertions passed — LRU cache works correctly.");
}
