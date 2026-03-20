# The Bouncer at the Door: LRU Cache for Offline Workouts

## The Problem

Your GrindIt PWA is beautiful. Athletes can browse workouts, log scores, check the leaderboard — all from their phone. Then they walk into the gym basement where the WiFi signal goes to die.

Mid-workout, your athlete taps "Log Score" and gets... nothing. A blank screen. The spinning wheel of despair. Their 185lb clean & jerk PR, gone like chalk dust in the wind.

"Fine," you say, "I'll cache everything." So you stuff every page, every workout, every image into the browser's cache. It works great — for about a week. Then your athlete's phone storage fills up, their other apps start complaining, and they uninstall your app in frustration.

Cache everything? Out of memory. Cache nothing? Useless offline. You need something smarter.

## The Naive Way

Your first instinct: cache the last N pages in a `Vec`, and when it's full, scan for the oldest entry and remove it.

```rust,ignore
struct NaiveCache {
    entries: Vec<(String, String, u64)>, // (url, html, last_accessed_timestamp)
    capacity: usize,
}

impl NaiveCache {
    fn get(&mut self, url: &str) -> Option<&str> {
        // Scan every entry to find the URL: O(n)
        for entry in &mut self.entries {
            if entry.0 == url {
                entry.2 = current_timestamp();
                return Some(&entry.1);
            }
        }
        None
    }

    fn put(&mut self, url: String, html: String) {
        if self.entries.len() >= self.capacity {
            // Scan every entry to find the oldest: O(n)
            let mut oldest_idx = 0;
            let mut oldest_time = u64::MAX;
            for (i, entry) in self.entries.iter().enumerate() {
                if entry.2 < oldest_time {
                    oldest_time = entry.2;
                    oldest_idx = i;
                }
            }
            self.entries.remove(oldest_idx); // O(n) shift
        }
        self.entries.push((url, html, current_timestamp()));
    }
}
```

Every `get` is O(n). Every `put` that triggers eviction is O(n). When your cache holds 500 workout pages and the athlete is frantically tapping between exercises mid-WOD, that lag adds up. Each tap triggers a linear scan through every cached page.

## The Insight

Think of your gym's membership. Every time a member swipes in, the front desk mentally moves them to the "recently active" list. When the gym hits capacity and someone new walks in, who gets asked to leave? Not the person who just walked in five minutes ago — the person who hasn't shown up in three months.

What if your cache worked the same way? Keep a line. Every time someone's accessed, they move to the front. When you're full, kick whoever's at the back. That's **LRU — Least Recently Used**.

But moving someone to the front of an array means shifting everyone else. We need a data structure where "move to front" is instant. Enter the **doubly-linked list** — paired with a **HashMap** for O(1) lookups.

## The Build

In Rust, we can't casually throw around pointers into a linked list. The borrow checker would have words. Instead, we use the **arena pattern**: store nodes in a `Vec`, reference them by index. Safe, fast, and the borrow checker stays happy.

```rust
use std::collections::HashMap;

struct Node {
    key: String,
    value: String,
    prev: Option<usize>,
    next: Option<usize>,
}

pub struct LruCache {
    capacity: usize,
    map: HashMap<String, usize>,  // key -> index in nodes
    nodes: Vec<Node>,
    head: Option<usize>,  // most recently used
    tail: Option<usize>,  // least recently used
    free: Vec<usize>,     // recycled node slots
}
```

Here's the mental picture — most recent at the head, least recent at the tail:

```
  HEAD                                              TAIL
   |                                                  |
   v                                                  v
[/workout/today] <-> [/scores/mine] <-> [/exercises] <-> [/history]
  (just viewed)                                   (evict me first!)
```

Now let's implement it piece by piece.

