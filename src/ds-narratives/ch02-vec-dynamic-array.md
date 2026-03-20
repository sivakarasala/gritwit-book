# Vec: The Dynamic Array -- "The Equipment Rack"

You've been using `Vec<Exercise>` since Chapter 1 like it's just a list. Push an exercise, pop one off, iterate. But what happens when you push the 9th exercise into a Vec that only has room for 8? The answer involves memory allocation, pointer arithmetic, and a growth strategy that computer scientists have argued about for decades. Let's crack open the Vec and build one from scratch -- because understanding your most-used collection means understanding how your app actually uses memory.

---

## What Vec Actually Is -- Three Numbers

A `Vec<T>` is not magic. Under the hood, it is exactly three values:

- **`ptr`** -- a raw pointer to a heap-allocated buffer
- **`len`** -- how many elements are actually stored right now
- **`capacity`** -- how many elements the buffer *can* hold before it needs to grow

Think of it as a gym's equipment rack. The rack has a fixed number of slots (capacity). Some slots hold dumbbells (len). The rest are empty, waiting. The pointer is the address of the rack on the gym floor.

When you write `let exercises: Vec<Exercise> = seed_exercises();` and that function returns 5 exercises, the Vec might look like this internally:

```
ptr ──> [Ex1][Ex2][Ex3][Ex4][Ex5][___][___][___]
         <────── len: 5 ──────>  <─ unused ─>
         <──────────── capacity: 8 ──────────>
```

Five exercises stored, room for three more before anything dramatic happens. You can verify this yourself:

```rust
fn main() {
    let mut exercises = Vec::new();
    exercises.push("Back Squat");
    exercises.push("Deadlift");
    exercises.push("Snatch");

    println!("len: {}, capacity: {}", exercises.len(), exercises.capacity());
    // len: 3, capacity: 4
}
```

The capacity is 4, not 3. Rust allocated room to grow. That extra slot is the empty space at the end of the rack -- free real estate for the next push.

---

## Build MyVec\<T\> from Scratch

Enough theory. Let's build a dynamic array from raw memory. This is the one place in the book where `unsafe` is necessary and educational -- you are doing what the standard library does for you behind the scenes.

```rust
use std::alloc::{alloc, dealloc, realloc, Layout};
use std::ptr;

pub struct MyVec<T> {
    ptr: *mut T,
    len: usize,
    capacity: usize,
}

impl<T> MyVec<T> {
    pub fn new() -> Self {
        MyVec {
            ptr: ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn push(&mut self, value: T) {
        if self.len == self.capacity {
            self.grow();
        }
        unsafe {
            // Write the value into the slot at index `len`
            ptr::write(self.ptr.add(self.len), value);
        }
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        unsafe {
            // Read the value out of the last occupied slot
            Some(ptr::read(self.ptr.add(self.len)))
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        unsafe { Some(&*self.ptr.add(index)) }
    }

    fn grow(&mut self) {
        let new_capacity = if self.capacity == 0 { 4 } else { self.capacity * 2 };
        let new_layout = Layout::array::<T>(new_capacity).expect("layout overflow");

        let new_ptr = if self.capacity == 0 {
            unsafe { alloc(new_layout) as *mut T }
        } else {
            let old_layout = Layout::array::<T>(self.capacity).expect("layout overflow");
            unsafe { realloc(self.ptr as *mut u8, old_layout, new_layout.size()) as *mut T }
        };

        if new_ptr.is_null() {
            panic!("allocation failed");
        }

        self.ptr = new_ptr;
        self.capacity = new_capacity;
    }
}

impl<T> Drop for MyVec<T> {
    fn drop(&mut self) {
        // Drop each element that is currently alive
        for i in 0..self.len {
            unsafe {
                ptr::drop_in_place(self.ptr.add(i));
            }
        }
        // Deallocate the buffer
        if self.capacity > 0 {
            let layout = Layout::array::<T>(self.capacity).expect("layout overflow");
            unsafe {
                dealloc(self.ptr as *mut u8, layout);
            }
        }
    }
}

fn main() {
    let mut v: MyVec<String> = MyVec::new();
    v.push("Back Squat".to_string());
    v.push("Deadlift".to_string());
    v.push("Clean & Jerk".to_string());

    println!("len: {}, capacity: {}", v.len(), v.capacity());
    // len: 3, capacity: 4

    if let Some(name) = v.get(1) {
        println!("Exercise at index 1: {}", name);
        // Exercise at index 1: Deadlift
    }

    if let Some(last) = v.pop() {
        println!("Popped: {}", last);
        // Popped: Clean & Jerk
    }

    println!("len after pop: {}", v.len());
    // len after pop: 2
}
```

That is about 80 lines for a fully working, generic, heap-allocated dynamic array. The `grow()` method is where the real action happens -- and it is where the next section gets interesting.

