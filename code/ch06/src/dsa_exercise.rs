// Chapter 6 DSA Exercise: Tree Matching (Prefix Tree / Trie for Route Resolution)
//
// Route resolution in leptos_router works like a prefix tree traversal.
// Each URL segment is a node, and the router walks the tree to find a match.
// Lookup is O(depth) — proportional to URL segments, not total route count.

use std::collections::HashMap;

/// A route handler — in a real framework this would be a component or function.
#[derive(Debug, Clone)]
struct RouteHandler {
    name: String,
}

/// A node in the route trie. Each node can have:
/// - Static children (exact match on a URL segment)
/// - A dynamic child (matches any segment, captures it as a parameter)
/// - A handler (if this node is a valid route endpoint)
#[derive(Debug)]
struct RouteNode {
    /// Static children keyed by segment name
    static_children: HashMap<String, RouteNode>,
    /// Dynamic child (e.g., ":id" or "[id]")
    dynamic_child: Option<(String, Box<RouteNode>)>,
    /// Handler if this node is a route endpoint
    handler: Option<RouteHandler>,
}

impl RouteNode {
    fn new() -> Self {
        RouteNode {
            static_children: HashMap::new(),
            dynamic_child: None,
            handler: None,
        }
    }
}

/// A route trie (prefix tree) for URL matching.
struct Router {
    root: RouteNode,
}

/// Result of a route match — handler + captured parameters.
#[derive(Debug)]
struct RouteMatch {
    handler: RouteHandler,
    params: HashMap<String, String>,
}

impl Router {
    fn new() -> Self {
        Router {
            root: RouteNode::new(),
        }
    }

    /// Register a route pattern like "/exercises/:id/edit"
    fn add_route(&mut self, pattern: &str, handler_name: &str) {
        let segments: Vec<&str> = pattern
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let mut node = &mut self.root;
        for segment in &segments {
            if segment.starts_with(':') || segment.starts_with('[') {
                // Dynamic segment
                let param_name = segment
                    .trim_start_matches(':')
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .to_string();
                if node.dynamic_child.is_none() {
                    node.dynamic_child = Some((param_name.clone(), Box::new(RouteNode::new())));
                }
                node = &mut node.dynamic_child.as_mut().unwrap().1;
            } else {
                // Static segment
                node = node
                    .static_children
                    .entry(segment.to_string())
                    .or_insert_with(RouteNode::new);
            }
        }
        node.handler = Some(RouteHandler {
            name: handler_name.to_string(),
        });
    }

    /// Match a URL path, returning the handler and captured params.
    /// This is the core trie traversal — O(depth) where depth = number of segments.
    fn match_route(&self, path: &str) -> Option<RouteMatch> {
        let segments: Vec<&str> = path
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let mut node = &self.root;
        let mut params = HashMap::new();
        let mut comparisons = 0;

        // Handle root path
        if segments.is_empty() {
            return node.handler.as_ref().map(|h| RouteMatch {
                handler: h.clone(),
                params: HashMap::new(),
            });
        }

        for segment in &segments {
            comparisons += 1;

            // Try static match first (higher priority)
            if let Some(child) = node.static_children.get(*segment) {
                node = child;
            } else if let Some((param_name, child)) = &node.dynamic_child {
                // Fall back to dynamic match
                params.insert(param_name.clone(), segment.to_string());
                node = child;
            } else {
                println!("    [miss after {} comparisons]", comparisons);
                return None;
            }
        }

        println!("    [matched in {} comparisons]", comparisons);
        node.handler.as_ref().map(|h| RouteMatch {
            handler: h.clone(),
            params,
        })
    }

    /// Print the route tree for visualization
    fn print_tree(&self) {
        println!("Route Trie:");
        self.print_node(&self.root, 0, "(root)");
    }

    fn print_node(&self, node: &RouteNode, depth: usize, label: &str) {
        let indent = "  ".repeat(depth);
        let handler_str = match &node.handler {
            Some(h) => format!(" => {}", h.name),
            None => String::new(),
        };
        println!("{}{}{}", indent, label, handler_str);

        for (segment, child) in &node.static_children {
            self.print_node(child, depth + 1, segment);
        }
        if let Some((param, child)) = &node.dynamic_child {
            self.print_node(child, depth + 1, &format!(":{}", param));
        }
    }
}

fn main() {
    println!("=== Route Resolution as Prefix Tree ===\n");

    // Build the GrindIt route table
    let mut router = Router::new();
    router.add_route("/", "HomePage");
    router.add_route("/exercises", "ExercisesPage");
    router.add_route("/exercises/:id", "ExerciseDetailPage");
    router.add_route("/exercises/:id/edit", "ExerciseEditPage");
    router.add_route("/log", "LogWorkoutPage");
    router.add_route("/log/:wod_id", "LogWodScorePage");
    router.add_route("/history", "HistoryPage");
    router.add_route("/login", "LoginPage");
    router.add_route("/profile", "ProfilePage");
    router.add_route("/admin", "AdminPage");
    router.add_route("/admin/users", "AdminUsersPage");
    router.add_route("/admin/users/:id", "AdminUserDetailPage");
    router.add_route("/api/v1/health_check", "HealthCheckHandler");
    router.add_route("/api/v1/exercises", "ApiExercisesHandler");

    // Print the tree structure
    router.print_tree();
    println!();

    // Test route matching
    println!("=== Route Matching Tests ===\n");
    let test_paths = vec![
        "/",
        "/exercises",
        "/exercises/abc-123",
        "/exercises/abc-123/edit",
        "/log",
        "/log/wod-456",
        "/history",
        "/admin/users/user-789",
        "/api/v1/health_check",
        "/api/v1/exercises",
        "/nonexistent",
        "/exercises/abc-123/nonexistent",
    ];

    for path in test_paths {
        print!("  {} -> ", path);
        match router.match_route(path) {
            Some(m) => {
                if m.params.is_empty() {
                    println!("    {} (no params)", m.handler.name);
                } else {
                    println!(
                        "    {} (params: {:?})",
                        m.handler.name, m.params
                    );
                }
            }
            None => println!("    404 Not Found"),
        }
    }

    // Performance analysis
    println!("\n=== Performance Analysis ===");
    println!("Route matching is O(depth) where depth = number of URL segments.");
    println!("It does NOT depend on the total number of routes.");
    println!();
    println!("{:<30} {:>8}", "URL", "Depth");
    println!("{}", "-".repeat(40));
    let analysis_paths = [
        "/exercises",
        "/exercises/abc-123",
        "/exercises/abc-123/edit",
        "/admin/users/user-789",
        "/api/v1/health_check",
    ];
    for path in &analysis_paths {
        let depth = path.trim_matches('/').split('/').count();
        println!("{:<30} {:>8}", path, depth);
    }
    println!();
    println!("Whether you have 5 routes or 500, matching /exercises takes");
    println!("the same time: walk 1 level deep in the trie.");
    println!();
    println!("Static segments use HashMap lookup (O(1) average).");
    println!("Dynamic segments (:id) match any value and capture it as a parameter.");
    println!("Static matches have higher priority than dynamic matches.");
}
