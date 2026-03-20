# The Undo Button (A Linked List Rite of Passage)

## The Problem: Delete Is Forever (Until It Isn't)

Your GrindIt delete modal works beautifully. Tap delete, confirm, exercise gone. Then your first real user does something predictable: deletes an exercise, deletes another, then says "wait, I didn't mean to delete the first one." They want Undo.

Your first instinct is a `Vec`:

```rust,ignore
#[derive(Debug, Clone)]
enum Action {
    DeleteExercise { id: u32, name: String },
    EditExercise { id: u32, old_name: String, new_name: String },
    AddExercise { id: u32, name: String },
}

struct UndoStack {
    history: Vec<Action>,
}

impl UndoStack {
    fn new() -> Self {
        UndoStack { history: Vec::new() }
    }

    fn perform(&mut self, action: Action) {
        println!("Performed: {:?}", action);
        self.history.push(action);
    }

    fn undo(&mut self) -> Option<Action> {
        self.history.pop()
    }
}
```

Push actions, pop to undo. Clean. Works. Ship it.

Then the product manager says: "I want Undo *and* Redo." Now you need to go backward *and* forward through history. Okay, add a cursor:

```rust,ignore
struct UndoRedoVec {
    history: Vec<Action>,
    cursor: usize, // points to the "current" position
}

impl UndoRedoVec {
    fn new() -> Self {
        UndoRedoVec { history: Vec::new(), cursor: 0 }
    }

    fn perform(&mut self, action: Action) {
        // Discard any "future" actions after the cursor
        self.history.truncate(self.cursor);
        self.history.push(action);
        self.cursor += 1;
    }

    fn undo(&mut self) -> Option<&Action> {
        if self.cursor > 0 {
            self.cursor -= 1;
            Some(&self.history[self.cursor])
        } else {
            None
        }
    }

    fn redo(&mut self) -> Option<&Action> {
        if self.cursor < self.history.len() {
            let action = &self.history[self.cursor];
            self.cursor += 1;
            Some(action)
        } else {
            None
        }
    }
}
```

This actually works great for the simple case. But then the feature request evolves: "When the user undoes to some point in the middle and performs a new action, I want to *keep* the discarded future as an alternate branch, not destroy it." Or: "I want to splice a checkpoint into the middle of the history." Now `truncate` won't cut it, and inserting into the middle of a Vec means shifting every element after the insertion point — O(n) for every splice.

Your gym buddy, who took CS 201, smirks and says "just use a linked list." Time to wipe that smirk off.

## The Insight: Linked Lists Are About Pointers (and Rust Hates Pointers)

A doubly-linked list gives you O(1) insertion and removal at any known position. Each node points to the next *and* the previous, so you can walk both directions — perfect for undo/redo.

But here's the thing about Rust: it *really* doesn't want you holding raw pointers. A classic linked list where each node owns a `Box` to the next and has a raw pointer back to the previous is a minefield of unsafe code. The borrow checker exists specifically to prevent the kind of aliased mutable references that doubly-linked lists require.

So we're going to use the **arena allocation pattern** — the Rust-idiomatic way to build graphs and linked structures. Instead of nodes pointing at each other via references, nodes hold *indices* into a shared `Vec`. The Vec owns all the nodes. Indices are just `usize` — they're `Copy`, they don't borrow anything, and the borrow checker is happy.

## The Build: Arena-Backed Doubly-Linked List

```rust
#[derive(Debug, Clone)]
struct Node<T> {
    value: T,
    prev: Option<usize>,
    next: Option<usize>,
}

#[derive(Debug)]
struct LinkedList<T> {
    arena: Vec<Node<T>>,
    head: Option<usize>,
    tail: Option<usize>,
    len: usize,
}

impl<T: std::fmt::Debug> LinkedList<T> {
    fn new() -> Self {
        LinkedList {
            arena: Vec::new(),
            head: None,
            tail: None,
            len: 0,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Push to the back. Returns the index of the new node.
    fn push_back(&mut self, value: T) -> usize {
        let idx = self.arena.len();
        self.arena.push(Node {
            value,
            prev: self.tail,
            next: None,
        });

        if let Some(old_tail) = self.tail {
            self.arena[old_tail].next = Some(idx);
        } else {
            self.head = Some(idx);
        }
        self.tail = Some(idx);
        self.len += 1;
        idx
    }

    /// Push to the front. Returns the index of the new node.
    fn push_front(&mut self, value: T) -> usize {
        let idx = self.arena.len();
        self.arena.push(Node {
            value,
            prev: None,
            next: self.head,
        });

        if let Some(old_head) = self.head {
            self.arena[old_head].prev = Some(idx);
        } else {
            self.tail = Some(idx);
        }
        self.head = Some(idx);
        self.len += 1;
        idx
    }

    /// Insert a new node AFTER the node at `after_idx`. Returns new node's index.
    fn insert_after(&mut self, after_idx: usize, value: T) -> usize {
        let new_idx = self.arena.len();
        let next_of_after = self.arena[after_idx].next;

        self.arena.push(Node {
            value,
            prev: Some(after_idx),
            next: next_of_after,
        });

        self.arena[after_idx].next = Some(new_idx);

        if let Some(next_idx) = next_of_after {
            self.arena[next_idx].prev = Some(new_idx);
        } else {
            self.tail = Some(new_idx);
        }

        self.len += 1;
        new_idx
    }

    /// "Remove" a node by unlinking it. O(1).
    /// Note: the node stays in the arena (indices are stable).
    fn remove(&mut self, idx: usize) -> &T {
        let prev = self.arena[idx].prev;
        let next = self.arena[idx].next;

        match prev {
            Some(p) => self.arena[p].next = next,
            None => self.head = next,
        }
        match next {
            Some(n) => self.arena[n].prev = prev,
            None => self.tail = prev,
        }

        self.len -= 1;
        &self.arena[idx].value
    }

    /// Get a reference to the value at an index.
    fn get(&self, idx: usize) -> &T {
        &self.arena[idx].value
    }

    /// Iterate from head to tail.
    fn iter_forward(&self) -> Vec<&T> {
        let mut result = Vec::new();
        let mut current = self.head;
        while let Some(idx) = current {
            result.push(&self.arena[idx].value);
            current = self.arena[idx].next;
        }
        result
    }
}
```

