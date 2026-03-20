# The WOD Is a Tree (You Just Haven't Seen It Yet)

## The Problem: Flat Data, Nested Pain

You've been building GrindIt's WOD (Workout of the Day) programming page, and it's starting to hurt. A WOD has sections — "Warm-Up," "Strength," "Metcon" — and each section has movements like "Back Squat 5x5" or "21-15-9 Thrusters & Pull-ups." Right now, you're storing everything in flat `Vec`s with parent IDs:

```rust
struct WodSection {
    id: usize,
    name: String,
}

struct WodMovement {
    id: usize,
    section_id: usize,  // which section owns me?
    name: String,
    reps: String,
}

struct Wod {
    name: String,
    sections: Vec<WodSection>,
    movements: Vec<WodMovement>,
}
```

Want to render the WOD? Buckle up:

```rust
fn render_wod(wod: &Wod) {
    println!("=== {} ===", wod.name);
    for section in &wod.sections {
        println!("\n[{}]", section.name);
        // Hunt through ALL movements for ones belonging to this section
        for movement in &wod.movements {
            if movement.section_id == section.id {
                println!("  {} - {}", movement.name, movement.reps);
            }
        }
    }
}
```

For every section, you scan *every* movement. That's O(sections x movements). With 5 sections and 30 movements, you're doing 150 comparisons to render something that should be trivial. Worse, adding a sub-section (like "Part A" inside "Metcon") means adding *another* level of IDs and another nested scan. Your rendering code becomes a Russian nesting doll of `for` loops and `if` checks.

## The Insight: A WOD IS a Tree

Step back and look at a whiteboard WOD:

```
"Murph"
├── Warm-Up
│   ├── 400m Jog
│   └── Dynamic Stretches
├── The Workout
│   ├── 1 Mile Run
│   ├── 100 Pull-ups
│   ├── 200 Push-ups
│   ├── 300 Squats
│   └── 1 Mile Run
└── Cool-Down
    └── 5 Min Stretch
```

That's a tree. The WOD is the root. Sections are children. Movements are children of sections. If you had sub-sections, they'd just be another level. The structure *is* the data — you've just been flattening it into tables because that's what databases do. But in memory? Let it be what it is.

## The Build: A Generic N-ary Tree

An N-ary tree is a tree where each node can have *any number* of children (not just two like a binary tree). Let's build one:

```rust
#[derive(Debug, Clone)]
struct TreeNode<T> {
    value: T,
    children: Vec<TreeNode<T>>,
}

impl<T> TreeNode<T> {
    fn new(value: T) -> Self {
        TreeNode {
            value,
            children: Vec::new(),
        }
    }

    fn add_child(&mut self, child: TreeNode<T>) {
        self.children.push(child);
    }

    fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}
```

That's it for the node. Now the tree itself, with traversals:

```rust
#[derive(Debug)]
struct Tree<T> {
    root: Option<TreeNode<T>>,
}

impl<T: std::fmt::Debug> Tree<T> {
    fn new() -> Self {
        Tree { root: None }
    }

    fn from_root(node: TreeNode<T>) -> Self {
        Tree { root: Some(node) }
    }

    /// DFS Pre-order: visit node BEFORE its children.
    /// Perfect for rendering a WOD top-down.
    fn dfs_preorder(&self) -> Vec<&T> {
        let mut result = Vec::new();
        if let Some(ref root) = self.root {
            Self::dfs_pre_helper(root, &mut result);
        }
        result
    }

    fn dfs_pre_helper<'a>(node: &'a TreeNode<T>, result: &mut Vec<&'a T>) {
        result.push(&node.value);
        for child in &node.children {
            Self::dfs_pre_helper(child, result);
        }
    }

    /// DFS Post-order: visit node AFTER its children.
    /// Useful for calculating totals bottom-up (total reps per section).
    fn dfs_postorder(&self) -> Vec<&T> {
        let mut result = Vec::new();
        if let Some(ref root) = self.root {
            Self::dfs_post_helper(root, &mut result);
        }
        result
    }

    fn dfs_post_helper<'a>(node: &'a TreeNode<T>, result: &mut Vec<&'a T>) {
        for child in &node.children {
            Self::dfs_post_helper(child, result);
        }
        result.push(&node.value);
    }

    /// BFS Level-order: visit all nodes at depth 0, then depth 1, etc.
    /// Perfect for a "summary bar" showing just sections.
    fn bfs(&self) -> Vec<&T> {
        let mut result = Vec::new();
        let mut queue = std::collections::VecDeque::new();
        if let Some(ref root) = self.root {
            queue.push_back(root);
        }
        while let Some(node) = queue.pop_front() {
            result.push(&node.value);
            for child in &node.children {
                queue.push_back(child);
            }
        }
        result
    }

    /// Get only nodes at a specific depth.
    /// Depth 0 = root, depth 1 = sections, depth 2 = movements.
    fn nodes_at_depth(&self, target: usize) -> Vec<&T> {
        let mut result = Vec::new();
        if let Some(ref root) = self.root {
            Self::depth_helper(root, 0, target, &mut result);
        }
        result
    }

    fn depth_helper<'a>(
        node: &'a TreeNode<T>,
        current: usize,
        target: usize,
        result: &mut Vec<&'a T>,
    ) {
        if current == target {
            result.push(&node.value);
            return; // no need to go deeper
        }
        for child in &node.children {
            Self::depth_helper(child, current + 1, target, result);
        }
    }

    fn count(&self) -> usize {
        match &self.root {
            None => 0,
            Some(root) => Self::count_helper(root),
        }
    }

    fn count_helper(node: &TreeNode<T>) -> usize {
        1 + node.children.iter().map(Self::count_helper).sum::<usize>()
    }

    fn depth(&self) -> usize {
        match &self.root {
            None => 0,
            Some(root) => Self::depth_calc(root),
        }
    }

    fn depth_calc(node: &TreeNode<T>) -> usize {
        if node.children.is_empty() {
            1
        } else {
            1 + node.children.iter().map(Self::depth_calc).max().unwrap_or(0)
        }
    }
}
```