---

## The Growth Strategy -- Why Double?

When every slot on the equipment rack is full and a new dumbbell arrives, you need a bigger rack. The critical question: **how much bigger?**

### Strategy 1: Grow by one

The naive approach -- buy a rack with exactly one more slot. Every single push triggers a reallocation: allocate new memory, copy all elements, free the old memory.

If you push 1,000 exercises:
- Push 1: allocate for 1, copy 0 elements
- Push 2: allocate for 2, copy 1 element
- Push 3: allocate for 3, copy 2 elements
- ...
- Push 1,000: allocate for 1,000, copy 999 elements

Total copies: 0 + 1 + 2 + ... + 999 = **499,500 copies**. That is O(n^2) total work, or O(n) per push. You are at the hardware store every single week, moving every dumbbell every time.

### Strategy 2: Double the capacity

When the rack is full, buy one twice as big. Now pushes are free until the next doubling:

- Push 1-4: capacity 4, zero reallocations
- Push 5: reallocate to 8, copy 4 elements
- Push 9: reallocate to 16, copy 8 elements
- Push 17: reallocate to 32, copy 16 elements
- ...

For 1,000 exercises: you reallocate at sizes 4, 8, 16, 32, 64, 128, 256, 512, 1024. That is **~10 reallocations** instead of 999. Total elements copied: 4 + 8 + 16 + 32 + 64 + 128 + 256 + 512 = **1,020 copies**.

### The amortized analysis

Here is the proof that doubling gives O(1) amortized cost per push. After n pushes, the total copy work is:

```
1 + 2 + 4 + 8 + ... + n  =  2n - 1  =  O(n)
```

O(n) total work for n pushes means **O(1) per push on average**. Most pushes cost nothing (just write to the next slot). The occasional expensive reallocation is "paid for" by all the cheap pushes that preceded it.

The gym analogy lands here: buying a rack twice as big means you will not be back at the hardware store for months. The up-front cost of the bigger rack is spread across all the dumbbells you will add before the next trip.

> **What Rust actually does:** `Vec` starts with capacity 0 (no allocation at all). The first `push` allocates enough for a small number of elements. After that, it doubles. The exact initial size depends on the element type's size, but the doubling strategy is the important part.

---

## Reallocation In Action -- Watch It Happen

Let's watch our `MyVec` grow in real time. This code pushes exercises and prints the capacity at each step:

```rust
use std::alloc::{alloc, dealloc, realloc, Layout};
use std::ptr;

pub struct MyVec<T> {
    ptr: *mut T,
    len: usize,
    capacity: usize,
}

impl<T> MyVec<T> {
    pub fn new() -> Self {
        MyVec { ptr: ptr::null_mut(), len: 0, capacity: 0 }
    }
    pub fn len(&self) -> usize { self.len }
    pub fn capacity(&self) -> usize { self.capacity }

    pub fn push(&mut self, value: T) {
        if self.len == self.capacity { self.grow(); }
        unsafe { ptr::write(self.ptr.add(self.len), value); }
        self.len += 1;
    }

    fn grow(&mut self) {
        let new_cap = if self.capacity == 0 { 4 } else { self.capacity * 2 };
        let new_layout = Layout::array::<T>(new_cap).expect("layout overflow");
        let new_ptr = if self.capacity == 0 {
            unsafe { alloc(new_layout) as *mut T }
        } else {
            let old_layout = Layout::array::<T>(self.capacity).expect("layout overflow");
            unsafe { realloc(self.ptr as *mut u8, old_layout, new_layout.size()) as *mut T }
        };
        if new_ptr.is_null() { panic!("allocation failed"); }
        self.ptr = new_ptr;
        self.capacity = new_cap;
    }
}

impl<T> Drop for MyVec<T> {
    fn drop(&mut self) {
        for i in 0..self.len { unsafe { ptr::drop_in_place(self.ptr.add(i)); } }
        if self.capacity > 0 {
            let layout = Layout::array::<T>(self.capacity).expect("layout overflow");
            unsafe { dealloc(self.ptr as *mut u8, layout); }
        }
    }
}

fn main() {
    let exercises = [
        "Back Squat", "Deadlift", "Clean & Jerk", "Snatch",
        "Front Squat", "Pull-ups", "Handstand Push-ups", "Muscle-ups",
        "Box Jumps", "Burpees",
    ];

    let mut rack: MyVec<&str> = MyVec::new();
    for name in &exercises {
        let old_cap = rack.capacity();
        rack.push(name);
        let new_cap = rack.capacity();
        let marker = if new_cap != old_cap { " <-- REALLOCATED!" } else { "" };
        println!(
            "Push {:20} -> len: {:2}, capacity: {:2}{}",
            format!("\"{}\"", name), rack.len(), rack.capacity(), marker
        );
    }
}
```

