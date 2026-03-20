// Chapter 14 DSA Exercise: Cache Invalidation
//
// Service worker versioning as a cache invalidation strategy.
// Simulates cache-first, network-first, and stale-while-revalidate patterns.
// "There are only two hard things in CS: cache invalidation and naming things."

use std::collections::HashMap;

// ----------------------------------------------------------------
// Part 1: Simulated versioned cache (like service worker caching)
// ----------------------------------------------------------------

#[derive(Debug, Clone)]
struct CacheEntry {
    data: String,
    version: u32,
    timestamp: u64,
}

struct VersionedCache {
    name: String,
    version: u32,
    entries: HashMap<String, CacheEntry>,
}

impl VersionedCache {
    fn new(name: &str, version: u32) -> Self {
        VersionedCache {
            name: format!("{}-v{}", name, version),
            version,
            entries: HashMap::new(),
        }
    }

    fn put(&mut self, key: &str, data: &str, timestamp: u64) {
        self.entries.insert(
            key.to_string(),
            CacheEntry {
                data: data.to_string(),
                version: self.version,
                timestamp,
            },
        );
    }

    fn get(&self, key: &str) -> Option<&CacheEntry> {
        self.entries.get(key)
    }

    fn remove(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    fn clear(&mut self) -> usize {
        let count = self.entries.len();
        self.entries.clear();
        count
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

// ----------------------------------------------------------------
// Part 2: Cache manager with version-based invalidation
// ----------------------------------------------------------------

struct CacheManager {
    caches: Vec<VersionedCache>,
    current_version: u32,
    cache_prefix: String,
}

impl CacheManager {
    fn new(prefix: &str, version: u32) -> Self {
        let mut manager = CacheManager {
            caches: Vec::new(),
            current_version: version,
            cache_prefix: prefix.to_string(),
        };
        manager
            .caches
            .push(VersionedCache::new(prefix, version));
        manager
    }

    fn current_cache(&mut self) -> &mut VersionedCache {
        let version = self.current_version;
        self.caches
            .iter_mut()
            .find(|c| c.version == version)
            .expect("Current cache should exist")
    }

    /// Simulate deploying a new version — creates new cache and purges old ones
    fn activate_new_version(&mut self, new_version: u32) -> Vec<String> {
        let old_version = self.current_version;
        self.current_version = new_version;
        self.caches
            .push(VersionedCache::new(&self.cache_prefix, new_version));

        // Purge old caches (like the service worker activate event)
        let mut purged = Vec::new();
        self.caches.retain(|cache| {
            if cache.version != new_version {
                purged.push(format!(
                    "{} ({} entries purged)",
                    cache.name,
                    cache.entries.len()
                ));
                false
            } else {
                true
            }
        });
        purged
    }
}

// ----------------------------------------------------------------
// Part 3: Caching strategies simulation
// ----------------------------------------------------------------

/// Simulated network response
struct NetworkResponse {
    data: String,
    ok: bool,
    latency_ms: u32,
}

fn simulate_network(url: &str, is_online: bool) -> NetworkResponse {
    if is_online {
        NetworkResponse {
            data: format!("[fresh] Content for {}", url),
            ok: true,
            latency_ms: 200,
        }
    } else {
        NetworkResponse {
            data: String::new(),
            ok: false,
            latency_ms: 5000,
        }
    }
}

/// Cache-first: check cache, fall back to network
fn cache_first(
    cache: &mut VersionedCache,
    url: &str,
    is_online: bool,
    timestamp: u64,
) -> (String, &'static str) {
    if let Some(entry) = cache.get(url) {
        return (entry.data.clone(), "cache-hit");
    }

    let response = simulate_network(url, is_online);
    if response.ok {
        cache.put(url, &response.data, timestamp);
        (response.data, "network-fetched")
    } else {
        ("Error: offline and no cache".to_string(), "error")
    }
}

/// Network-first: try network, fall back to cache
fn network_first(
    cache: &mut VersionedCache,
    url: &str,
    is_online: bool,
    timestamp: u64,
) -> (String, &'static str) {
    let response = simulate_network(url, is_online);
    if response.ok {
        cache.put(url, &response.data, timestamp);
        return (response.data, "network-fresh");
    }

    if let Some(entry) = cache.get(url) {
        (entry.data.clone(), "cache-fallback")
    } else {
        ("Error: offline and no cache".to_string(), "error")
    }
}

/// Stale-while-revalidate: return cache immediately, update in background
fn stale_while_revalidate(
    cache: &mut VersionedCache,
    url: &str,
    is_online: bool,
    timestamp: u64,
) -> (String, &'static str) {
    let cached = cache.get(url).map(|e| e.data.clone());

    // "Background" revalidation
    let response = simulate_network(url, is_online);
    if response.ok {
        cache.put(url, &response.data, timestamp);
    }

    if let Some(data) = cached {
        (data, "stale-served-fresh-updating")
    } else if response.ok {
        (
            cache.get(url).unwrap().data.clone(),
            "network-fetched-no-stale",
        )
    } else {
        ("Error: offline and no cache".to_string(), "error")
    }
}

// ----------------------------------------------------------------
// Part 4: LRU Cache (interview classic)
// ----------------------------------------------------------------

struct LruCache {
    capacity: usize,
    entries: Vec<(String, String)>, // (key, value) — most recent at end
}

impl LruCache {
    fn new(capacity: usize) -> Self {
        LruCache {
            capacity,
            entries: Vec::new(),
        }
    }

    fn get(&mut self, key: &str) -> Option<String> {
        if let Some(pos) = self.entries.iter().position(|(k, _)| k == key) {
            let entry = self.entries.remove(pos);
            let value = entry.1.clone();
            self.entries.push(entry);
            Some(value)
        } else {
            None
        }
    }

    fn put(&mut self, key: &str, value: &str) {
        // Remove existing entry if present
        if let Some(pos) = self.entries.iter().position(|(k, _)| k == key) {
            self.entries.remove(pos);
        }
        // Evict oldest if at capacity
        if self.entries.len() >= self.capacity {
            let evicted = self.entries.remove(0);
            println!("      [LRU evicted: '{}']", evicted.0);
        }
        self.entries.push((key.to_string(), value.to_string()));
    }

    fn contents(&self) -> Vec<&str> {
        self.entries.iter().map(|(k, _)| k.as_str()).collect()
    }
}

fn main() {
    println!("=== Cache Invalidation ===\n");

    // Part 1: Versioned cache
    println!("--- Part 1: Versioned Cache (Service Worker Pattern) ---");
    let mut manager = CacheManager::new("grindit-static", 5);

    {
        let cache = manager.current_cache();
        cache.put("/app.js", "console.log('v5')", 1000);
        cache.put("/style.css", "body { color: #333 }", 1000);
        cache.put("/manifest.json", "{\"name\": \"GrindIt\"}", 1000);
        println!(
            "  Cache '{}': {} entries",
            cache.name,
            cache.len()
        );
    }

    // Deploy new version
    println!("\n  Deploying version 6...");
    let purged = manager.activate_new_version(6);
    for p in &purged {
        println!("    Purged: {}", p);
    }

    {
        let cache = manager.current_cache();
        println!(
            "  New cache '{}': {} entries (fresh start)",
            cache.name,
            cache.len()
        );
    }

    // Part 2: Caching strategies
    println!("\n--- Part 2: Caching Strategies ---");

    let urls = ["/exercises", "/app.js", "/api/v1/wods"];
    let mut cache = VersionedCache::new("test", 1);

    // Pre-populate cache with stale data
    for url in &urls {
        cache.put(url, &format!("[stale] Old content for {}", url), 500);
    }

    println!("\n  Cache-First (static assets like fonts, images):");
    for url in &urls {
        let (data, source) = cache_first(&mut cache, url, true, 1000);
        println!("    {} => [{}] {}", url, source, &data[..data.len().min(50)]);
    }

    let mut cache2 = VersionedCache::new("test", 1);
    for url in &urls {
        cache2.put(url, &format!("[stale] Old content for {}", url), 500);
    }

    println!("\n  Network-First (HTML pages, API calls):");
    for (i, url) in urls.iter().enumerate() {
        let is_online = i != 2; // third request fails (simulate offline)
        let (data, source) = network_first(&mut cache2, url, is_online, 1000);
        let online_label = if is_online { "online" } else { "OFFLINE" };
        println!(
            "    {} [{}] => [{}] {}",
            url,
            online_label,
            source,
            &data[..data.len().min(50)]
        );
    }

    let mut cache3 = VersionedCache::new("test", 1);
    for url in &urls {
        cache3.put(url, &format!("[stale] Old content for {}", url), 500);
    }

    println!("\n  Stale-While-Revalidate (JS/CSS bundles):");
    for url in &urls {
        let (data, source) = stale_while_revalidate(&mut cache3, url, true, 1000);
        println!("    {} => [{}] {}", url, source, &data[..data.len().min(50)]);
    }

    // Strategy comparison
    println!("\n  Strategy Comparison:");
    println!(
        "  {:<28} {:<15} {:<15} {:<20}",
        "Strategy", "Latency", "Freshness", "Use Case"
    );
    println!("  {}", "-".repeat(78));
    println!(
        "  {:<28} {:<15} {:<15} {:<20}",
        "Cache-first", "Instant", "May be stale", "Fonts, images"
    );
    println!(
        "  {:<28} {:<15} {:<15} {:<20}",
        "Network-first", "Network RTT", "Always fresh", "HTML, API calls"
    );
    println!(
        "  {:<28} {:<15} {:<15} {:<20}",
        "Stale-while-revalidate", "Instant", "Fresh next load", "JS/CSS bundles"
    );
    println!(
        "  {:<28} {:<15} {:<15} {:<20}",
        "Network-only", "Network RTT", "Always fresh", "Auth, uploads"
    );

    // Part 3: LRU Cache
    println!("\n--- Part 3: LRU Cache (Interview Classic) ---");
    let mut lru = LruCache::new(3);

    let operations = vec![
        ("put", "squat", "Back Squat"),
        ("put", "deadlift", "Deadlift"),
        ("put", "press", "Bench Press"),
        ("get", "squat", ""),
        ("put", "clean", "Clean"), // evicts deadlift (least recently used)
        ("get", "deadlift", ""),   // miss
        ("put", "snatch", "Snatch"), // evicts press
    ];

    for (op, key, value) in &operations {
        match *op {
            "put" => {
                println!("    put('{}', '{}')", key, value);
                lru.put(key, value);
            }
            "get" => {
                let result = lru.get(key);
                println!("    get('{}') => {:?}", key, result);
            }
            _ => {}
        }
        println!("      cache state: {:?}", lru.contents());
    }

    println!("\n=== Key Insights ===");
    println!("1. Version-based invalidation: bump version, old caches are purged on activate");
    println!("2. Never cache auth endpoints — stale session data is a security risk");
    println!("3. Cache-first for immutable assets, network-first for dynamic content");
    println!("4. Stale-while-revalidate gives instant response + eventual freshness");
    println!("5. LRU cache: O(1) get/put with ordered eviction (use LinkedHashMap in production)");
}
