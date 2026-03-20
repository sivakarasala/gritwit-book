// Chapter 3 DSA Exercise: Linear Search & String Matching
//
// .contains() is O(n*m) naive string search. When to optimize
// with tries, inverted indexes, or server-side search.

use std::collections::HashMap;

/// Naive string matching: O(n * m) where n = text length, m = pattern length.
/// This is what str::contains() does internally.
fn naive_search(text: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }
    let text_bytes = text.as_bytes();
    let pattern_bytes = pattern.as_bytes();
    if pattern_bytes.len() > text_bytes.len() {
        return false;
    }
    for i in 0..=(text_bytes.len() - pattern_bytes.len()) {
        if text_bytes[i..i + pattern_bytes.len()] == *pattern_bytes {
            return true;
        }
    }
    false
}

/// KMP (Knuth-Morris-Pratt) string matching: O(n + m).
/// Builds a failure function to avoid re-scanning characters.
fn kmp_search(text: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }
    let text = text.as_bytes();
    let pattern = pattern.as_bytes();
    let n = text.len();
    let m = pattern.len();

    if m > n {
        return false;
    }

    // Build failure function (partial match table)
    let mut failure = vec![0usize; m];
    let mut k = 0;
    for i in 1..m {
        while k > 0 && pattern[k] != pattern[i] {
            k = failure[k - 1];
        }
        if pattern[k] == pattern[i] {
            k += 1;
        }
        failure[i] = k;
    }

    // Search
    let mut j = 0;
    for i in 0..n {
        while j > 0 && pattern[j] != text[i] {
            j = failure[j - 1];
        }
        if pattern[j] == text[i] {
            j += 1;
        }
        if j == m {
            return true;
        }
    }
    false
}

/// A simple Trie for prefix-based search — O(m) lookup where m is pattern length.
/// Uses HashMap children to handle any character.
struct TrieNode {
    children: HashMap<u8, Box<TrieNode>>,
    is_end: bool,
    /// Store the original word for retrieval
    word: Option<String>,
}

impl TrieNode {
    fn new() -> Self {
        TrieNode {
            children: HashMap::new(),
            is_end: false,
            word: None,
        }
    }
}

struct Trie {
    root: TrieNode,
}

impl Trie {
    fn new() -> Self {
        Trie {
            root: TrieNode::new(),
        }
    }

    fn insert(&mut self, word: &str) {
        let lower = word.to_lowercase();
        let mut node = &mut self.root;
        for ch in lower.bytes() {
            node = node.children.entry(ch).or_insert_with(|| Box::new(TrieNode::new()));
        }
        node.is_end = true;
        node.word = Some(word.to_string());
    }

    /// Find all words that start with the given prefix
    fn search_prefix(&self, prefix: &str) -> Vec<String> {
        let lower = prefix.to_lowercase();
        let mut node = &self.root;
        for ch in lower.bytes() {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return vec![],
            }
        }
        // Collect all words under this node
        let mut results = Vec::new();
        Self::collect_words(node, &mut results);
        results
    }

    fn collect_words(node: &TrieNode, results: &mut Vec<String>) {
        if let Some(word) = &node.word {
            results.push(word.clone());
        }
        let mut keys: Vec<&u8> = node.children.keys().collect();
        keys.sort();
        for key in keys {
            Self::collect_words(&node.children[key], results);
        }
    }
}

/// GrindIt exercise search — simulates the client-side filter
fn linear_filter(exercises: &[&str], query: &str) -> Vec<String> {
    let q = query.to_lowercase();
    exercises
        .iter()
        .filter(|name| name.to_lowercase().contains(&q))
        .map(|s| s.to_string())
        .collect()
}

fn main() {
    let exercises = vec![
        "Back Squat",
        "Front Squat",
        "Overhead Squat",
        "Deadlift",
        "Sumo Deadlift",
        "Pull-Up",
        "Chest-to-Bar Pull-Up",
        "Muscle-Up",
        "Box Jump",
        "Bench Press",
        "Shoulder Press",
        "Push Press",
        "Thruster",
        "Wall Ball",
    ];

    // 1. Linear search with .contains() — what GrindIt uses
    println!("=== Linear Search (.contains) ===");
    let query = "squat";
    let results = linear_filter(&exercises, query);
    println!("Query '{}': {:?}", query, results);
    println!("Time complexity: O(n * m) where n=text_len, m=pattern_len");
    println!("For {} exercises, this is fast enough.\n", exercises.len());

    // 2. Naive vs KMP search comparison
    println!("=== Naive vs KMP String Matching ===");
    let text = "chest-to-bar pull-up";
    let pattern = "pull";
    println!(
        "Naive search '{}' in '{}': {}",
        pattern,
        text,
        naive_search(text, pattern)
    );
    println!(
        "KMP search '{}' in '{}': {}",
        pattern,
        text,
        kmp_search(text, pattern)
    );
    println!("Naive: O(n*m) worst case. KMP: O(n+m) guaranteed.\n");

    // 3. Trie for prefix search — autocomplete pattern
    println!("=== Trie Prefix Search (Autocomplete) ===");
    let mut trie = Trie::new();
    for ex in &exercises {
        // Insert each word of the exercise name for word-level prefix search
        trie.insert(ex);
    }

    let prefix = "Back";
    let trie_results = trie.search_prefix(prefix);
    println!("Prefix '{}': {:?}", prefix, trie_results);

    let prefix = "Push";
    let trie_results = trie.search_prefix(prefix);
    println!("Prefix '{}': {:?}", prefix, trie_results);

    let prefix = "Over";
    let trie_results = trie.search_prefix(prefix);
    println!("Prefix '{}': {:?}", prefix, trie_results);

    println!("\nTrie lookup: O(m) where m = prefix length, regardless of exercise count.");

    // 4. Scaling analysis
    println!("\n=== Scaling Analysis ===");
    println!("| Exercises | Pattern len | Naive comparisons (worst) |");
    println!("|-----------|-------------|---------------------------|");
    for (n, m) in [(14, 5), (100, 10), (1000, 10), (10000, 20)] {
        println!("|  {:>7}  |     {:>5}   |          {:>12}       |", n, m, n * m * 30);
    }
    println!("\nFor <200 exercises: .contains() is fine (client-side).");
    println!("For 1000+: move search to server (PostgreSQL ILIKE or full-text search).");
    println!("For autocomplete: use a Trie for O(prefix_len) lookups.");
}
