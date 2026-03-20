// Chapter 5 DSA Exercise: B-Tree Indexes
//
// Simulates how PostgreSQL B-tree indexes work.
// A B-tree is a self-balancing tree where each node holds multiple keys,
// providing O(log n) lookup with excellent cache locality.

/// A simplified B-tree node for demonstration.
/// Real PostgreSQL B-tree nodes are 8KB pages with hundreds of keys.
#[derive(Debug)]
struct BTreeNode {
    keys: Vec<String>,
    children: Vec<BTreeNode>,
    is_leaf: bool,
}

/// Build a two-level B-tree manually from sorted data.
/// This demonstrates the structure without the complexity of insert/split.
fn build_demo_btree(sorted_keys: &[&str]) -> BTreeNode {
    // Split into groups of 3 for leaf nodes
    let chunk_size = 3;
    let mut leaves = Vec::new();
    let mut separator_keys = Vec::new();

    let mut i = 0;
    while i < sorted_keys.len() {
        let end = (i + chunk_size).min(sorted_keys.len());
        let leaf_keys: Vec<String> = sorted_keys[i..end]
            .iter()
            .map(|s| s.to_string())
            .collect();
        if !leaves.is_empty() {
            // The first key of each non-first leaf becomes a separator
            separator_keys.push(leaf_keys[0].clone());
        }
        leaves.push(BTreeNode {
            keys: leaf_keys,
            children: Vec::new(),
            is_leaf: true,
        });
        i = end;
    }

    if leaves.len() <= 1 {
        return leaves.into_iter().next().unwrap_or(BTreeNode {
            keys: Vec::new(),
            children: Vec::new(),
            is_leaf: true,
        });
    }

    // Create root node with separators pointing to leaves
    BTreeNode {
        keys: separator_keys,
        children: leaves,
        is_leaf: false,
    }
}

/// Search the B-tree, counting comparisons
fn btree_search(node: &BTreeNode, key: &str) -> (bool, usize) {
    let mut comparisons = 0;
    let mut current = node;

    loop {
        // Search within this node's keys
        let mut child_idx = current.keys.len();
        for (i, node_key) in current.keys.iter().enumerate() {
            comparisons += 1;
            match key.cmp(node_key.as_str()) {
                std::cmp::Ordering::Equal => return (true, comparisons),
                std::cmp::Ordering::Less => {
                    child_idx = i;
                    break;
                }
                std::cmp::Ordering::Greater => {}
            }
        }

        if current.is_leaf {
            return (false, comparisons);
        }

        if child_idx < current.children.len() {
            current = &current.children[child_idx];
        } else {
            return (false, comparisons);
        }
    }
}

/// Print the tree structure for visualization
fn print_btree(node: &BTreeNode, depth: usize) {
    let indent = "  ".repeat(depth + 1);
    let node_type = if node.is_leaf { "leaf" } else { "node" };
    println!("{}[{}] {:?}", indent, node_type, node.keys);
    for child in &node.children {
        print_btree(child, depth + 1);
    }
}

/// Linear scan — what happens WITHOUT an index
fn linear_scan(exercises: &[&str], target: &str) -> (bool, usize) {
    let target_lower = target.to_lowercase();
    let mut comparisons = 0;
    for name in exercises {
        comparisons += 1;
        if name.to_lowercase() == target_lower {
            return (true, comparisons);
        }
    }
    (false, comparisons)
}

/// Binary search on a sorted array — same O(log n) as B-tree
fn binary_search_count(sorted: &[&str], target: &str) -> (bool, usize) {
    let mut comparisons = 0;
    let mut lo = 0usize;
    let mut hi = sorted.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        comparisons += 1;
        match target.cmp(sorted[mid]) {
            std::cmp::Ordering::Equal => return (true, comparisons),
            std::cmp::Ordering::Less => hi = mid,
            std::cmp::Ordering::Greater => lo = mid + 1,
        }
    }
    (false, comparisons)
}

