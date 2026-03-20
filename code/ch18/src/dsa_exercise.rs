// Chapter 18 DSA Exercise: Pipeline as Directed Acyclic Graph (DAG)
//
// CI/CD pipelines are DAG scheduling problems. Jobs are nodes, dependencies
// are edges. Topological sort determines execution order.
// GitHub Actions uses the `needs` keyword to express this DAG.

use std::collections::{HashMap, HashSet, VecDeque};

// ----------------------------------------------------------------
// Part 1: DAG representation and topological sort
// ----------------------------------------------------------------

#[derive(Debug, Clone)]
struct Job {
    name: String,
    duration_seconds: u32,
    dependencies: Vec<String>,
}

struct Pipeline {
    jobs: HashMap<String, Job>,
}

impl Pipeline {
    fn new() -> Self {
        Pipeline {
            jobs: HashMap::new(),
        }
    }

    fn add_job(&mut self, name: &str, duration: u32, deps: &[&str]) {
        self.jobs.insert(
            name.to_string(),
            Job {
                name: name.to_string(),
                duration_seconds: duration,
                dependencies: deps.iter().map(|s| s.to_string()).collect(),
            },
        );
    }

    /// Kahn's algorithm: BFS-based topological sort.
    /// Returns jobs in a valid execution order, or Err if there is a cycle.
    fn topological_sort_kahn(&self) -> Result<Vec<String>, String> {
        // Build in-degree map
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();

        for (name, _) in &self.jobs {
            in_degree.entry(name.clone()).or_insert(0);
            adj.entry(name.clone()).or_insert_with(Vec::new);
        }

        for (name, job) in &self.jobs {
            for dep in &job.dependencies {
                adj.entry(dep.clone())
                    .or_insert_with(Vec::new)
                    .push(name.clone());
                *in_degree.entry(name.clone()).or_insert(0) += 1;
            }
        }

        // Start with nodes that have no dependencies
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(name, _)| name.clone())
            .collect();

        // Sort the initial queue for deterministic output
        let mut sorted_start: Vec<String> = queue.drain(..).collect();
        sorted_start.sort();
        queue.extend(sorted_start);

