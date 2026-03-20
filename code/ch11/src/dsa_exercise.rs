// Chapter 11 DSA Exercise: Stack (LIFO)
//
// Modal z-index as a stack. The most recently opened modal sits on top
// and must be dismissed first. Event bubbling is also stack-like.

use std::fmt;

// ----------------------------------------------------------------
// Part 1: Modal Stack — LIFO for UI overlay management
// ----------------------------------------------------------------

#[derive(Debug, Clone)]
struct Modal {
    id: String,
    title: String,
    z_index: u32,
}

impl fmt::Display for Modal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Modal('{}', z-index={})", self.title, self.z_index)
    }
}

struct ModalStack {
    modals: Vec<Modal>,
    base_z_index: u32,
}

impl ModalStack {
    fn new() -> Self {
        ModalStack {
            modals: Vec::new(),
            base_z_index: 1000,
        }
    }

    /// Open a new modal — pushes onto the stack with the next z-index
    fn open(&mut self, id: &str, title: &str) {
        let z_index = self.base_z_index + self.modals.len() as u32;
        let modal = Modal {
            id: id.to_string(),
            title: title.to_string(),
            z_index,
        };
        println!("  OPEN:  {}", modal);
        self.modals.push(modal);
    }

    /// Close the topmost modal — LIFO order
    fn close_top(&mut self) -> Option<Modal> {
        let modal = self.modals.pop();
        if let Some(ref m) = modal {
            println!("  CLOSE: {}", m);
        }
        modal
    }

    /// Close a specific modal by id (removing from anywhere in the stack)
    fn close_by_id(&mut self, id: &str) -> Option<Modal> {
        if let Some(pos) = self.modals.iter().position(|m| m.id == id) {
            let modal = self.modals.remove(pos);
            println!("  CLOSE: {} (removed from position {})", modal, pos);
            Some(modal)
        } else {
            None
        }
    }

    /// Get the topmost (visible) modal
    fn top(&self) -> Option<&Modal> {
        self.modals.last()
    }

    fn is_empty(&self) -> bool {
        self.modals.is_empty()
    }

    fn depth(&self) -> usize {
        self.modals.len()
    }

    fn print_stack(&self) {
        if self.modals.is_empty() {
            println!("  [empty stack]");
            return;
        }
        println!("  Stack (top to bottom):");
        for modal in self.modals.iter().rev() {
            println!("    | {} |", modal);
        }
        println!("    +{}+", "-".repeat(40));
    }
}

// ----------------------------------------------------------------
// Part 2: Classic Stack Problems — Interview Exercises
// ----------------------------------------------------------------

/// Balanced parentheses (LeetCode 20)
/// Used in expression parsing, HTML/XML validation, etc.
fn is_balanced(s: &str) -> bool {
    let mut stack: Vec<char> = Vec::new();
    for ch in s.chars() {
        match ch {
            '(' | '[' | '{' => stack.push(ch),
            ')' => {
                if stack.pop() != Some('(') {
                    return false;
                }
            }
            ']' => {
                if stack.pop() != Some('[') {
                    return false;
                }
            }
            '}' => {
                if stack.pop() != Some('{') {
                    return false;
                }
            }
            _ => {} // ignore other characters
        }
    }
    stack.is_empty()
}

/// Simplify file path (LeetCode 71)
/// Relevant for URL/route processing
fn simplify_path(path: &str) -> String {
    let mut stack: Vec<&str> = Vec::new();
    for component in path.split('/') {
        match component {
            "" | "." => {} // skip empty and current dir
            ".." => {
                stack.pop();
            }
            name => stack.push(name),
        }
    }
    format!("/{}", stack.join("/"))
}

/// Evaluate a workout notation in reverse Polish notation (RPN)
/// Example: "5 3 + 2 *" = (5 + 3) * 2 = 16 sets
fn eval_rpn(tokens: &[&str]) -> Result<i32, String> {
    let mut stack: Vec<i32> = Vec::new();
    for &token in tokens {
        match token {
            "+" | "-" | "*" | "/" => {
                let b = stack.pop().ok_or("Stack underflow")?;
                let a = stack.pop().ok_or("Stack underflow")?;
                let result = match token {
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    "/" => {
                        if b == 0 {
                            return Err("Division by zero".to_string());
                        }
                        a / b
                    }
                    _ => unreachable!(),
                };
                stack.push(result);
            }
            num => {
                let n: i32 = num
                    .parse()
                    .map_err(|_| format!("Invalid token: '{}'", num))?;
                stack.push(n);
            }
        }
    }
    stack.pop().ok_or_else(|| "Empty expression".to_string())
}

