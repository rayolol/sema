# Understanding Iterators & Filtering in Sema

This guide explains the iterator patterns used throughout the sema codebase.

## 1. What is an Iterator?

An iterator is a way to loop through a collection **one item at a time**, without copying everything.

```rust
// The old way (copies/clones data)
let items = vec![1, 2, 3];
for item in items.clone() {
    println!("{}", item);
}

// The iterator way (no copies, efficient)
let items = vec![1, 2, 3];
for item in items.iter() {
    println!("{}", item);
}
```

**Key insight:** Iterators are **lazy** — they only process items when you ask for them.

```rust
let numbers = vec![1, 2, 3, 4, 5];

// This does NOTHING yet (lazy)
let doubled = numbers.iter().map(|x| x * 2);

// This forces the iterator to run (eager)
let result: Vec<_> = doubled.collect();
// Now result = [2, 4, 6, 8, 10]
```

---

## 2. Filter: Keep Only What You Want

`.filter()` keeps only items that match a condition.

```rust
let numbers = vec![1, 2, 3, 4, 5];

// Keep only even numbers
let evens: Vec<_> = numbers
    .iter()
    .filter(|x| *x % 2 == 0)  // condition: is even?
    .collect();

// evens = [2, 4]
```

**Breaking down the closure:**
```rust
|x| *x % 2 == 0
 ↑  ↑         ↑
 |  |         return true/false
 |  dereference x (it's &i32, so *x gets the i32)
 parameter name
```

**In your code:**
```rust
// From query.rs
self.filter(move |item| item.attrs().iter().any(|a| a.path().is_ident(name)))
       ↑                                                                    ↑
    filter   returns true if ANY attribute matches this name
```

---

## 3. Map: Transform Each Item

`.map()` changes each item into something else.

```rust
let numbers = vec![1, 2, 3];

// Square each number
let squared: Vec<_> = numbers
    .iter()
    .map(|x| x * x)  // transform: x → x²
    .collect();

// squared = [1, 4, 9]
```

**Combining filter + map:**
```rust
let numbers = vec![1, 2, 3, 4, 5];

let result: Vec<_> = numbers
    .iter()
    .filter(|x| *x % 2 == 0)  // keep: [2, 4]
    .map(|x| x * 10)           // transform: [20, 40]
    .collect();
```

**In your code:**
```rust
// From bridge.rs
.map(|n| n.as_str().to_string())
    ↑
    transform Name → String
```

---

## 4. Filter_map: Filter AND Transform

`.filter_map()` does both in one step — keeps only items that transform successfully.

```rust
let items = vec!["1", "two", "3", "four"];

// Parse to numbers, skip failures
let numbers: Vec<i32> = items
    .iter()
    .filter_map(|s| s.parse::<i32>().ok())  // parse, keep only Ok
    .collect();

// numbers = [1, 3]  ("two" and "four" were skipped)
```

**In your code:**
```rust
// From bridge.rs
let structs: Vec<ResolvedStruct> = raw
    .structs
    .into_iter()
    .filter_map(|s| convert_struct(s, db, vfs))  // convert, skip None
    .collect();
```

---

## 5. Box<dyn Iterator>: The Trait Object

`Box<dyn Iterator>` is a **stored iterator of unknown type**.

**Why?** Iterators have complex types:

```rust
// This is what the type REALLY is:
let iter = vec![1, 2, 3]
    .iter()
    .filter(|x| x % 2 == 0);

// Type is something like:
// Filter<Iter<Vec<i32>>, fn(&i32) -> bool>
// (very complex, implementation detail)
```

**Solution:** Use a trait object:

```rust
// Store the iterator without knowing its exact type
let iter: Box<dyn Iterator<Item = &i32>> = Box::new(
    vec![1, 2, 3]
        .iter()
        .filter(|x| *x % 2 == 0)
);

// Now you can store it and use it later
```

**In your code:**
```rust
pub struct Query<'a, T> {
    items: Box<dyn Iterator<Item = &'a T> + 'a>,
    //     ↑↑↑
    //     "store any iterator type, I don't care which"
}

impl<'a, T: SemaItem> Query<'a, T> {
    pub fn new(iter: impl Iterator<Item = &'a T> + 'a) -> Self {
        Query {
            items: Box::new(iter),  // wrap it in a Box
        }
    }
}
```

---

## 6. Closures: Functions on the Fly

A closure is an anonymous function, often used with iterators.

```rust
// Function (named)
fn is_even(x: &i32) -> bool {
    *x % 2 == 0
}

// Closure (unnamed, inline)
let is_even = |x: &i32| *x % 2 == 0;

// With iterator
numbers.iter().filter(|x| *x % 2 == 0)
              //    ↑              ↑
              //    closure (inline function)
```

**Closure rules:**
```rust
|x| x * 2           // parameter → body
 ↑  ↑
 |  what to do with x
 where x comes from
```

**Capturing variables:**
```rust
let factor = 2;

let multiply = |x| x * factor;  // captures 'factor' from outside
let result = (0..5)
    .map(multiply)
    .collect::<Vec<_>>();
// [0, 2, 4, 6, 8]
```