## The Payoff: Rendering Is Now a Walk in the Park

Let's model "Murph" as a tree:

```rust
fn main() {
    // Each node holds a simple label for now
    let mut root = TreeNode::new("Murph".to_string());

    let mut warmup = TreeNode::new("Warm-Up".to_string());
    warmup.add_child(TreeNode::new("400m Jog".to_string()));
    warmup.add_child(TreeNode::new("Dynamic Stretches".to_string()));

    let mut workout = TreeNode::new("The Workout".to_string());
    workout.add_child(TreeNode::new("1 Mile Run".to_string()));
    workout.add_child(TreeNode::new("100 Pull-ups".to_string()));
    workout.add_child(TreeNode::new("200 Push-ups".to_string()));
    workout.add_child(TreeNode::new("300 Squats".to_string()));
    workout.add_child(TreeNode::new("1 Mile Run".to_string()));

    let mut cooldown = TreeNode::new("Cool-Down".to_string());
    cooldown.add_child(TreeNode::new("5 Min Stretch".to_string()));

    root.add_child(warmup);
    root.add_child(workout);
    root.add_child(cooldown);

    let wod = Tree::from_root(root);

    // Full WOD preview — DFS pre-order gives us the natural reading order
    println!("--- Full WOD (DFS Pre-order) ---");
    for item in wod.dfs_preorder() {
        println!("  {}", item);
    }

    // Summary bar — BFS depth 1 gives us just the sections
    println!("\n--- Section Tabs (BFS Depth 1) ---");
    for section in wod.nodes_at_depth(1) {
        print!(" [{}] ", section);
    }
    println!();

    // Stats
    println!("\nTotal nodes: {}", wod.count());
    println!("Tree depth: {}", wod.depth());

    // Post-order: movements first, then sections, then root
    // Useful for "calculate totals bottom-up"
    println!("\n--- Post-order (bottom-up) ---");
    for item in wod.dfs_postorder() {
        println!("  {}", item);
    }
}
```

Output:
```
--- Full WOD (DFS Pre-order) ---
  Murph
  Warm-Up
  400m Jog
  Dynamic Stretches
  The Workout
  1 Mile Run
  100 Pull-ups
  200 Push-ups
  300 Squats
  1 Mile Run
  Cool-Down
  5 Min Stretch

--- Section Tabs (BFS Depth 1) ---
 [Warm-Up]  [The Workout]  [Cool-Down]

Total nodes: 11
Tree depth: 3

--- Post-order (bottom-up) ---
  400m Jog
  Dynamic Stretches
  Warm-Up
  ...
  5 Min Stretch
  Cool-Down
  Murph
```

No more ID lookups. No more nested scans. The structure *is* the iteration order. DFS pre-order gives you the natural top-down rendering. BFS at depth 1 gives you section tabs. Post-order lets you calculate totals from leaves up.

## Complexity Comparison

| Operation | Flat Vecs + IDs | N-ary Tree |
|---|---|---|
| Render full WOD | O(S x M) nested scan | O(N) single DFS walk |
| Get sections only | O(S) | O(S) via `nodes_at_depth(1)` |
| Add sub-section | New ID layer + refactor | `section.add_child(subsection)` |
| Find a movement | O(M) linear scan | O(N) DFS (same worst case, but structured) |
| Count total movements | O(M) | O(N) recursive count |

Where S = sections, M = movements, N = total nodes.

## Try It Yourself

1. **Add a `find` method** that takes a predicate `FnMut(&T) -> bool` and returns `Option<&T>` using DFS. Use it to find a movement by name.

2. **Build a `map` method** that transforms `Tree<T>` into `Tree<U>` by applying a function to every node's value. Use it to convert a `Tree<String>` of movement names into a `Tree<usize>` of estimated rep counts.

3. **Add depth-aware rendering.** Modify `dfs_preorder` to return `Vec<(usize, &T)>` where the `usize` is the depth. Use it to render the WOD with proper indentation — root at 0 spaces, sections at 2, movements at 4.