Notice what we *didn't* use: no `Box`, no `Rc`, no `RefCell`, no `unsafe`. Just a `Vec` and indices. The arena pattern trades the ability to free individual nodes (they linger in the Vec) for simplicity and borrow-checker peace. For an undo history that's bounded in size, this trade-off is excellent.

## The Payoff: Undo/Redo History

Now let's build the undo/redo system on top of our linked list:

```rust
#[derive(Debug, Clone)]
enum Action {
    DeleteExercise { id: u32, name: String },
    EditExercise { id: u32, old_name: String, new_name: String },
    AddExercise { id: u32, name: String },
}

#[derive(Debug)]
struct UndoRedoHistory {
    list: LinkedList<Action>,
    cursor: Option<usize>, // index of the "current" action (last performed)
}

impl UndoRedoHistory {
    fn new() -> Self {
        UndoRedoHistory {
            list: LinkedList::new(),
            cursor: None,
        }
    }

    fn perform(&mut self, action: Action) {
        // If we're in the middle of history, we could keep or discard
        // the future. For simplicity, we append after the cursor.
        // (A branching version would keep the old future as an alternate path.)
        let idx = match self.cursor {
            None => self.list.push_front(action), // first action
            Some(c) => self.list.insert_after(c, action),
        };
        self.cursor = Some(idx);
    }

    fn undo(&mut self) -> Option<&Action> {
        let cursor = self.cursor?;
        let action = self.list.get(cursor);
        // Move cursor to previous node
        self.cursor = self.list.arena[cursor].prev;
        Some(action)
    }

    fn redo(&mut self) -> Option<&Action> {
        let next_idx = match self.cursor {
            Some(c) => self.list.arena[c].next?,
            None => self.list.head?, // redo from the very beginning
        };
        self.cursor = Some(next_idx);
        Some(self.list.get(next_idx))
    }

    fn can_undo(&self) -> bool {
        self.cursor.is_some()
    }

    fn can_redo(&self) -> bool {
        match self.cursor {
            Some(c) => self.list.arena[c].next.is_some(),
            None => self.list.head.is_some(),
        }
    }
}
```

Let's take it for a spin:

```rust
fn main() {
    let mut history = UndoRedoHistory::new();

    history.perform(Action::AddExercise { id: 1, name: "Back Squat".into() });
    history.perform(Action::AddExercise { id: 2, name: "Deadlift".into() });
    history.perform(Action::DeleteExercise { id: 1, name: "Back Squat".into() });

    println!("Can undo: {}", history.can_undo()); // true
    println!("Can redo: {}", history.can_redo()); // false

    if let Some(action) = history.undo() {
        println!("Undo: {:?}", action);
        // "Undo: DeleteExercise { id: 1, name: "Back Squat" }"
        // -> Restore Back Squat!
    }

    println!("Can redo: {}", history.can_redo()); // true

    if let Some(action) = history.redo() {
        println!("Redo: {:?}", action);
        // "Redo: DeleteExercise { id: 1, name: "Back Squat" }"
        // -> Delete Back Squat again
    }

    // Undo twice, then perform a new action
    history.undo();
    history.undo();
    history.perform(Action::EditExercise {
        id: 2,
        old_name: "Deadlift".into(),
        new_name: "Romanian Deadlift".into(),
    });
    println!("\nHistory after branch:");
    for action in history.list.iter_forward() {
        println!("  {:?}", action);
    }
}
```

## Complexity Comparison

| Operation | Vec + Cursor | Arena Linked List |
|---|---|---|
| Perform (append) | O(1) amortized | O(1) |
| Undo | O(1) | O(1) |
| Redo | O(1) | O(1) |
| Insert in middle | O(n) shift elements | O(1) relink |
| Truncate future | O(n) | O(1) just move cursor |
| Memory overhead | Low (contiguous) | Moderate (prev/next per node) |
| Cache performance | Excellent | Good (arena is contiguous) |

For pure undo/redo, the Vec approach is honestly fine — simpler and cache-friendlier. The linked list earns its keep when you need to splice, branch, or manipulate the middle of the history without shifting. The arena pattern gives you the linked list's flexibility with most of the Vec's cache friendliness, and *all* of Rust's safety guarantees.

The real lesson here isn't "linked lists are better than Vecs." It's that Rust forces you to think about ownership, and the arena pattern is a powerful tool for modeling any graph-like structure — undo histories today, workout dependency graphs tomorrow.

## Try It Yourself

1. **Add a `max_history` limit.** When the list exceeds N actions, drop the oldest one (remove from the head). This prevents unbounded memory growth during a long editing session.

2. **Implement `Display` for `UndoRedoHistory`** that shows the full history with an arrow `->` pointing at the current cursor position. Example: `[Add Squat] -> [Add Deadlift] -> [*Delete Squat*] -> [Edit DL]` where the asterisked item is the cursor.

3. **Branching history.** Instead of discarding future actions when performing a new action after undo, keep *both* branches. Store a `Vec<usize>` of "next" pointers per node instead of a single `next`. This turns your linked list into a tree of timelines — like `git` branches for your workout edits.
