# Trie: Teaching Your Search Bar to Remember

## The Problem

Your GrindIt search bar works. The user types "back" and you filter the exercise list:

```rust,ignore
fn search_exercises(exercises: &[String], query: &str) -> Vec<String> {
    exercises
        .iter()
        .filter(|name| name.to_lowercase().starts_with(&query.to_lowercase()))
        .cloned()
        .collect()
}
```

Clean. Readable. And slow.

Every single keystroke triggers a scan of *all 500 exercises*. The user types "B" -- 500 comparisons. Types "Ba" -- 500 more. "Bac" -- another 500. By the time they've typed "Back Squat", you've done 5,000+ string comparisons. On each one, `starts_with` compares character by character, so you're also doing up to `m` character comparisons per exercise (where `m` is the query length).

On a desktop, nobody notices. On a phone over a slow connection with server-side rendering, the user sees the search bar stutter. Your coach texts you: "search feels laggy."

## The Naive Way

Let's be concrete about the cost. With `n` exercises and a query of length `m`:

```rust,ignore
// For every keystroke:
// - Loop through all n exercises: O(n)
// - Each starts_with checks up to m characters: O(m)
// Total per keystroke: O(n * m)
// Total for a 10-character query: O(10 * n * m)

fn main() {
    let exercises = vec![
        "Back Squat".to_string(),
        "Bench Press".to_string(),
        "Barbell Row".to_string(),
        "Box Jump".to_string(),
        "Burpee".to_string(),
        "Deadlift".to_string(),
        "Dumbbell Curl".to_string(),
        // ... imagine 500 of these
    ];

    // User types "B", "Ba", "Bac", "Back", "Back " ...
    // Each call scans ALL exercises from scratch
    let results = search_exercises(&exercises, "back");
    println!("Found: {:?}", results);
}

fn search_exercises(exercises: &[String], query: &str) -> Vec<String> {
    exercises
        .iter()
        .filter(|name| name.to_lowercase().starts_with(&query.to_lowercase()))
        .cloned()
        .collect()
}
```

The brutal part: when the user types "Ba", they've already typed "B". You *already know* which exercises start with "B". But you throw that knowledge away and scan everything again.

## The Insight

What if your search bar had *memory*? Like flipping through a dictionary -- once you're in the B section, you never look at A again. Once you're in "Ba", you only look at words starting with "Ba".

That's a **Trie** (pronounced "try", from re*trie*val). It's a tree where each node represents a character, and paths from root to leaf spell out words. When the user types a prefix, you walk down the tree, and everything below that node is a valid completion.

Here's what our exercise names look like as a trie:

```
        (root)
        /    \
       b      d
      / \      \
     a   o    e(a)
    / \   \      \
   c   r   x     d(lift)
   |   |   |
   k   b   j
   |   |   |
   s   e   u
   |   l   m
   q   l   p
   u   |
   a   r
   t   o
       w
```

When the user types "ba", you walk root -> b -> a. Everything below that node ("back squat", "barbell row") is a match. You never even *look* at "deadlift" or "box jump".

## The Build

Let's build it piece by piece.

### The Node

Each node holds a map of children (character -> child node) and a flag marking whether this node completes a word:

```rust
use std::collections::HashMap;

#[derive(Default, Debug)]
struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end: bool,
}
```

### The Trie

```rust
#[derive(Default, Debug)]
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
        let mut current = &mut self.root;
        for ch in word.to_lowercase().chars() {
            current = current.children.entry(ch).or_default();
        }
        current.is_end = true;
    }

    fn search(&self, word: &str) -> bool {
        let mut current = &self.root;
        for ch in word.to_lowercase().chars() {
            match current.children.get(&ch) {
                Some(node) => current = node,
                None => return false,
            }
        }
        current.is_end
    }

    fn starts_with(&self, prefix: &str) -> bool {
        let mut current = &self.root;
        for ch in prefix.to_lowercase().chars() {
            match current.children.get(&ch) {
                Some(node) => current = node,
                None => return false,
            }
        }
        true
    }
}
```

Walk down the tree character by character. If you hit a dead end, the word (or prefix) isn't there. That's it. Each operation touches exactly `m` nodes, where `m` is the length of the input -- regardless of how many exercises are stored.

### Autocomplete: The Real Prize

The search bar doesn't just need to check "does this prefix exist?" -- it needs to *suggest completions*. That means collecting every word below a given node:

