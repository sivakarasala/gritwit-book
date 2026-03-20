// Problem 2: Exercise Autocomplete — Trie (Prefix Tree)
// Prefix-based exercise name search.
// Run with: cargo run --bin exercise_autocomplete

use std::collections::HashMap;

fn autocomplete_brute<'a>(exercises: &[&'a str], prefix: &str) -> Vec<String> {
    let prefix_lower = prefix.to_lowercase();
    let mut results: Vec<String> = exercises
        .iter()
        .filter(|e| e.to_lowercase().starts_with(&prefix_lower))
        .map(|e| e.to_string())
        .collect();
    results.sort();
    results
}

#[derive(Default)]
struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end: bool,
}

struct Trie {
    root: TrieNode,
}

impl Trie {
    fn new() -> Self {
        Trie {
            root: TrieNode::default(),
        }
    }

    fn insert(&mut self, word: &str) {
        let mut node = &mut self.root;
        for ch in word.to_lowercase().chars() {
            node = node.children.entry(ch).or_default();
        }
        node.is_end = true;
    }

    fn autocomplete(&self, prefix: &str) -> Vec<String> {
        let prefix_lower = prefix.to_lowercase();
        let mut node = &self.root;

        for ch in prefix_lower.chars() {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return vec![],
            }
        }

        let mut results = Vec::new();
        let mut current_word = prefix_lower.clone();
        Self::collect(node, &mut current_word, &mut results);
        results.sort();
        results
    }

    fn collect(node: &TrieNode, current: &mut String, results: &mut Vec<String>) {
        if node.is_end {
            results.push(current.clone());
        }
        for (&ch, child) in &node.children {
            current.push(ch);
            Self::collect(child, current, results);
            current.pop();
        }
    }
}

fn main() {
    let exercises = [
        "Back Squat",
        "Bench Press",
        "Box Jump",
        "Burpee",
        "Bulgarian Split Squat",
        "Clean and Jerk",
        "Deadlift",
    ];

    // Brute force
    println!("=== Brute Force ===");
    let results = autocomplete_brute(&exercises, "Bu");
    println!("prefix 'Bu': {:?}", results);

    // Trie
    println!("\n=== Trie ===");
    let mut trie = Trie::new();
    for name in &exercises {
        trie.insert(name);
    }

    for prefix in &["B", "Bu", "Bac", "Z", "De"] {
        let results = trie.autocomplete(prefix);
        println!("prefix '{}': {:?}", prefix, results);
    }
}