**In your code:**
```rust
pub fn filter(self, f: impl Fn(&T) -> bool + 'a) -> Self {
    Query::new(self.items.filter(move |item| f(item)))
                          //     ↑         ↑
                          //     closure   calls f() with item
}
```

---

## 7. The 'move' Keyword

`move` means the closure **takes ownership** of captured variables.

```rust
let name = "Alice".to_string();

// Without move: borrows 'name'
let greet = || println!("Hi {}", name);
greet();
greet();
println!("{}", name);  // Still usable!

// With move: takes ownership
let greet = move || println!("Hi {}", name);
greet();
// name is now owned by greet, can't use it here
```

**In your code:**
```rust
pub fn with_attribute(self, name: &'a str) -> Self {
    self.filter(move |item| {
        item.attrs().iter().any(|a| a.path().is_ident(name))
    })
    //   ↑
    //   move: closure takes ownership of 'name'
}
```

---

## 8. Chaining: The Real Power

Iterators chain together elegantly:

```rust
let users = vec![
    ("Alice", 25),
    ("Bob", 30),
    ("Charlie", 22),
];

let result: Vec<&str> = users
    .iter()
    // Filter: keep people over 25
    .filter(|(_, age)| *age > 25)
    // Map: extract just the name
    .map(|(name, _)| *name)
    // Collect into a Vec
    .collect();

// result = ["Alice", "Bob"]
```

**In your code:**
```rust
let structs: Vec<ResolvedStruct> = raw
    .structs
    .into_iter()
    .filter_map(|s| convert_struct(s, db, vfs))  // transform, skip None
    .collect();

// Step by step:
// 1. Take ownership of raw.structs (into_iter)
// 2. For each struct s, try to convert it
// 3. Keep only successful conversions (filter_map)
// 4. Collect all results into a Vec
```

---

## 9. Common Patterns in Sema

### Pattern 1: Filter and Collect
```rust
pub fn named(self, name: &'a str) -> Self {
    self.filter(move |item| item.name() == name)
}

// Usage:
let motors = workspace.structs().named("Motor").collect();
//                                              ↑
//                                         force evaluation
```

### Pattern 2: Query Building (Fluent API)
```rust
workspace
    .structs()                           // Iterator<ResolvedStruct>
    .public()                            // filter: only public
    .in_module("motor")                  // filter: only in "motor" module
    .named_matching(|n| n.starts_with("Motor"))  // filter: name pattern
    .collect()                           // make it a Vec
```

### Pattern 3: Transform with filter_map
```rust
pub fn methods<'a>(&self, workspace: &'a Workspace) -> Vec<MethodInfo> {
    self.impls(workspace)
        .iter()
        .flat_map(|impl_| {
            impl_.node.items.iter().filter_map(|item| {
                if let syn::ImplItem::Fn(method) = item {
                    Some(MethodInfo {
                        name: method.sig.ident.to_string(),
                        params: method.sig.inputs.len(),
                        is_async: method.sig.asyncness.is_some(),
                        is_unsafe: method.sig.unsafety.is_some(),
                    })
                } else {
                    None
                }
            })
        })
        .collect()
}
```

---

## 10. Debugging Iterators

**Problem:** "What's actually in this iterator?"

**Solution:** Add `.inspect()` to see each item:

```rust
let result: Vec<_> = numbers
    .iter()
    .filter(|x| *x % 2 == 0)
    .inspect(|x| println!("Keeping: {}", x))  // debug: see what passes
    .map(|x| x * 10)
    .inspect(|x| println!("After map: {}", x))  // debug: see the result
    .collect();
```

**Output:**
```
Keeping: 2
After map: 20
Keeping: 4
After map: 40
```

---

## 11. Summary: The Flow

```
Original Data (Vec)
        ↓
    .iter()  ← iterate without copying
        ↓
  .filter()  ← keep only what matches
        ↓
   .map()    ← transform each item
        ↓
.collect()   ← materialize into a Vec
        ↓
   Result
```

**Key points:**
- ✅ Iterators are **lazy** (don't run until collected)
- ✅ `.filter()` keeps items matching a condition
- ✅ `.map()` transforms each item
- ✅ `.filter_map()` does both
- ✅ Closures are inline functions
- ✅ `move` transfers ownership into the closure
- ✅ `Box<dyn Iterator>` stores any iterator type
- ✅ Chain operations for clean code

---

## Practice Exercise

Try to understand this from your code:

```rust
impl<'a, T: SemaItem> Query<'a, T> {
    pub fn filter(self, f: impl Fn(&T) -> bool + 'a) -> Self {
        Query::new(self.items.filter(move |item| f(item)))
    }

    pub fn collect(self) -> Vec<&'a T> {
        self.items.collect()
    }
}
```

**What's happening:**
1. `self.items` is a `Box<dyn Iterator<Item = &'a T>>`
2. `.filter(move |item| f(item))` adds a filter layer
3. Returns a new `Query` wrapping the filtered iterator
4. `.collect()` forces evaluation and returns a Vec

This pattern lets you **chain multiple filters** without evaluating until the end!
