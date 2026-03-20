# Binary Search Tree: Where Every Exercise Finds Its Place

## The Problem

Your exercise library has grown to 500 exercises, stored in a sorted `Vec<Exercise>`. Lookups are fast -- binary search gives you O(log n). Life is good.

Then your coach says: "I need to add new exercises on the fly. Custom movements for my athletes."

You write the insert function and immediately hit the wall:

```rust,ignore
fn insert_sorted(exercises: &mut Vec<String>, new_exercise: String) {
    let pos = exercises.partition_point(|e| e < &new_exercise);
    exercises.insert(pos, new_exercise);
}
```

That `insert` call looks innocent. But `Vec::insert` at position `pos` has to shift every element after `pos` to the right. With 500 exercises and an insert near the beginning, that's moving ~499 strings in memory. Add 50 new exercises and you're doing thousands of shifts.

Binary search finds *where* to insert in O(log n). But the actual insert is O(n). You've got a sports car engine bolted to a bicycle frame.

## The Naive Way

Let's see the pain clearly:

```rust,ignore
fn main() {
    let mut exercises: Vec<String> = vec![
        "Back Squat", "Bench Press", "Deadlift", "Front Squat",
        "Hip Thrust", "Leg Press", "Overhead Press", "Pull Up",
    ].into_iter().map(String::from).collect();

    // Each insert is O(log n) to find position + O(n) to shift elements
    let new_exercises = vec!["Box Jump", "Clean", "Snatch", "Thruster", "Wall Ball"];

    for name in new_exercises {
        let pos = exercises.partition_point(|e| e.as_str() < name);
        exercises.insert(pos, name.to_string());
        println!("Inserted '{}' at position {} (shifted {} elements)",
            name, pos, exercises.len() - pos - 1);
    }
}
```

Output:

```
Inserted 'Box Jump' at position 2 (shifted 7 elements)
Inserted 'Clean' at position 3 (shifted 7 elements)
Inserted 'Snatch' at position 9 (shifted 2 elements)
Inserted 'Thruster' at position 10 (shifted 2 elements)
Inserted 'Wall Ball' at position 12 (shifted 0 elements)
```

Every insert near the front shifts almost everything. With 500 exercises, the average insert moves 250 elements. That's not just slow -- it's *unpredictably* slow, depending on where the new exercise falls alphabetically.

## The Insight

What if the data could organize itself as you insert? No shifting, no moving existing elements. Each new exercise just... finds its spot and sits down.

Picture a gym organized by equipment weight. There's a central rack. Everything lighter goes left, everything heavier goes right. When a new dumbbell arrives, you don't rearrange the whole floor. You start at the center, go left or right based on weight, and keep going until you find an empty spot. Done.

That's a binary search tree. And in Rust, building one teaches you something about ownership that changes how you think about the language.

## The Build

### The Node

Each node holds a value and optionally points to a left child and a right child. Here's where Rust gets interesting. A tree node *owns* its children. But a node can have zero, one, or two children. And each child is another node on the heap. We need `Option<Box<Node>>`:

```rust
#[derive(Debug)]
struct Node {
    exercise: String,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

impl Node {
    fn new(exercise: String) -> Self {
        Node {
            exercise,
            left: None,
            right: None,
        }
    }
}
```

Why `Box`? Because without it, a `Node` would contain a `Node` which contains a `Node`... infinite size. Rust needs to know a struct's size at compile time. `Box<Node>` is a pointer (8 bytes, fixed size) to a `Node` on the heap. The recursion happens in memory, not in the type's size.

Why `Option`? Because children are optional. `None` means "no child here" -- it's the leaf of the tree.

### The Tree

```rust
#[derive(Debug)]
struct ExerciseBST {
    root: Option<Box<Node>>,
}

impl ExerciseBST {
    fn new() -> Self {
        ExerciseBST { root: None }
    }
}
```

### Insert

Here's where ownership gets real. We need to walk down the tree, find the right spot, and attach a new node. The trick is using `&mut Option<Box<Node>>` -- a mutable reference to the *slot* where a node might go:

```rust
impl ExerciseBST {
    fn insert(&mut self, exercise: String) {
        Self::insert_into(&mut self.root, exercise);
    }

    fn insert_into(slot: &mut Option<Box<Node>>, exercise: String) {
        match slot {
            None => {
                *slot = Some(Box::new(Node::new(exercise)));
            }
            Some(node) => {
                if exercise < node.exercise {
                    Self::insert_into(&mut node.left, exercise);
                } else if exercise > node.exercise {
                    Self::insert_into(&mut node.right, exercise);
                }
                // If equal, it's a duplicate -- do nothing
            }
        }
    }
}
```