```rust
impl LruCache {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Cache capacity must be positive");
        LruCache {
            capacity,
            map: HashMap::with_capacity(capacity),
            nodes: Vec::with_capacity(capacity),
            head: None,
            tail: None,
            free: Vec::new(),
        }
    }

    /// Allocate a node — reuse a freed slot or push new.
    fn alloc_node(&mut self, key: String, value: String) -> usize {
        if let Some(idx) = self.free.pop() {
            self.nodes[idx] = Node { key, value, prev: None, next: None };
            idx
        } else {
            let idx = self.nodes.len();
            self.nodes.push(Node { key, value, prev: None, next: None });
            idx
        }
    }

    /// Detach a node from wherever it sits in the list.
    fn detach(&mut self, idx: usize) {
        let prev = self.nodes[idx].prev;
        let next = self.nodes[idx].next;

        match prev {
            Some(p) => self.nodes[p].next = next,
            None => self.head = next, // was the head
        }
        match next {
            Some(n) => self.nodes[n].prev = prev,
            None => self.tail = prev, // was the tail
        }

        self.nodes[idx].prev = None;
        self.nodes[idx].next = None;
    }

    /// Push a node to the front (most recently used).
    fn push_front(&mut self, idx: usize) {
        self.nodes[idx].next = self.head;
        self.nodes[idx].prev = None;

        if let Some(old_head) = self.head {
            self.nodes[old_head].prev = Some(idx);
        }
        self.head = Some(idx);

        if self.tail.is_none() {
            self.tail = Some(idx);
        }
    }

    /// Get a cached page. Moves it to the front (just visited the gym!).
    pub fn get(&mut self, key: &str) -> Option<&str> {
        if let Some(&idx) = self.map.get(key) {
            self.detach(idx);
            self.push_front(idx);
            Some(&self.nodes[idx].value)
        } else {
            None
        }
    }

    /// Cache a page. Evicts the least recently used if at capacity.
    pub fn put(&mut self, key: String, value: String) {
        // If key already exists, update it and move to front
        if let Some(&idx) = self.map.get(&key) {
            self.nodes[idx].value = value;
            self.detach(idx);
            self.push_front(idx);
            return;
        }

        // Evict if at capacity
        if self.map.len() >= self.capacity {
            if let Some(tail_idx) = self.tail {
                self.detach(tail_idx);
                let evicted_key = self.nodes[tail_idx].key.clone();
                self.map.remove(&evicted_key);
                self.free.push(tail_idx);
            }
        }

        // Insert new node at front
        let idx = self.alloc_node(key.clone(), value);
        self.push_front(idx);
        self.map.insert(key, idx);
    }
}
```

## The Payoff

Let's simulate the gym WiFi dropping mid-workout:

```rust
fn main() {
    // Phone can cache 3 workout pages
    let mut cache = LruCache::new(3);

    // Athlete browses before hitting the gym
    cache.put("/workout/monday".into(), "<h1>Monday: Fran</h1>...".into());
    cache.put("/workout/tuesday".into(), "<h1>Tuesday: Grace</h1>...".into());
    cache.put("/exercises/thruster".into(), "<h1>Thruster</h1>...".into());

    // Cache is full: [thruster] <-> [tuesday] <-> [monday]

    // Athlete opens Monday's workout (moves to front)
    assert!(cache.get("/workout/monday").is_some());
    // Now: [monday] <-> [thruster] <-> [tuesday]

    // WiFi drops! Athlete needs the score log page
    cache.put("/log/score".into(), "<h1>Log Score</h1>...".into());
    // Tuesday was LRU, gets evicted
    // Now: [log/score] <-> [monday] <-> [thruster]

    // Monday's workout is still cached — they can log their score!
    assert!(cache.get("/workout/monday").is_some());
    // Tuesday is gone (they don't need it today)
    assert!(cache.get("/workout/tuesday").is_none());

    println!("WiFi is down, but the athlete still logged their PR.");
}
```

The cache kept exactly what mattered — the pages the athlete was actively using — and evicted the stale ones. No full-storage panic, no blank screens mid-WOD.

## Complexity Comparison

| Operation | Naive (Vec scan) | LRU Cache (HashMap + Linked List) |
|-----------|-----------------|-----------------------------------|
| Get / Lookup | O(n) | **O(1)** |
| Put (insert) | O(1) | **O(1)** |
| Evict oldest | O(n) scan + O(n) shift | **O(1)** tail removal |
| Move to front | O(n) shift | **O(1)** pointer rewire |
| Space | O(n) | O(n) + HashMap overhead |

Every operation that matters in a real-time PWA — lookup, insert, evict — drops from linear to constant time.

## Try It Yourself

1. **Peek without promoting**: Add a `peek(&self, key: &str) -> Option<&str>` method that checks if a key is cached but does NOT move it to the front. Useful for "is this page available offline?" checks in the UI.

2. **Cache statistics**: Track hit count and miss count. Add a `hit_rate(&self) -> f64` method. Display it in the PWA's debug panel so you can tune the cache size.

3. **TTL expiration**: Add a `created_at: u64` field to each node. Modify `get` to return `None` (and evict the node) if the entry is older than a configurable TTL. Stale workout data from last week shouldn't be served if the athlete is online and can fetch fresh data.
