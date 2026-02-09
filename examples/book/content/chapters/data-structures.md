+++
title = "Data Structures"
weight = 2
template = "book.html"
+++

# Data Structures

Rust's standard library provides a rich set of collection types. Choosing the right one depends on your access patterns, ordering requirements, and performance constraints.

## Vec — The Workhorse

`Vec<T>` is a growable array and the most commonly used collection:

```rust
let mut numbers = vec![1, 2, 3, 4, 5];

numbers.push(6);
numbers.extend([7, 8, 9]);

let sum: i32 = numbers.iter().sum();
println!("Sum: {sum}");

let evens: Vec<_> = numbers.iter().filter(|number| *number % 2 == 0).collect();
println!("Evens: {evens:?}");
```

### Pre-allocating Capacity

When you know the approximate size in advance, pre-allocate to avoid repeated reallocations:

```rust
let mut buffer = Vec::with_capacity(1024);
for index in 0..1024 {
    buffer.push(index);
}
```

## HashMap — Key-Value Pairs

`HashMap<K, V>` provides O(1) average-case lookups:

```rust
use std::collections::HashMap;

let mut scores: HashMap<String, u32> = HashMap::new();

scores.insert("Alice".to_string(), 95);
scores.insert("Bob".to_string(), 87);
scores.insert("Charlie".to_string(), 92);

if let Some(score) = scores.get("Alice") {
    println!("Alice scored {score}");
}

scores.entry("Dave".to_string()).or_insert(0);
```

### Counting Occurrences

A common pattern for counting items:

```rust
use std::collections::HashMap;

fn word_frequencies(text: &str) -> HashMap<&str, usize> {
    let mut frequencies = HashMap::new();
    for word in text.split_whitespace() {
        *frequencies.entry(word).or_insert(0) += 1;
    }
    frequencies
}
```

## BTreeMap — Sorted Keys

When you need keys in sorted order, use `BTreeMap`:

```rust
use std::collections::BTreeMap;

let mut timeline = BTreeMap::new();
timeline.insert(2015, "Rust 1.0 released");
timeline.insert(2018, "Rust 2018 edition");
timeline.insert(2021, "Rust 2021 edition");

for (year, event) in &timeline {
    println!("{year}: {event}");
}

let recent = timeline.range(2018..);
```

## HashSet — Unique Values

`HashSet<T>` stores unique values with O(1) membership testing:

```rust
use std::collections::HashSet;

let languages: HashSet<&str> = ["rust", "go", "python", "rust"].into_iter().collect();
assert_eq!(languages.len(), 3);
assert!(languages.contains("rust"));

let systems: HashSet<&str> = ["rust", "c", "cpp"].into_iter().collect();

let both = languages.intersection(&systems).collect::<Vec<_>>();
println!("In both sets: {both:?}");
```

## VecDeque — Double-Ended Queue

`VecDeque<T>` supports efficient push and pop at both ends:

```rust
use std::collections::VecDeque;

let mut queue = VecDeque::new();

queue.push_back("first");
queue.push_back("second");
queue.push_front("zeroth");

while let Some(item) = queue.pop_front() {
    println!("Processing: {item}");
}
```

## Choosing the Right Collection

| Collection | Use When |
|-----------|----------|
| `Vec<T>` | Ordered sequence, frequent iteration |
| `HashMap<K, V>` | Fast key-value lookup |
| `BTreeMap<K, V>` | Sorted key-value pairs, range queries |
| `HashSet<T>` | Unique values, membership testing |
| `VecDeque<T>` | Queue or stack behavior |
| `BinaryHeap<T>` | Priority queue |