Read that `None` arm carefully. We have a mutable reference to an `Option` that's currently `None`. We *replace its contents* with `Some(Box::new(...))`. The parent node doesn't move. Nothing else shifts. We just filled an empty slot. That's O(log n) for a balanced tree -- no shifting, no copying.

### Search

Walking the tree to find an exercise follows the same left/right logic:

```rust
impl ExerciseBST {
    fn search(&self, exercise: &str) -> bool {
        Self::search_in(&self.root, exercise)
    }

    fn search_in(node: &Option<Box<Node>>, exercise: &str) -> bool {
        match node {
            None => false,
            Some(n) => {
                if exercise == n.exercise {
                    true
                } else if exercise < n.exercise.as_str() {
                    Self::search_in(&n.left, exercise)
                } else {
                    Self::search_in(&n.right, exercise)
                }
            }
        }
    }
}
```

### In-Order Traversal (Sorted Output)

Visit left subtree, then current node, then right subtree. You get every exercise in alphabetical order:

```rust
impl ExerciseBST {
    fn in_order(&self) -> Vec<&str> {
        let mut result = Vec::new();
        Self::collect_in_order(&self.root, &mut result);
        result
    }

    fn collect_in_order<'a>(node: &'a Option<Box<Node>>, result: &mut Vec<&'a str>) {
        if let Some(n) = node {
            Self::collect_in_order(&n.left, result);
            result.push(&n.exercise);
            Self::collect_in_order(&n.right, result);
        }
    }
}
```

### Min and Max

The smallest exercise name is all the way left. The largest is all the way right:

```rust
impl ExerciseBST {
    fn min(&self) -> Option<&str> {
        let mut current = &self.root;
        let mut result = None;
        while let Some(node) = current {
            result = Some(node.exercise.as_str());
            current = &node.left;
        }
        result
    }

    fn max(&self) -> Option<&str> {
        let mut current = &self.root;
        let mut result = None;
        while let Some(node) = current {
            result = Some(node.exercise.as_str());
            current = &node.right;
        }
        result
    }
}
```

## The Payoff

```rust
fn main() {
    let mut bst = ExerciseBST::new();

    // Insertions: O(log n) each, no shifting
    let exercises = vec![
        "Hip Thrust", "Deadlift", "Pull Up", "Back Squat",
        "Front Squat", "Overhead Press", "Bench Press", "Leg Press",
    ];

    for name in &exercises {
        bst.insert(name.to_string());
    }

    // Add new exercises on the fly -- still O(log n)
    bst.insert("Clean".to_string());
    bst.insert("Snatch".to_string());
    bst.insert("Wall Ball".to_string());

    // Search: O(log n)
    println!("Has 'Deadlift': {}", bst.search("Deadlift"));
    println!("Has 'Bicep Curl': {}", bst.search("Bicep Curl"));

    // Sorted output without ever sorting
    println!("\nAll exercises (sorted):");
    for name in bst.in_order() {
        println!("  {}", name);
    }

    println!("\nFirst: {:?}", bst.min());
    println!("Last:  {:?}", bst.max());
}
```

No shifting. No re-sorting. The tree maintains order as a structural property. Every insert, every search -- O(log n).

One caveat: this is true for *balanced* trees. If you insert exercises in already-sorted order ("A", "B", "C", "D"...), the tree degenerates into a linked list and everything becomes O(n). Real-world code uses self-balancing trees (AVL, Red-Black). But the core idea -- data that organizes itself through comparison -- starts here.

## Complexity Comparison

| Operation | Sorted Vec | BST (balanced) |
|-----------|-----------|----------------|
| Search | O(log n) | O(log n) |
| Insert | O(n) -- shift elements | O(log n) |
| Delete | O(n) -- shift elements | O(log n) |
| Sorted traversal | O(n) -- already sorted | O(n) |
| Min / Max | O(1) | O(log n) |

The BST trades O(1) min/max for O(log n) inserts. When your exercise library is mostly read, the sorted `Vec` wins. When coaches are constantly adding and removing custom exercises, the BST wins.

## Try It Yourself

1. **Count it.** Add a `size(&self) -> usize` method that returns the total number of exercises in the tree. Do it recursively: a `None` node has size 0, a `Some` node has size 1 + left size + right size.

2. **Height check.** Add a `height(&self) -> usize` method. Insert the numbers 1 through 15 in a random order vs. in sorted order. Print the height both ways. A balanced tree of 15 nodes has height 4 (log2(15)+1). A degenerate one has height 15. See the difference?

3. **Range query.** Implement `exercises_in_range(&self, from: &str, to: &str) -> Vec<&str>` that returns all exercises alphabetically between `from` and `to` (inclusive). This is where BSTs really shine -- you can prune entire subtrees that are out of range.