        let mut result = Vec::new();
        while let Some(node) = queue.pop_front() {
            result.push(node.clone());
            if let Some(neighbors) = adj.get(&node) {
                let mut next_ready = Vec::new();
                for neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        next_ready.push(neighbor.clone());
                    }
                }
                next_ready.sort();
                queue.extend(next_ready);
            }
        }

        if result.len() != self.jobs.len() {
            Err("Cycle detected in pipeline!".to_string())
        } else {
            Ok(result)
        }
    }

    /// DFS-based topological sort (alternative approach)
    fn topological_sort_dfs(&self) -> Result<Vec<String>, String> {
        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new(); // for cycle detection
        let mut result = Vec::new();

        // Sort job names for deterministic output
        let mut job_names: Vec<&String> = self.jobs.keys().collect();
        job_names.sort();

        for name in job_names {
            if !visited.contains(name) {
                self.dfs_visit(name, &mut visited, &mut in_stack, &mut result)?;
            }
        }

        result.reverse();
        Ok(result)
    }

    fn dfs_visit(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        in_stack: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) -> Result<(), String> {
        if in_stack.contains(node) {
            return Err(format!("Cycle detected at job '{}'", node));
        }
        if visited.contains(node) {
            return Ok(());
        }

        in_stack.insert(node.to_string());

        if let Some(job) = self.jobs.get(node) {
            let mut sorted_deps = job.dependencies.clone();
            sorted_deps.sort();
            for dep in &sorted_deps {
                self.dfs_visit(dep, visited, in_stack, result)?;
            }
        }

        in_stack.remove(node);
        visited.insert(node.to_string());
        result.push(node.to_string());
        Ok(())
    }

    /// Calculate which jobs can run in parallel at each "level"
    fn parallel_schedule(&self) -> Result<Vec<Vec<String>>, String> {
        let order = self.topological_sort_kahn()?;

        // Calculate the "level" of each job (max dependency depth + 1)
        let mut levels: HashMap<String, usize> = HashMap::new();
        for name in &order {
            let job = &self.jobs[name];
            let level = if job.dependencies.is_empty() {
                0
            } else {
                job.dependencies
                    .iter()
                    .map(|dep| levels.get(dep).copied().unwrap_or(0) + 1)
                    .max()
                    .unwrap_or(0)
            };
            levels.insert(name.clone(), level);
        }

        // Group by level
        let max_level = levels.values().copied().max().unwrap_or(0);
        let mut schedule: Vec<Vec<String>> = vec![Vec::new(); max_level + 1];
        for (name, level) in &levels {
            schedule[*level].push(name.clone());
        }
        for level in &mut schedule {
            level.sort();
        }
        Ok(schedule)
    }

    /// Calculate critical path (longest path through the DAG)
    fn critical_path(&self) -> Result<(Vec<String>, u32), String> {
        let order = self.topological_sort_kahn()?;
        let mut dist: HashMap<String, u32> = HashMap::new();
        let mut prev: HashMap<String, String> = HashMap::new();

        for name in &order {
            let job = &self.jobs[name];
            let max_dep_dist = job
                .dependencies
                .iter()
                .map(|dep| dist.get(dep).copied().unwrap_or(0))
                .max()
                .unwrap_or(0);
            dist.insert(name.clone(), max_dep_dist + job.duration_seconds);

            if let Some(longest_dep) = job
                .dependencies
                .iter()
                .max_by_key(|dep| dist.get(*dep).copied().unwrap_or(0))
            {
                prev.insert(name.clone(), longest_dep.clone());
            }
        }

        // Find the job with maximum distance
        let (end_job, &total_time) = dist.iter().max_by_key(|(_, &d)| d).unwrap();

        // Trace back the critical path
        let mut path = vec![end_job.clone()];
        let mut current = end_job.clone();
        while let Some(p) = prev.get(&current) {
            path.push(p.clone());
            current = p.clone();
        }
        path.reverse();
        Ok((path, total_time))
    }

    /// Print the DAG as ASCII art
    fn print_dag(&self) {
        let schedule = self.parallel_schedule().unwrap_or_default();
        for (level, jobs) in schedule.iter().enumerate() {
            let job_strs: Vec<String> = jobs
                .iter()
                .map(|name| {
                    let job = &self.jobs[name];
                    format!("[{} ({}s)]", name, job.duration_seconds)
                })
                .collect();
            println!("  Level {}: {}", level, job_strs.join("  "));
        }
    }
}

// ----------------------------------------------------------------
// Part 2: Cycle detection in a dependency graph
// Interview problem: detect if adding a dependency would create a cycle.
// ----------------------------------------------------------------

fn would_create_cycle(
    pipeline: &Pipeline,
    from: &str,
    to: &str,
) -> bool {
    // Check if there is already a path from `to` to `from` (BFS)
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(to.to_string());

    while let Some(node) = queue.pop_front() {
        if node == from {
            return true; // adding from->to would create a cycle
        }
        if visited.contains(&node) {
            continue;
        }
        visited.insert(node.clone());
        if let Some(job) = pipeline.jobs.get(&node) {
            for dep in &job.dependencies {
                queue.push_back(dep.clone());
            }
        }
    }
    false
}

