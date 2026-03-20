# HashMap from Scratch: Building the Locker Room

## The Problem

GrindIt now has 500 athletes. Every morning between 5:00 and 6:00 AM, they all open the app and log in. Your authentication code looks like this:

```rust,ignore
struct Athlete {
    email: String,
    name: String,
    password_hash: String,
    box_name: String,
}

fn find_athlete<'a>(athletes: &'a [Athlete], email: &str) -> Option<&'a Athlete> {
    athletes.iter().find(|a| a.email == email)
}
```

Simple. And during the morning rush, catastrophically slow.

The first athlete to log in triggers one comparison. The last triggers 500. On average, each login scans 250 entries. With 500 athletes logging in within minutes, that's 125,000 string comparisons just for authentication. Your coach texts: "the app is slow in the morning when everyone logs in at once."

You stare at that `find` call and think: what if we could skip straight to the right athlete, like opening a locker with a combination?

## The Naive Way

Let's quantify the damage:

```rust,ignore
fn main() {
    // Simulate 500 athletes
    let athletes: Vec<Athlete> = (0..500)
        .map(|i| Athlete {
            email: format!("athlete{}@grindit.com", i),
            name: format!("Athlete {}", i),
            password_hash: format!("hash_{}", i),
            box_name: "CrossFit Downtown".to_string(),
        })
        .collect();

    // Morning rush: everyone logs in
    let mut total_comparisons = 0u64;
    for i in 0..500 {
        let target = format!("athlete{}@grindit.com", i);
        // Worst case: scan through all athletes before this one
        for (idx, athlete) in athletes.iter().enumerate() {
            total_comparisons += 1;
            if athlete.email == target {
                break;
            }
        }
    }

    println!("Total comparisons for 500 logins: {}", total_comparisons);
    // Output: roughly 125,250
}

struct Athlete {
    email: String,
    name: String,
    password_hash: String,
    box_name: String,
}
```

125,000 string comparisons. Each one comparing characters one by one. For something that should be instant.

## The Insight

Think about a locker room. Each locker has a number. You don't walk down the row reading every name tag -- you go straight to locker #247 because that's *your* number. The magic is the function that converts your identity into a locker number.

That function is a **hash function**. It takes a key (like an email address) and converts it into an array index. Instead of scanning 500 athletes, you compute the index and go *directly* there.

Let's build the entire locker room.

## The Build

### The Hash Function

We need a function that turns a string into a number. It should be deterministic (same input always gives same output) and spread values evenly across our array. Here's a simple but effective one:

```rust
fn hash(key: &str, capacity: usize) -> usize {
    let mut hash_value: u64 = 0;
    for byte in key.bytes() {
        // Multiply by a prime and add the byte value
        hash_value = hash_value.wrapping_mul(31).wrapping_add(byte as u64);
    }
    (hash_value % capacity as u64) as usize
}
```

The `wrapping_mul` and `wrapping_add` let the number overflow without panicking -- we only care about the final modulo result anyway.

### The Bucket Array

Two athletes might hash to the same locker number. That's called a **collision**. We handle it with **chaining**: each bucket is a `Vec` of key-value pairs. If two athletes land in the same bucket, they share it.

```rust
struct HashMap<V> {
    buckets: Vec<Vec<(String, V)>>,
    len: usize,
}

impl<V> HashMap<V> {
    fn new() -> Self {
        let initial_capacity = 16;
        let mut buckets = Vec::with_capacity(initial_capacity);
        for _ in 0..initial_capacity {
            buckets.push(Vec::new());
        }
        HashMap { buckets, len: 0 }
    }

    fn capacity(&self) -> usize {
        self.buckets.len()
    }

    fn load_factor(&self) -> f64 {
        self.len as f64 / self.capacity() as f64
    }
}
```

### Insert

Hash the key, find the bucket, check for duplicates, append:

```rust
impl<V> HashMap<V> {
    fn insert(&mut self, key: String, value: V) {
        // Resize if load factor exceeds 0.75
        if self.load_factor() > 0.75 {
            self.resize();
        }

        let index = hash(&key, self.capacity());
        let bucket = &mut self.buckets[index];

        // Check if key already exists -- update in place
        for entry in bucket.iter_mut() {
            if entry.0 == key {
                entry.1 = value;
                return;
            }
        }

        // New key
        bucket.push((key, value));
        self.len += 1;
    }
}
```

### Get

Hash the key, find the bucket, scan only that bucket:

```rust
impl<V> HashMap<V> {
    fn get(&self, key: &str) -> Option<&V> {
        let index = hash(key, self.capacity());
        let bucket = &self.buckets[index];

        for entry in bucket {
            if entry.0 == key {
                return Some(&entry.1);
            }
        }
        None
    }
}
```

