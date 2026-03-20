// Chapter 8 DSA Exercise: N-ary Tree Traversal
//
// A WOD (Workout of the Day) is an N-ary tree:
//   WOD -> Sections -> Movements
// DFS visits one branch fully before the next (natural for rendering).
// BFS visits all nodes at one level before going deeper.

use std::collections::VecDeque;

/// A movement within a section (leaf node)
#[derive(Debug, Clone)]
struct Movement {
    exercise_name: String,
    rep_scheme: String,
    weight: Option<String>,
}

/// A section of a WOD (internal node)
#[derive(Debug, Clone)]
struct Section {
    title: String,
    section_type: String, // "fortime", "amrap", "emom", "strength"
    time_cap_seconds: Option<u32>,
    movements: Vec<Movement>,
}

/// A Workout of the Day (root node)
#[derive(Debug, Clone)]
struct Wod {
    title: String,
    date: String,
    sections: Vec<Section>,
}

/// Represents a node in the WOD tree for traversal purposes
#[derive(Debug)]
enum WodNode<'a> {
    Root(&'a Wod),
    Section(&'a Section),
    Movement(&'a Movement),
}

impl<'a> WodNode<'a> {
    fn label(&self) -> String {
        match self {
            WodNode::Root(wod) => format!("WOD: {} ({})", wod.title, wod.date),
            WodNode::Section(sec) => {
                let time_str = sec
                    .time_cap_seconds
                    .map(|t| format!(", {}min cap", t / 60))
                    .unwrap_or_default();
                format!("Section: {} [{}{}]", sec.title, sec.section_type, time_str)
            }
            WodNode::Movement(mvt) => {
                let weight_str = mvt
                    .weight
                    .as_ref()
                    .map(|w| format!(" @ {}", w))
                    .unwrap_or_default();
                format!("Movement: {} {}{}", mvt.exercise_name, mvt.rep_scheme, weight_str)
            }
        }
    }

    fn children(&self, wod: &'a Wod) -> Vec<WodNode<'a>> {
        match self {
            WodNode::Root(_) => wod
                .sections
                .iter()
                .map(|s| WodNode::Section(s))
                .collect(),
            WodNode::Section(sec) => sec
                .movements
                .iter()
                .map(|m| WodNode::Movement(m))
                .collect(),
            WodNode::Movement(_) => vec![],
        }
    }
}

/// DFS traversal — visits one branch fully before the next.
/// This is the natural rendering order: Section 1 header, Section 1 movements,
/// Section 2 header, Section 2 movements, etc.
fn dfs_traversal(wod: &Wod) -> Vec<(usize, String)> {
    let mut result = Vec::new();
    let root = WodNode::Root(wod);

    fn dfs<'a>(
        node: WodNode<'a>,
        wod: &'a Wod,
        depth: usize,
        result: &mut Vec<(usize, String)>,
    ) {
        result.push((depth, node.label()));
        let children = node.children(wod);
        for child in children {
            dfs(child, wod, depth + 1, result);
        }
    }

    dfs(root, wod, 0, &mut result);
    result
}

/// BFS traversal — visits all nodes at one level before going deeper.
/// Useful for: "find all sections of type AMRAP" or "count movements across all sections".
fn bfs_traversal(wod: &Wod) -> Vec<(usize, String)> {
    let mut result = Vec::new();
    let mut queue: VecDeque<(WodNode<'_>, usize)> = VecDeque::new();
    queue.push_back((WodNode::Root(wod), 0));

    while let Some((node, depth)) = queue.pop_front() {
        let children = node.children(wod);
        result.push((depth, node.label()));
        for child in children {
            queue.push_back((child, depth + 1));
        }
    }

    result
}

/// Count total movements across all sections (uses BFS-like flat iteration)
fn count_movements(wod: &Wod) -> usize {
    wod.sections.iter().map(|s| s.movements.len()).sum()
}

/// Find all sections of a specific type
fn find_sections_by_type<'a>(wod: &'a Wod, section_type: &str) -> Vec<&'a Section> {
    wod.sections
        .iter()
        .filter(|s| s.section_type == section_type)
        .collect()
}

/// Find all movements for a specific exercise across all sections
fn find_exercise_across_sections<'a>(wod: &'a Wod, exercise: &str) -> Vec<(&'a str, &'a Movement)> {
    let mut results = Vec::new();
    for section in &wod.sections {
        for movement in &section.movements {
            if movement.exercise_name.to_lowercase().contains(&exercise.to_lowercase()) {
                results.push((section.title.as_str(), movement));
            }
        }
    }
    results
}

// ----------------------------------------------------------------
// Interview Problem: Maximum depth of an N-ary tree
// Our WOD tree has fixed depth (3), but the algorithm generalizes.
// ----------------------------------------------------------------
fn max_depth(wod: &Wod) -> usize {
    if wod.sections.is_empty() {
        return 1; // Just the root
    }
    let max_section_depth = wod
        .sections
        .iter()
        .map(|s| {
            if s.movements.is_empty() {
                1 // Section with no movements
            } else {
                2 // Section + movements
            }
        })
        .max()
        .unwrap_or(0);
    1 + max_section_depth // root + deepest branch
}

