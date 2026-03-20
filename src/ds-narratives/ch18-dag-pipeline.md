# One-Way Roads: Directed Acyclic Graphs for CI Pipelines

## The Problem

Your GrindIt project has grown. You've got a proper CI pipeline now — six jobs that run when you push code:

1. **Format** — `cargo fmt --check`
2. **Lint** — `cargo clippy`
3. **Test** — `cargo test`
4. **Build** — `cargo build --release`
5. **Audit** — `cargo audit` (check for security vulnerabilities)
6. **Deploy** — push to production

But they can't all run at the same time. You can't deploy before tests pass. You can't test before the code builds. Lint and format can run in parallel — they don't depend on each other. Audit can run alongside tests.

You draw the dependencies on a whiteboard:

```
  Format ──┐
            ├──> Build ──> Test ──> Deploy
  Lint ────┘                ↑
                            │
  Audit ────────────────────┘
```

Then you stare at it and realize: this is a map with one-way roads. Nodes are jobs. Arrows are "must finish before." And the question every team eventually asks at the whiteboard: **"Is there a cycle?"**

If Test depends on Build, and someone accidentally makes Build depend on Test... your CI runs forever. Or more accurately, it can never start. That's a deadlock, and detecting it is the first thing you need.

## The Naive Way

Without a proper graph, you might hard-code the order:

```rust,ignore
fn run_pipeline() {
    // Just... run them in a sequence that seems right?
    run_job("format");
    run_job("lint");
    run_job("build");
    run_job("audit");
    run_job("test");
    run_job("deploy");
}
```

This works until it doesn't. Format and lint could run in parallel but you've serialized them — wasting time. Worse, there's no cycle detection. If someone adds a dependency that creates a loop, you won't know until the pipeline hangs.

And if you want to figure out the *optimal* ordering by brute force? With 6 jobs, there are 6! = 720 possible orderings. With 20 jobs, that's 2.4 quintillion. Good luck.

## The Insight

What you drew on the whiteboard has a name: a **Directed Acyclic Graph** (DAG).