```rust
impl Trie {
    fn autocomplete(&self, prefix: &str) -> Vec<String> {
        let mut current = &self.root;
        let lower_prefix = prefix.to_lowercase();

        // Walk to the prefix node
        for ch in lower_prefix.chars() {
            match current.children.get(&ch) {
                Some(node) => current = node,
                None => return vec![],
            }
        }

        // Collect all words below this node
        let mut results = Vec::new();
        self.collect_words(current, &lower_prefix, &mut results);
        results
    }

    fn collect_words(&self, node: &TrieNode, prefix: &str, results: &mut Vec<String>) {
        if node.is_end {
            results.push(prefix.to_string());
        }
        // Sort keys so results come out alphabetically
        let mut keys: Vec<&char> = node.children.keys().collect();
        keys.sort();
        for ch in keys {
            let child = &node.children[ch];
            let mut new_prefix = prefix.to_string();
            new_prefix.push(*ch);
            self.collect_words(child, &new_prefix, results);
        }
    }
}
```

## The Payoff

Let's wire it all together and see the difference:

```rust
fn main() {
    let exercise_names = vec![
        "Back Squat", "Bench Press", "Barbell Row", "Box Jump",
        "Burpee", "Deadlift", "Dumbbell Curl", "Dumbbell Press",
        "Front Squat", "Goblet Squat", "Hip Thrust", "Kettlebell Swing",
        "Lateral Raise", "Leg Press", "Lunge", "Overhead Press",
        "Pull Up", "Push Up", "Romanian Deadlift", "Thruster",
    ];

    // Build the trie once (O(n * k) where k = avg word length)
    let mut trie = Trie::new();
    for name in &exercise_names {
        trie.insert(name);
    }

    // Now each keystroke is O(m) -- length of query only
    println!("Prefix 'b':    {:?}", trie.autocomplete("b"));
    println!("Prefix 'ba':   {:?}", trie.autocomplete("ba"));
    println!("Prefix 'du':   {:?}", trie.autocomplete("du"));
    println!("Prefix 'pull': {:?}", trie.autocomplete("pull"));
    println!("Prefix 'z':    {:?}", trie.autocomplete("z"));

    // Exact search
    println!("\nIs 'deadlift' an exercise? {}", trie.search("deadlift"));
    println!("Is 'dead' an exercise?     {}", trie.search("dead"));
    println!("Does 'dead' prefix exist?  {}", trie.starts_with("dead"));
}
```

Output:

```
Prefix 'b':    ["back squat", "barbell row", "bench press", "box jump", "burpee"]
Prefix 'ba':   ["back squat", "barbell row"]
Prefix 'du':   ["dumbbell curl", "dumbbell press"]
Prefix 'pull': ["pull up"]
Prefix 'z':    []

Is 'deadlift' an exercise? true
Is 'dead' an exercise?     false
Does 'dead' prefix exist?  true
```

Notice what happened with prefix "ba": we went from scanning 20 exercises to visiting exactly 2 characters in the trie, then collecting the 2 matches below. With 500 exercises, the trie still visits just 2 characters. The naive approach still scans all 500.

## Complexity Comparison

| Operation | Naive (linear scan) | Trie |
|-----------|-------------------|------|
| Build | -- | O(n * k) one-time cost |
| Search exact | O(n * m) | O(m) |
| Prefix check | O(n * m) | O(m) |
| Autocomplete | O(n * m) | O(m + results) |
| Memory | O(n * k) | O(n * k) (more overhead per char) |

*n = number of exercises, m = query length, k = average exercise name length*

The trade-off is memory. Each character in the trie is a `HashMap` entry with its own allocation overhead. For 500 exercise names, that's fine. For millions of entries, you'd look at compressed tries or other structures. But for a search bar with hundreds of items? A trie is the perfect fit.

## Try It Yourself

1. **Count it.** Add a `count_words(&self) -> usize` method that returns how many total words are stored in the trie. (Hint: recursively count nodes where `is_end` is true.)

2. **Delete it.** Implement `remove(&mut self, word: &str) -> bool` that removes a word from the trie. Be careful -- you should only remove nodes that aren't prefixes of other words. "Deadlift" and "Dead hang" share the prefix "dead"; removing "dead hang" shouldn't break "deadlift".

3. **Rank it.** Modify the trie so each word has an associated `popularity: u32` score (how often athletes search for it). Change `autocomplete` to return results sorted by popularity descending, so "Back Squat" (searched 847 times) appears before "Barbell Row" (searched 203 times).