// ----------------------------------------------------------------
// Interview Problem: Serialize tree to flat list (for database storage)
// This is how the normalized DB schema works — tree to tables.
// ----------------------------------------------------------------
#[derive(Debug)]
struct FlatRecord {
    level: &'static str,
    parent_id: Option<usize>,
    id: usize,
    label: String,
}

fn flatten_wod(wod: &Wod) -> Vec<FlatRecord> {
    let mut records = Vec::new();
    let wod_id = 0;
    records.push(FlatRecord {
        level: "wod",
        parent_id: None,
        id: wod_id,
        label: wod.title.clone(),
    });

    let mut next_id = 1;
    for section in &wod.sections {
        let section_id = next_id;
        next_id += 1;
        records.push(FlatRecord {
            level: "section",
            parent_id: Some(wod_id),
            id: section_id,
            label: format!("{} [{}]", section.title, section.section_type),
        });

        for movement in &section.movements {
            records.push(FlatRecord {
                level: "movement",
                parent_id: Some(section_id),
                id: next_id,
                label: format!("{} {}", movement.exercise_name, movement.rep_scheme),
            });
            next_id += 1;
        }
    }
    records
}

fn main() {
    // Build a sample WOD tree
    let wod = Wod {
        title: "Murph".to_string(),
        date: "2024-05-27".to_string(),
        sections: vec![
            Section {
                title: "Warm-Up".to_string(),
                section_type: "warmup".to_string(),
                time_cap_seconds: Some(600),
                movements: vec![
                    Movement {
                        exercise_name: "Rowing".to_string(),
                        rep_scheme: "500m".to_string(),
                        weight: None,
                    },
                    Movement {
                        exercise_name: "PVC Pass-Throughs".to_string(),
                        rep_scheme: "2x10".to_string(),
                        weight: None,
                    },
                ],
            },
            Section {
                title: "Strength".to_string(),
                section_type: "strength".to_string(),
                time_cap_seconds: Some(1200),
                movements: vec![
                    Movement {
                        exercise_name: "Back Squat".to_string(),
                        rep_scheme: "5-5-5-5-5".to_string(),
                        weight: Some("80% 1RM".to_string()),
                    },
                ],
            },
            Section {
                title: "Murph".to_string(),
                section_type: "fortime".to_string(),
                time_cap_seconds: Some(3600),
                movements: vec![
                    Movement {
                        exercise_name: "Run".to_string(),
                        rep_scheme: "1 mile".to_string(),
                        weight: None,
                    },
                    Movement {
                        exercise_name: "Pull-Up".to_string(),
                        rep_scheme: "100".to_string(),
                        weight: None,
                    },
                    Movement {
                        exercise_name: "Push-Up".to_string(),
                        rep_scheme: "200".to_string(),
                        weight: None,
                    },
                    Movement {
                        exercise_name: "Air Squat".to_string(),
                        rep_scheme: "300".to_string(),
                        weight: None,
                    },
                    Movement {
                        exercise_name: "Run".to_string(),
                        rep_scheme: "1 mile".to_string(),
                        weight: None,
                    },
                ],
            },
            Section {
                title: "Cool Down".to_string(),
                section_type: "static".to_string(),
                time_cap_seconds: None,
                movements: vec![
                    Movement {
                        exercise_name: "Stretching".to_string(),
                        rep_scheme: "5 min".to_string(),
                        weight: None,
                    },
                ],
            },
        ],
    };

    // DFS Traversal (rendering order)
    println!("=== DFS Traversal (Rendering Order) ===");
    let dfs_result = dfs_traversal(&wod);
    for (depth, label) in &dfs_result {
        let indent = "  ".repeat(*depth);
        println!("{}{}", indent, label);
    }

    // BFS Traversal
    println!("\n=== BFS Traversal (Level Order) ===");
    let bfs_result = bfs_traversal(&wod);
    for (depth, label) in &bfs_result {
        let indent = "  ".repeat(*depth);
        println!("{}[L{}] {}", indent, depth, label);
    }

    // Queries
    println!("\n=== Tree Queries ===");
    println!("Total movements: {}", count_movements(&wod));
    println!("Max depth: {}", max_depth(&wod));

    let amrap_sections = find_sections_by_type(&wod, "fortime");
    println!(
        "For-Time sections: {:?}",
        amrap_sections.iter().map(|s| &s.title).collect::<Vec<_>>()
    );

    println!("\nExercise 'Run' appears in:");
    for (section_title, movement) in find_exercise_across_sections(&wod, "Run") {
        println!("  - {} > {} {}", section_title, movement.exercise_name, movement.rep_scheme);
    }

    // Flatten to database records
    println!("\n=== Flatten Tree to Database Records ===");
    println!("(How the normalized schema stores the tree)");
    let flat = flatten_wod(&wod);
    for record in &flat {
        println!(
            "  {:>8} | id={:<3} parent={:<6} | {}",
            record.level,
            record.id,
            record
                .parent_id
                .map(|p| p.to_string())
                .unwrap_or("NULL".to_string()),
            record.label
        );
    }

    println!("\n=== Key Insights ===");
    println!("DFS: Natural for rendering — shows complete sections one at a time");
    println!("BFS: Useful for level-based queries — 'find all sections' or 'count movements'");
    println!("Database: Stores tree as normalized tables (wods -> sections -> movements)");
    println!("Queries across branches: SELECT FROM movements WHERE exercise_id = X");
    println!("  This avoids traversing the tree entirely — the flat structure enables it");
}