Output:

```
Push "Back Squat"          -> len:  1, capacity:  4 <-- REALLOCATED!
Push "Deadlift"            -> len:  2, capacity:  4
Push "Clean & Jerk"        -> len:  3, capacity:  4
Push "Snatch"              -> len:  4, capacity:  4
Push "Front Squat"         -> len:  5, capacity:  8 <-- REALLOCATED!
Push "Pull-ups"            -> len:  6, capacity:  8
Push "Handstand Push-ups"  -> len:  7, capacity:  8
Push "Muscle-ups"          -> len:  8, capacity:  8
Push "Box Jumps"           -> len:  9, capacity: 16 <-- REALLOCATED!
Push "Burpees"             -> len: 10, capacity: 16
```

Three reallocations for ten pushes. The rack doubled from 0 to 4, then 4 to 8, then 8 to 16. Compare that with `Vec::with_capacity`, which pre-allocates the rack to the exact size you need:

```rust
fn main() {
    let mut exercises = Vec::with_capacity(10);
    println!("Before pushes: len={}, capacity={}", exercises.len(), exercises.capacity());
    // Before pushes: len=0, capacity=10

    for name in ["Back Squat", "Deadlift", "Clean & Jerk", "Snatch",
                  "Front Squat", "Pull-ups", "Handstand Push-ups",
                  "Muscle-ups", "Box Jumps", "Burpees"] {
        exercises.push(name);
    }

    println!("After pushes:  len={}, capacity={}", exercises.len(), exercises.capacity());
    // After pushes:  len=10, capacity=10
}
```

Zero reallocations. One allocation up front, done. When you know the size, pre-allocate the rack.

---

## Why This Matters for GrindIt

This is not academic. Every Vec operation in GrindIt has a cost, and now you understand what drives it.

**Loading exercises from the database.** When you fetch 500 exercises in Chapter 5, this is the difference between one allocation and nine:

```rust
// Bad: starts at capacity 0, reallocates ~9 times as it grows to 512+
let mut exercises = Vec::new();
for row in database_rows {
    exercises.push(row); // occasional O(n) reallocation hidden in here
}

// Good: one allocation, zero reallocations
let mut exercises = Vec::with_capacity(500);
for row in database_rows {
    exercises.push(row); // always O(1), guaranteed
}

// Best: collect already does this if the iterator knows its size
let exercises: Vec<Exercise> = database_rows.collect();
```

**Removing from the middle.** When a coach deletes an exercise from the library, `exercises.remove(index)` shifts every element after the deleted one to the left. Removing the first exercise from a 500-element Vec copies 499 elements. If order does not matter, `exercises.swap_remove(index)` swaps the last element into the gap -- O(1) instead of O(n).

```rust
// O(n) -- shifts everything left
exercises.remove(3);

// O(1) -- swaps last element into position 3, does not preserve order
exercises.swap_remove(3);
```

**Filtering exercises.** `retain` removes elements in place without allocating a new Vec:

```rust
// Keep only weightlifting exercises -- modifies in place, O(n)
exercises.retain(|ex| ex.category == "weightlifting");
```

**Bulk operations.** `drain` removes a range and returns an iterator over the removed elements. Useful for moving exercises between collections:

```rust
// Remove exercises 2..5 and collect them into a separate Vec
let removed: Vec<Exercise> = exercises.drain(2..5).collect();
```

**Iterator invalidation.** In C++, you can push to a vector while iterating and get undefined behavior -- the reallocation invalidates all your pointers. Rust prevents this at compile time. This code will not compile:

```rust,ignore
for ex in &exercises {
    if ex.category == "mobility" {
        exercises.push(another_exercise); // ERROR: cannot borrow as mutable
    }
}
```

The borrow checker sees that `&exercises` holds an immutable reference and `.push()` requires a mutable reference. Two references to the same data, one mutable -- Rust says no. This is not a limitation; it is the compiler catching a real bug that would crash a C++ program.

---

## Vec vs Other Collections -- When NOT to Use Vec

Vec is the right default. But it is not always the right choice.

| Need | Use | Why not Vec? |
|------|-----|-------------|
| Fast lookup by key | `HashMap` | Vec search is O(n) |
| Ordered unique set | `BTreeSet` | Vec allows duplicates, unsorted |
| Queue (FIFO) | `VecDeque` | `Vec::remove(0)` is O(n) |
| Frequent middle insertions | `LinkedList` (rare) | Vec shifts on insert |
| Stack (LIFO) | `Vec` | `push`/`pop` are O(1) -- Vec IS a stack |

That last row is important. Every time you use `push` and `pop`, you are using Vec as a stack. It is the most natural stack implementation in Rust, and you will never need a separate "Stack" type.