fn main() {
    let exercises = vec![
        "Back Squat", "Bench Press", "Box Jump", "Burpee",
        "Clean", "Deadlift", "Front Squat", "Handstand Push-Up",
        "Kettlebell Swing", "Muscle-Up", "Overhead Squat", "Pull-Up",
        "Push Press", "Rowing", "Shoulder Press", "Snatch",
        "Thruster", "Toes-to-Bar", "Turkish Get-Up", "Wall Ball",
    ];

    println!("=== Building B-Tree Index ===");
    println!("Simulates: CREATE UNIQUE INDEX idx_exercises_name ON exercises (LOWER(name));\n");

    // Build B-tree from sorted lowercase names
    let lowercase_sorted: Vec<&str> = vec![
        "back squat", "bench press", "box jump", "burpee",
        "clean", "deadlift", "front squat", "handstand push-up",
        "kettlebell swing", "muscle-up", "overhead squat", "pull-up",
        "push press", "rowing", "shoulder press", "snatch",
        "thruster", "toes-to-bar", "turkish get-up", "wall ball",
    ];

    let btree = build_demo_btree(&lowercase_sorted);
    println!("B-Tree structure (max 3 keys per leaf):");
    print_btree(&btree, 0);

    // Compare search performance
    println!("\n=== Search Comparison: B-Tree vs Binary Search vs Linear Scan ===");
    println!(
        "{:<25} {:>10} {:>10} {:>10}",
        "Exercise", "B-Tree", "BinSearch", "Linear"
    );
    println!("{}", "-".repeat(57));

    let search_targets = vec![
        "back squat",
        "muscle-up",
        "wall ball",
        "rope climb", // not found
    ];

    for target in &search_targets {
        let (bt_found, bt_comps) = btree_search(&btree, target);
        let (bs_found, bs_comps) = binary_search_count(&lowercase_sorted, target);
        let (ln_found, ln_comps) = linear_scan(&exercises, target);

        let label = |found: bool, comps: usize| -> String {
            if found {
                format!("{} (hit)", comps)
            } else {
                format!("{} (miss)", comps)
            }
        };

        println!(
            "{:<25} {:>10} {:>10} {:>10}",
            target,
            label(bt_found, bt_comps),
            label(bs_found, bs_comps),
            label(ln_found, ln_comps),
        );
    }

    // Scaling analysis
    println!("\n=== Scaling: B-Tree O(log n) vs Linear O(n) ===");
    println!(
        "{:>12} {:>18} {:>18}",
        "Exercises", "B-Tree (max)", "Linear (max)"
    );
    println!("{}", "-".repeat(50));
    for n in [14u64, 100, 1_000, 10_000, 100_000, 1_000_000] {
        let btree_max = (n as f64).log2().ceil() as u64;
        println!("{:>12} {:>18} {:>18}", n, btree_max, n);
    }

    // B-tree anatomy
    println!("\n=== How PostgreSQL B-Tree Indexes Work ===");
    println!("  Lookup example: finding 'pull-up'");
    println!("  1. Root node: keys = [clean, handstand push-up, ...]");
    println!("     'pull-up' > 'handstand push-up' -> go to right child");
    println!("  2. Leaf node: keys = [pull-up, push press, rowing]");
    println!("     Found 'pull-up' at position 0");
    println!("  Total: 2 node accesses (vs 12 for linear scan)");

    println!("\n=== B-Tree vs Binary Search ===");
    println!("Both are O(log n), but B-trees win in practice:");
    println!("  - B-tree nodes hold multiple keys (better cache locality)");
    println!("  - PostgreSQL pages are 8KB (~hundreds of keys per node)");
    println!("  - Fewer memory accesses per level (1 page read vs many)");
    println!("  - B-trees support range queries (walk leaf chain)");

    println!("\n=== Key Insights ===");
    println!("1. CREATE INDEX builds a B-tree for O(log n) lookups");
    println!("2. Without index: full table scan O(n)");
    println!("3. Partial index (WHERE deleted_at IS NULL) skips deleted rows");
    println!("4. Indexes speed up reads but slow writes (every INSERT updates the index)");
    println!("5. For 14 exercises the difference is negligible; for 14,000 it matters");
    println!("6. Index columns in WHERE, ORDER BY, JOIN ON, and UNIQUE constraints");
}