fn main() {
    println!("=== Pipeline as Directed Acyclic Graph ===\n");

    // Part 1: GrindIt's CI pipeline
    println!("--- Part 1: GrindIt CI Pipeline ---");
    let mut pipeline = Pipeline::new();
    pipeline.add_job("fmt", 10, &[]);
    pipeline.add_job("clippy", 45, &[]);
    pipeline.add_job("test", 120, &[]);
    pipeline.add_job("merge-gate", 1, &["fmt", "clippy", "test"]);

    println!("  DAG structure:");
    pipeline.print_dag();

    println!("\n  Topological order (Kahn's):");
    match pipeline.topological_sort_kahn() {
        Ok(order) => println!("    {}", order.join(" -> ")),
        Err(e) => println!("    ERROR: {}", e),
    }

    println!("\n  Topological order (DFS):");
    match pipeline.topological_sort_dfs() {
        Ok(order) => println!("    {}", order.join(" -> ")),
        Err(e) => println!("    ERROR: {}", e),
    }

    println!("\n  Parallel schedule:");
    if let Ok(schedule) = pipeline.parallel_schedule() {
        for (level, jobs) in schedule.iter().enumerate() {
            println!("    Step {}: {} (parallel)", level, jobs.join(", "));
        }
    }

    if let Ok((path, time)) = pipeline.critical_path() {
        println!("\n  Critical path: {} (total: {}s)", path.join(" -> "), time);
        println!("  Sequential time: {}s", pipeline.jobs.values().map(|j| j.duration_seconds).sum::<u32>());
        println!("  Parallel saves: {}s", pipeline.jobs.values().map(|j| j.duration_seconds).sum::<u32>() - time);
    }

    // More complex pipeline with diamond dependency
    println!("\n--- Part 2: Complex Pipeline (Diamond Dependency) ---");
    let mut complex = Pipeline::new();
    complex.add_job("build", 60, &[]);
    complex.add_job("unit-test", 90, &["build"]);
    complex.add_job("lint", 30, &["build"]);
    complex.add_job("security-audit", 20, &["build"]);
    complex.add_job("deploy-staging", 45, &["unit-test", "lint"]);
    complex.add_job("integration-test", 180, &["deploy-staging"]);
    complex.add_job("deploy-production", 45, &["integration-test", "security-audit"]);

    println!("  DAG structure:");
    complex.print_dag();

    println!("\n  Topological order:");
    if let Ok(order) = complex.topological_sort_kahn() {
        println!("    {}", order.join(" -> "));
    }

    println!("\n  Parallel schedule:");
    if let Ok(schedule) = complex.parallel_schedule() {
        for (level, jobs) in schedule.iter().enumerate() {
            println!("    Step {}: {}", level, jobs.join(", "));
        }
    }

    if let Ok((path, time)) = complex.critical_path() {
        println!("\n  Critical path: {}", path.join(" -> "));
        println!("  Critical path time: {}s ({}m {}s)", time, time / 60, time % 60);
    }

    // Part 3: Cycle detection
    println!("\n--- Part 3: Cycle Detection ---");
    let test_edges = vec![
        ("build", "unit-test", false),       // normal direction
        ("deploy-production", "build", true), // would create cycle
        ("lint", "security-audit", false),    // no cycle
        ("integration-test", "build", true),  // would create cycle
    ];

    for (from, to, expected_cycle) in &test_edges {
        let has_cycle = would_create_cycle(&complex, to, from);
        let status = if has_cycle == *expected_cycle {
            "PASS"
        } else {
            "FAIL"
        };
        println!(
            "  [{}] Adding {} -> {} dependency: {}",
            status,
            from,
            to,
            if has_cycle {
                "WOULD CREATE CYCLE"
            } else {
                "safe"
            }
        );
    }

    // Part 4: Interview version — course schedule (LeetCode 207/210)
    println!("\n--- Part 4: Course Schedule (LeetCode 210) ---");
    println!("  Same algorithm as CI pipeline scheduling!");
    let mut courses = Pipeline::new();
    courses.add_job("Intro to Rust", 1, &[]);
    courses.add_job("Data Structures", 1, &["Intro to Rust"]);
    courses.add_job("Web Development", 1, &["Intro to Rust"]);
    courses.add_job("Databases", 1, &["Data Structures"]);
    courses.add_job("Algorithms", 1, &["Data Structures"]);
    courses.add_job("Full Stack", 1, &["Web Development", "Databases"]);
    courses.add_job("Capstone", 1, &["Full Stack", "Algorithms"]);

    if let Ok(order) = courses.topological_sort_kahn() {
        println!("  Course order: {}", order.join(" -> "));
    }

    if let Ok(schedule) = courses.parallel_schedule() {
        println!("  Semester plan:");
        for (sem, courses) in schedule.iter().enumerate() {
            println!("    Semester {}: {}", sem + 1, courses.join(", "));
        }
    }

    println!("\n=== Key Insights ===");
    println!("1. CI pipelines are DAGs — jobs are nodes, 'needs' creates edges");
    println!("2. Topological sort finds a valid execution order: O(V + E)");
    println!("3. Parallel scheduling groups jobs by dependency level");
    println!("4. Critical path = longest path = minimum pipeline time");
    println!("5. Sequential time = sum of all jobs; parallel time = critical path");
    println!("6. Same algorithm applies to: course scheduling, build systems, task planning");
}