---

## Complexity Table

| Operation | Vec | MyVec | Notes |
|-----------|-----|-------|-------|
| `push` | O(1) amortized | O(1) amortized | Occasional O(n) realloc |
| `pop` | O(1) | O(1) | |
| `get(index)` | O(1) | O(1) | Direct pointer arithmetic |
| `insert(index)` | O(n) | -- | Shifts elements right |
| `remove(index)` | O(n) | -- | Shifts elements left |
| `contains` | O(n) | -- | Linear scan |
| `with_capacity` | O(1) | -- | One allocation up front |

The O(1) for `get` is what makes Vec powerful for random access. The pointer arithmetic is literally `ptr + index * size_of::<T>()` -- one multiplication and one addition, regardless of how many elements are in the Vec.

---

## Try It Yourself

Three exercises to extend your `MyVec`. Each one teaches a different aspect of how dynamic arrays really work.

### Exercise 1: `insert(index, value)`

Add an `insert` method that places a value at the given index, shifting all elements at and after that index one position to the right. Think about it: you are making room on the equipment rack by sliding every dumbbell from that slot onward one position to the right, then placing the new one in the gap.

```rust
impl<T> MyVec<T> {
    pub fn insert(&mut self, index: usize, value: T) {
        assert!(index <= self.len, "index out of bounds");
        if self.len == self.capacity {
            self.grow();
        }
        unsafe {
            // Shift elements [index..len] one position to the right
            ptr::copy(
                self.ptr.add(index),
                self.ptr.add(index + 1),
                self.len - index,
            );
            // Write the new value into the gap
            ptr::write(self.ptr.add(index), value);
        }
        self.len += 1;
    }
}
```

`ptr::copy` is Rust's equivalent of C's `memmove` -- it handles overlapping source and destination regions correctly. Using `ptr::copy_nonoverlapping` here would be a bug because the source and destination overlap.

### Exercise 2: `remove(index)`

Add a `remove` method that extracts the value at the given index and shifts everything after it one position to the left. The inverse of insert -- you are pulling a dumbbell off the rack and sliding everything after it to close the gap.

```rust
impl<T> MyVec<T> {
    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len, "index out of bounds");
        let value = unsafe {
            // Read the value out of the slot
            ptr::read(self.ptr.add(index))
        };
        unsafe {
            // Shift elements [index+1..len] one position to the left
            ptr::copy(
                self.ptr.add(index + 1),
                self.ptr.add(index),
                self.len - index - 1,
            );
        }
        self.len -= 1;
        value
    }
}
```

Notice the cost: removing from index 0 copies `len - 1` elements. This is why `Vec::remove(0)` is O(n) and why `VecDeque` exists for queue-like access patterns.

### Exercise 3: Implement `Iterator`

Create a `MyVecIter` struct that lets you write `for item in &my_vec`. This requires implementing the `Iterator` trait with a simple index counter.

```rust
pub struct MyVecIter<'a, T> {
    vec: &'a MyVec<T>,
    index: usize,
}

impl<T> MyVec<T> {
    pub fn iter(&self) -> MyVecIter<'_, T> {
        MyVecIter { vec: self, index: 0 }
    }
}

impl<'a, T> Iterator for MyVecIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.vec.len() {
            return None;
        }
        let item = self.vec.get(self.index);
        self.index += 1;
        item
    }
}
```

Now you can iterate:

```rust
fn main() {
    let mut v: MyVec<&str> = MyVec::new();
    v.push("Back Squat");
    v.push("Deadlift");
    v.push("Snatch");

    for exercise in v.iter() {
        println!("{}", exercise);
    }
}
```

The `'a` lifetime annotation tells Rust: "the references returned by this iterator cannot outlive the MyVec they came from." This is the borrow checker ensuring you cannot hold a reference to element 2 while the Vec gets dropped and its memory freed. Lifetimes are the compile-time equivalent of a gym's rule that you cannot take equipment home -- it stays on the rack.

---

## Recap

A `Vec` is three numbers: a pointer, a length, and a capacity. When length hits capacity, it doubles the buffer and copies everything over. That doubling strategy gives you amortized O(1) pushes -- the occasional expensive reallocation is paid for by all the cheap ones between doublings.

You built `MyVec<T>` from raw memory allocations and saw that there is no magic -- just pointer arithmetic, careful memory management, and a growth strategy. The standard library's `Vec` does the same thing with more edge-case handling, SIMD optimizations, and years of battle-testing.

The practical takeaway for GrindIt: when you know how many elements you need, use `Vec::with_capacity`. When you remove from the middle and order does not matter, use `swap_remove`. And when the compiler stops you from pushing to a Vec while iterating -- thank it. It just prevented a memory safety bug that would have been a segfault in C++.