- **Directed**: arrows go one way (Build *must come before* Test, not the other way)
- **Acyclic**: no cycles (you can't follow arrows and end up where you started)
- **Graph**: nodes connected by edges

The question "what order should I run these jobs?" is exactly **topological sort** — line up the nodes so that every arrow points forward. And the question "did someone create a cycle?" falls out of the same algorithm for free.

## The Build

We'll represent the graph with an **adjacency list** — for each node, store which nodes it points to. Nodes are indices, and we keep a name map for readability.

```rust
use std::collections::VecDeque;

pub struct Dag {
    node_names: Vec<String>,
    adj: Vec<Vec<usize>>,  // adj[i] = list of nodes that i points TO
}

impl Dag {
    pub fn new() -> Self {
        Dag {
            node_names: Vec::new(),
            adj: Vec::new(),
        }
    }

    /// Add a job to the pipeline. Returns its index.
    pub fn add_node(&mut self, name: &str) -> usize {
        let idx = self.node_names.len();
        self.node_names.push(name.to_string());
        self.adj.push(Vec::new());
        idx
    }

    /// Add a dependency: `from` must complete before `to` can start.
    pub fn add_edge(&mut self, from: usize, to: usize) {
        self.adj[from].push(to);
    }

    pub fn node_name(&self, idx: usize) -> &str {
        &self.node_names[idx]
    }

    pub fn node_count(&self) -> usize {
        self.node_names.len()
    }
}
```

Now for the main event: **Kahn's algorithm** for topological sort. The idea is beautiful in its simplicity:

1. Compute the **in-degree** of each node (how many arrows point *into* it)
2. All nodes with in-degree 0 have no dependencies — they can run first. Put them in a queue.
3. Process the queue: take a node out, "remove" its outgoing edges (decrement in-degrees of its neighbors). If any neighbor's in-degree drops to 0, add it to the queue.
4. If you process all nodes, you have a valid order. If some nodes are left with non-zero in-degree, **there's a cycle**.

```rust
impl Dag {
    /// Topological sort using Kahn's algorithm.
    /// Returns Ok(ordered_indices) or Err with the cycle participants.
    pub fn topological_sort(&self) -> Result<Vec<usize>, Vec<usize>> {
        let n = self.node_count();

        // Step 1: Compute in-degrees
        let mut in_degree = vec![0usize; n];
        for edges in &self.adj {
            for &to in edges {
                in_degree[to] += 1;
            }
        }

        // Step 2: Enqueue all nodes with in-degree 0
        let mut queue = VecDeque::new();
        for i in 0..n {
            if in_degree[i] == 0 {
                queue.push_back(i);
            }
        }

        // Step 3: Process
        let mut order = Vec::with_capacity(n);
        while let Some(node) = queue.pop_front() {
            order.push(node);
            for &neighbor in &self.adj[node] {
                in_degree[neighbor] -= 1;
                if in_degree[neighbor] == 0 {
                    queue.push_back(neighbor);
                }
            }
        }

        // Step 4: Cycle detection
        if order.len() == n {
            Ok(order)
        } else {
            // Nodes still with in_degree > 0 are in a cycle
            let cycle_nodes: Vec<usize> = (0..n)
                .filter(|&i| in_degree[i] > 0)
                .collect();
            Err(cycle_nodes)
        }
    }

    /// Which jobs can run in parallel at each stage?
    pub fn parallel_stages(&self) -> Result<Vec<Vec<usize>>, Vec<usize>> {
        let n = self.node_count();
        let mut in_degree = vec![0usize; n];
        for edges in &self.adj {
            for &to in edges {
                in_degree[to] += 1;
            }
        }

        let mut stages: Vec<Vec<usize>> = Vec::new();
        let mut remaining = n;

        loop {
            // Collect all nodes with in-degree 0
            let stage: Vec<usize> = (0..n)
                .filter(|&i| in_degree[i] == 0)
                .collect();

            if stage.is_empty() {
                break;
            }

            // "Remove" these nodes
            for &node in &stage {
                in_degree[node] = usize::MAX; // mark as processed
                for &neighbor in &self.adj[node] {
                    if in_degree[neighbor] != usize::MAX {
                        in_degree[neighbor] -= 1;
                    }
                }
            }

            remaining -= stage.len();
            stages.push(stage);
        }

        if remaining > 0 {
            let cycle_nodes: Vec<usize> = (0..n)
                .filter(|&i| in_degree[i] != usize::MAX)
                .collect();
            Err(cycle_nodes)
        } else {
            Ok(stages)
        }
    }
}
```

## The Payoff

Let's wire up our CI pipeline:

```rust
fn main() {
    let mut dag = Dag::new();

    let format = dag.add_node("format");
    let lint   = dag.add_node("lint");
    let build  = dag.add_node("build");
    let test   = dag.add_node("test");
    let audit  = dag.add_node("audit");
    let deploy = dag.add_node("deploy");

    // Dependencies: format and lint must finish before build
    dag.add_edge(format, build);
    dag.add_edge(lint, build);

    // Build must finish before test
    dag.add_edge(build, test);

    // Audit must finish before test (security check)
    dag.add_edge(audit, test);

    // Test must finish before deploy
    dag.add_edge(test, deploy);

    // What's the execution order?
    match dag.topological_sort() {
        Ok(order) => {
            println!("Pipeline execution order:");
            for (i, &node) in order.iter().enumerate() {
                println!("  {}. {}", i + 1, dag.node_name(node));
            }
        }
        Err(cycle) => {
            println!("CYCLE DETECTED in jobs:");
            for &node in &cycle {
                println!("  - {}", dag.node_name(node));
            }
        }
    }
    // Output:
    //   1. format
    //   2. lint
    //   3. audit
    //   4. build
    //   5. test
    //   6. deploy

    // What can run in parallel?
    match dag.parallel_stages() {
        Ok(stages) => {
            println!("\nParallel execution plan:");
            for (i, stage) in stages.iter().enumerate() {
                let names: Vec<&str> = stage.iter()
                    .map(|&n| dag.node_name(n))
                    .collect();
                println!("  Stage {}: {:?}", i + 1, names);
            }
        }
        Err(_) => println!("Cycle detected!"),
    }
    // Output:
    //   Stage 1: ["format", "lint", "audit"]   <-- all three in parallel!
    //   Stage 2: ["build"]
    //   Stage 3: ["test"]
    //   Stage 4: ["deploy"]

    // Now let's break it — someone makes build depend on test
    println!("\n--- Introducing a circular dependency ---");
    let mut bad_dag = Dag::new();
    let b = bad_dag.add_node("build");
    let t = bad_dag.add_node("test");
    bad_dag.add_edge(b, t);
    bad_dag.add_edge(t, b);  // Oops! test -> build -> test -> ...

    match bad_dag.topological_sort() {
        Ok(_) => println!("No cycle (this shouldn't happen)"),
        Err(cycle) => {
            println!("CYCLE DETECTED! These jobs are deadlocked:");
            for &node in &cycle {
                println!("  - {}", bad_dag.node_name(node));
            }
        }
    }
    // Output:
    //   CYCLE DETECTED! These jobs are deadlocked:
    //   - build
    //   - test
}
```

Without the DAG, format/lint/audit would run sequentially — wasting time. With parallel stages, Stage 1 runs three jobs simultaneously. That's real time savings on every push.

## Complexity Comparison

| Operation | Naive (try all orderings) | DAG + Topological Sort |
|-----------|--------------------------|----------------------|
| Find valid order | O(n!) permutations | **O(V + E)** |
| Cycle detection | Hope and pray | **O(V + E)** — falls out for free |
| Parallel stages | Manual analysis | **O(V + E)** |
| Add a new job | Restructure everything | `add_node` + `add_edge` |

Where V = number of jobs and E = number of dependency edges. For a typical CI pipeline with 10-20 jobs, this is essentially instant.

## Try It Yourself

1. **Critical path**: The longest path through the DAG determines the minimum total pipeline time. If format takes 10s, lint takes 15s, build takes 60s, test takes 120s, audit takes 30s, and deploy takes 45s — what's the minimum pipeline duration? Write a function `critical_path(&self, durations: &[u64]) -> u64` that computes it.

2. **Reverse dependencies**: Add a `dependents(&self, node: usize) -> Vec<usize>` method that returns all jobs that (transitively) depend on a given job. If `build` is broken, which downstream jobs are affected? This is useful for "blast radius" analysis.

3. **Pipeline visualization**: Write a `fn to_ascii(&self) -> String` that prints the DAG as an ASCII diagram showing which jobs are at each parallel stage, with arrows showing dependencies. This is what you'd display in a CI dashboard.