This is the magic moment. Instead of scanning 500 athletes, we hash the email, jump to one bucket, and scan maybe 1-3 entries. That's it.

### Resize: When the Locker Room Gets Crowded

When too many athletes share too few buckets, collisions pile up and performance degrades. The **load factor** (items / capacity) tells us when to grow. At 0.75, we double the capacity and rehash everything:

```rust
impl<V> HashMap<V> {
    fn resize(&mut self) {
        let new_capacity = self.capacity() * 2;
        let mut new_buckets = Vec::with_capacity(new_capacity);
        for _ in 0..new_capacity {
            new_buckets.push(Vec::new());
        }

        // Rehash all existing entries into the new buckets
        let old_buckets = std::mem::replace(&mut self.buckets, new_buckets);
        self.len = 0;

        for bucket in old_buckets {
            for (key, value) in bucket {
                // Reinsert into the resized map
                let index = hash(&key, self.capacity());
                self.buckets[index].push((key, value));
                self.len += 1;
            }
        }
    }
}
```

The `std::mem::replace` trick is pure Rust elegance. We swap in the new empty buckets, take ownership of the old ones, and drain them into the new layout. No unsafe code, no dangling pointers.

Resizing is O(n) -- expensive. But it happens so rarely (doubling means it happens O(log n) times total) that the *amortized* cost of insert stays O(1).

### Remove

For completeness:

```rust
impl<V> HashMap<V> {
    fn remove(&mut self, key: &str) -> Option<V> {
        let index = hash(key, self.capacity());
        let bucket = &mut self.buckets[index];

        let pos = bucket.iter().position(|entry| entry.0 == key);
        if let Some(pos) = pos {
            self.len -= 1;
            Some(bucket.swap_remove(pos).1)
        } else {
            None
        }
    }
}
```

We use `swap_remove` instead of `remove` -- it swaps the target with the last element and pops, avoiding the O(n) shift within the bucket. Since bucket order doesn't matter, this is safe and fast.

## The Payoff

Let's solve the morning rush:

```rust
#[derive(Debug, Clone)]
struct Athlete {
    email: String,
    name: String,
    password_hash: String,
    box_name: String,
}

fn main() {
    // Build the locker room once at startup
    let mut athlete_map = HashMap::new();

    for i in 0..500 {
        let email = format!("athlete{}@grindit.com", i);
        let athlete = Athlete {
            email: email.clone(),
            name: format!("Athlete {}", i),
            password_hash: format!("hash_{}", i),
            box_name: "CrossFit Downtown".to_string(),
        };
        athlete_map.insert(email, athlete);
    }

    // Morning rush: 500 logins
    let mut found = 0;
    for i in 0..500 {
        let email = format!("athlete{}@grindit.com", i);
        if athlete_map.get(&email).is_some() {
            found += 1;
        }
    }
    println!("Authenticated {} athletes", found);
    println!("Map size: {} entries in {} buckets", athlete_map.len, athlete_map.capacity());
    println!("Load factor: {:.2}", athlete_map.load_factor());

    // Each lookup: hash computation + scan ~1-2 entries in bucket
    // Total: ~500-1000 comparisons vs 125,000 with linear scan
}
```

From 125,000 comparisons to roughly 500. The coach stops complaining. The morning rush feels instant.

## Complexity Comparison

| Operation | Linear scan (`Vec`) | HashMap |
|-----------|-------------------|---------|
| Lookup | O(n) | O(1) amortized |
| Insert | O(1) (append) or O(n) (sorted) | O(1) amortized |
| Delete | O(n) | O(1) amortized |
| Build from n items | O(n) | O(n) |
| Memory overhead | Low | Moderate (bucket array + pointers) |
| Worst case (all collisions) | O(n) | O(n) |

The "amortized" qualifier matters. Individual operations *might* trigger a resize (O(n)), but across many operations, the average cost per operation stays O(1). The worst case -- every key hashing to the same bucket -- is theoretically possible but practically nonexistent with a decent hash function.

## Try It Yourself

1. **Collision counter.** Add a method `collision_stats(&self) -> (usize, usize, usize)` that returns (empty buckets, max bucket length, total collisions). Insert 1000 athlete emails and print the stats. How evenly distributed are your buckets?

2. **Contains and length.** Implement `contains_key(&self, key: &str) -> bool` and `len(&self) -> usize`. Then implement `keys(&self) -> Vec<&str>` that returns all keys in the map. What order do they come out in? (Spoiler: not insertion order. Think about why.)

3. **The better hash.** Replace the hash function with the DJB2 algorithm: start with `hash = 5381`, then for each byte, do `hash = hash * 33 + byte`. Compare collision stats between the two hash functions on 1000 athlete emails. Which distributes more evenly?