// ----------------------------------------------------------------
// Part 3: Event bubbling simulation (stack-like traversal)
// ----------------------------------------------------------------

#[derive(Debug)]
struct DomNode {
    tag: String,
    class: String,
    children: Vec<DomNode>,
    stop_propagation: bool,
}

fn simulate_click_bubbling(path: &[(&str, &str, bool)]) {
    println!("  Click event bubbling:");
    for (tag, class, stops) in path {
        println!("    -> <{} class=\"{}\"> handler fires", tag, class);
        if *stops {
            println!("       ev.stop_propagation() — bubbling stopped!");
            return;
        }
    }
    println!("    -> reached document root");
}

fn main() {
    println!("=== Stack (LIFO): Modal Management ===\n");

    // Part 1: Modal Stack
    println!("--- Part 1: Modal Stack ---");
    let mut stack = ModalStack::new();

    stack.open("delete-modal", "Delete Exercise");
    stack.open("confirm-modal", "Are you sure?");
    stack.open("error-modal", "Error occurred");

    println!();
    stack.print_stack();

    println!("\nClosing modals (LIFO order):");
    while !stack.is_empty() {
        stack.close_top();
    }
    println!();

    // Demonstrate closing a specific modal (like pressing Escape)
    println!("--- Nested Modal Scenario ---");
    stack.open("exercise-form", "Create Exercise");
    stack.open("category-select", "Select Category");
    stack.open("confirm-save", "Save Changes?");

    println!();
    stack.print_stack();

    println!("\nUser clicks overlay of middle modal:");
    stack.close_by_id("category-select");
    println!();
    stack.print_stack();

    // Clear
    while !stack.is_empty() {
        stack.close_top();
    }

    // Part 2: Classic Stack Problems
    println!("\n--- Part 2: Balanced Parentheses ---");
    let test_cases = vec![
        ("()", true),
        ("()[]{}", true),
        ("(]", false),
        ("{[()]}", true),
        ("((())", false),
        ("view! { <div class={move || class}></div> }", true),
    ];

    for (input, expected) in &test_cases {
        let result = is_balanced(input);
        let status = if result == *expected { "PASS" } else { "FAIL" };
        println!("  [{}] '{}' => balanced: {}", status, input, result);
    }

    println!("\n--- Simplify URL Path ---");
    let paths = vec![
        "/exercises/../log",
        "/exercises/./abc-123",
        "/admin/../../../exercises",
        "/api/v1//health_check",
        "/exercises/abc-123/edit",
    ];
    for path in &paths {
        println!("  {} => {}", path, simplify_path(path));
    }

    println!("\n--- Evaluate Workout RPN ---");
    let expressions = vec![
        (vec!["5", "3", "+", "2", "*"], "(5 + 3) * 2 = total reps"),
        (vec!["21", "15", "+", "9", "+"], "21 + 15 + 9 = Fran reps"),
        (vec!["100", "200", "+", "300", "+", "400", "+"], "100+200+300+400 = Murph total"),
    ];

    for (tokens, description) in &expressions {
        match eval_rpn(tokens) {
            Ok(result) => println!("  {} => {}: {}", tokens.join(" "), description, result),
            Err(e) => println!("  {} => ERROR: {}", tokens.join(" "), e),
        }
    }

    // Part 3: Event bubbling
    println!("\n--- Part 3: Event Bubbling (Stack-like) ---");

    println!("\nClick on delete button (no stopPropagation):");
    simulate_click_bubbling(&[
        ("button", "delete-btn", false),
        ("div", "exercise-card", false),
        ("div", "exercises-page", false),
    ]);

    println!("\nClick inside modal dialog (with stopPropagation):");
    simulate_click_bubbling(&[
        ("button", "confirm-btn", false),
        ("div", "modal-dialog", true), // stopPropagation here
        ("div", "modal-overlay", false),
    ]);
    println!("  (The overlay's close handler never fires — dialog caught the event)");

    println!("\n=== Key Insights ===");
    println!("1. Modal z-index management is LIFO — last opened = topmost = first closed");
    println!("2. Event bubbling traverses the DOM from child to parent (like a stack unwind)");
    println!("3. stopPropagation() is like 'popping' the current handler off the bubbling stack");
    println!("4. Vec<T> in Rust is a natural stack: push/pop are O(1) amortized");
    println!("5. Stack problems appear in: parsing, undo/redo, call stacks, and browser history");
}
