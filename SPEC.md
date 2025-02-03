# Fuzzy Unified Diff Format 

## Introduction

This specification describes an fuzzy unified diff format intended for systems
where AI models output patches to modify text files. It is based on the classic
unified diff format but has been modified to address common challenges:

- **Avoiding explicit line numbers:** AI models often have difficulty producing
  correct line indices.
- **Context-based matching:** The diff relies solely on unchanged context lines
  to locate the correct patch position.
- **Clear demarcation of changes:** Simple markers indicate removed and added
  lines.

This format is intended for text-based files (such as source code or
configuration files) and is designed for integration into patching tools
written in Rust or similar languages.

## Scope

This format applies to text files only; it is not intended for binary diffs.
The specification covers:

- File headers to identify the target file.
- Hunk headers that demarcate the start of a change block.
- Hunk bodies that contain context lines, deletions, and additions.

## Overall Structure

An FUDiff document consists of three main parts:

1. **Hunk Headers:** Mark the start of each change block.
2. **Hunk Bodies:** Contain the actual context lines and changes.

### Hunk Headers

Each hunk header is written in a simplified format that does not include line
numbers or any extra metadata. The header consists simply of a pair of `@@`
symbols:

```
@@ @@
```

**Rules:**

- The header must appear on a single line by itself.
- No line numbers or context hashes are provided. The patching tool relies
  solely on the context lines that follow to locate the correct patch position.

### Hunk Bodies

The hunk body is a sequence of lines that represent:
- **Context lines:** Unchanged lines (prefixed with a single space `" "`).
- **Deletions:** Lines removed from the original file (prefixed with a minus `"-"`).
- **Additions:** Lines added in the new file (prefixed with a plus `"+"`).

**General Format:**

```diff
@@ @@
<unchanged context lines>
-<line removed>
+<line added>
<unchanged context lines>
```

**Rules:**

- Every line in the hunk must begin with one of the three prefixes: a space, a minus, or a plus.
- Context lines (beginning with a space) serve to locate the patch insertion point via fuzzy matching.
- A deletion line indicates that the specified text must exist in the target file and will be removed.
- An addition line indicates that the given text will be inserted at that point.

## Detailed Syntax and Semantics

### Hunk Header Details

- The hunk header is always on a line by itself and appears as:
  
  ```
  @@ @@
  ```
  
- No additional identifiers, line numbers, or hashes are included.
- The patching tool uses the subsequent context lines for locating the correct section in the target file.

### Hunk Body Details

Each hunk body should ideally include a minimum of two context lines before and
after a change, although shorter context may be acceptable at the file
boundaries.

**Line Prefixes:**

- **Context line:**  
  Begins with a single space.  
  *Example:*
  ```
   fn main() {
  ```
- **Deletion line:**  
  Begins with a minus sign.  
  *Example:*
  ```
  -    println!("Hello, world!");
  ```
- **Addition line:**  
  Begins with a plus sign.  
  *Example:*
  ```
  +    println!("Goodbye, world!");
  ```

## AI-Focused Considerations

### Omission of Line Numbers

Traditional unified diffs include line numbers (e.g., `@@ -1,4 +1,4 @@`), but
these are omitted in this format to avoid AI errors. The patching tool locates
the patch area using the surrounding context.

### Fuzzy Matching via Context

Since exact line numbers are not provided:
- The library uses fuzzy matching algorithms to locate the block of context
  lines in the target file.
- The matching allows for minor differences such as whitespace changes.
- A minimum of 2–3 context lines before and after the change is recommended to
  reduce ambiguity.
- If multiple matches are found, the library will return an error indicating
  ambiguous matches.

### Handling Special Characters

- Lines with special characters (e.g., `@`, `#`, `<`, `>`) are treated as
  literal text.
- No additional escaping is required beyond the first character, which serves
  as the marker.
- If a file’s content includes the marker characters at the start of a line, a
  pre-processing step (or an escaping mechanism) must be applied before
  generating the diff.

### Multi-Hunk and Overlapping Changes

- Multiple hunks may appear sequentially in one diff file. They should be
  processed in order.
- If hunks are adjacent or overlap, the patching engine should merge them if
  possible. Otherwise, it should report a conflict and request manual
  resolution.
- In cases where context lines are sparse (e.g., changes near the beginning or
  end of a file), the engine should allow “partial” context matching.

## Examples

### Simple Single-Line Replacement

Suppose you have a file `example.rs` with the following content:

```rust
fn main() {
    println!("Hello, world!");
}
```

To update the greeting, the diff would be:

```diff
--- filename: example.rs
+++ filename: example.rs
@@ @@
  fn main() {
-    println!("Hello, world!");
+    println!("Goodbye, world!");
  }
```

### Multi-Line Changes with Context

The format supports multiple consecutive line removals and additions. Here are
examples of common multi-line change patterns:

**Original file (`compute.rs`):**

```rust
fn compute(x: i32) -> i32 {
    let y = x * 2;
    println!("Value: {}", y);
    println!("Debug info:");
    println!("  - multiplier: 2");
    println!("  - input: {}", x);
    y
}
```

**Example 1:** Replace multiple consecutive lines:

```diff
--- filename: compute.rs
+++ filename: compute.rs
@@ @@
  fn compute(x: i32) -> i32 {
-    let y = x * 2;
-    println!("Value: {}", y);
-    println!("Debug info:");
-    println!("  - multiplier: 2");
-    println!("  - input: {}", x);
+    let y = x + 10;
+    println!("Result: {}", y);
+    println!("Input was: {}", x);
     y
  }
```

**Example 2:** Replace some lines and add more:

```diff
--- filename: compute.rs
+++ filename: compute.rs
@@ @@
  fn compute(x: i32) -> i32 {
     let y = x * 2;
-    println!("Value: {}", y);
-    println!("Debug info:");
+    println!("Computing result...");
+    println!("Step 1: Initialize");
+    println!("Step 2: Multiply");
+    println!("Step 3: Validate");
     println!("  - multiplier: 2");
     println!("  - input: {}", x);
     y
  }
```

### Multiple Hunks in a Single Patch

When multiple changes are needed in different parts of a file, multiple hunks
can be included in a single patch. Each hunk is processed in order from top to
bottom.

**Original file (`config.rs`):**

```rust
use std::path::Path;

struct Config {
    path: String,
    timeout: u32,
    retries: u8,
}

impl Config {
    fn new(path: &str) -> Self {
        Config {
            path: path.to_string(),
            timeout: 30,
            retries: 3,
        }
    }

    fn validate(&self) -> bool {
        Path::new(&self.path).exists()
    }
}
```

**Patch with multiple hunks:**

```diff
--- filename: config.rs
+++ filename: config.rs
@@ @@
 use std::path::Path;
+use std::time::Duration;
 
 struct Config {
     path: String,
-    timeout: u32,
+    timeout: Duration,
     retries: u8,
 }
@@ @@
     fn new(path: &str) -> Self {
         Config {
             path: path.to_string(),
-            timeout: 30,
+            timeout: Duration::from_secs(30),
             retries: 3,
         }
     }
@@ @@
     fn validate(&self) -> bool {
-        Path::new(&self.path).exists()
+        let path = Path::new(&self.path);
+        path.exists() && path.is_file()
     }
```

This patch shows three separate hunks:
1. Adding an import and changing a type
2. Updating the constructor to use the new Duration type
3. Enhancing the validation logic

### Changes at File Boundaries

Special consideration is needed for changes at the beginning or end of files. These
cases require clear rules for context matching.

#### Beginning of File

When adding content at the start of a file, use the first few lines as context:

**Original file (`main.rs`):**
```rust
fn main() {
    println!("Hello!");
}
```

**Diff adding a header:**
```diff
--- filename: main.rs
+++ filename: main.rs
@@ @@
+// Copyright 2024 Example Corp.
+// Licensed under MIT
 fn main() {
     println!("Hello!");
}
```

#### End of File

When changing content at the end of a file, use the last few lines as context:

**Original file (`config.rs`):**
```rust
fn process() {
    // ... processing code
}
// End of implementation
```

**Diff adding a footer:**
```diff
--- filename: config.rs
+++ filename: config.rs
@@ @@
fn process() {
    // ... processing code
}
-// End of implementation
+// End of implementation
+
+#[cfg(test)]
+mod tests {
+    // Tests go here
+}
```

The implementation must handle these boundary cases by recognizing when context
lines appear at file boundaries and adjusting matching accordingly.

## Implementation Details

### Core Data Structures

```rust
#[derive(Debug)]
pub struct FuzzyDiff {
    original_file: String,
    modified_file: String,
    hunks: Vec<Hunk>,
}

#[derive(Debug)]
pub struct Hunk {
    context_before: Vec<String>,
    deletions: Vec<String>,
    additions: Vec<String>,
    context_after: Vec<String>,
}

#[derive(Debug)]
pub enum PatchError {
    AmbiguousContext(String),
    InvalidHunk(String),
    NoMatch,
    MultipleMatches(Vec<usize>),
    // ... other error variants
}
```

### Key Implementation Components

1. **Parser Implementation:**
```rust
impl FuzzyDiff {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        let mut parser = DiffParser::new(input);
        parser.parse_headers()?;
        
        let mut hunks = Vec::new();
        while let Some(hunk) = parser.next_hunk()? {
            hunks.push(hunk);
        }
        
        // Validate hunks don't overlap
        Self::validate_hunk_ordering(&hunks)?;
        
        Ok(FuzzyDiff {
            original_file: parser.original_file,
            modified_file: parser.modified_file,
            hunks,
        })
    }
}
```

2. **Context Matching:**
```rust
impl Hunk {
    pub fn find_match(&self, content: &str) -> Result<usize, PatchError> {
        let context = self.context_before
            .iter()
            .chain(self.context_after.iter())
            .collect::<Vec<_>>();
            
        let matches = find_context_matches(content, &context)?;
        
        match matches.len() {
            0 => Err(PatchError::NoMatch),
            1 => Ok(matches[0]),
            _ => Err(PatchError::MultipleMatches(matches)),
        }
    }
}
```

3. **Patch Application:**
```rust
impl FuzzyDiff {
    pub fn apply(&self, content: &str) -> Result<String, PatchError> {
        let mut result = content.to_string();
        let mut offset = 0;
        
        for hunk in &self.hunks {
            let pos = hunk.find_match(&result)?;
            
            // Apply deletions
            for line in &hunk.deletions {
                if !result[pos..].contains(line) {
                    return Err(PatchError::DeletedLineNotFound);
                }
            }
            
            // Apply additions
            let new_content = hunk.additions.join("\n");
            result.replace_range(
                pos..pos + hunk.deletions.len(),
                &new_content
            );
            
            offset += new_content.len() - hunk.deletions.len();
        }
        
        Ok(result)
    }
}
```

### Fuzzy Matching Strategy

```rust
fn find_context_matches(content: &str, context: &[&str]) -> Result<Vec<usize>> {
    let lines: Vec<&str> = content.lines().collect();
    let mut matches = Vec::new();
    
    // Sliding window over the content
    'outer: for (i, window) in lines.windows(context.len()).enumerate() {
        for (ctx_line, content_line) in context.iter().zip(window) {
            if !lines_match(ctx_line, content_line) {
                continue 'outer;
            }
        }
        matches.push(i);
    }
    
    Ok(matches)
}

fn lines_match(a: &str, b: &str) -> bool {
    // Implement fuzzy matching logic here
    // Consider whitespace normalization
    // Consider partial matches for long lines
    // Consider case sensitivity options
    todo!()
}
```

### Error Handling 

```rust
impl FuzzyDiff {
    fn validate_hunk_ordering(hunks: &[Hunk]) -> Result<(), PatchError> {
        for window in hunks.windows(2) {
            if let [h1, h2] = window {
                if h1.overlaps(h2) {
                    return Err(PatchError::OverlappingHunks);
                }
            }
        }
        Ok(())
    }
    
    fn handle_ambiguous_match(
        &self,
        hunk: &Hunk,
        matches: Vec<usize>
    ) -> Result<usize, PatchError> {
        // Implement resolution strategy:
        // 1. Check surrounding context
        // 2. Use heuristics based on position
        // 3. Prompt user if configured
        todo!()
    }
}
```

## Corner Cases and Error Handling

### Ambiguous Context

- **Scenario:** The same block of context appears in multiple locations.
- **Behavior:** The library returns an `AmbiguousContext` error indicating
  multiple possible match locations.

### Incomplete Hunk Bodies

- **Scenario:** A hunk is missing either a deletion or addition line.
- **Behavior:** A hunk with only context lines is treated as a no-op (useful for
  verification). A hunk with only deletions or only additions (outside of
  file-boundary insertions) returns an `InvalidHunk` error.

### Handling Trailing Newlines

- **Scenario:** Differences in trailing newlines could cause mismatches.
- **Behavior:**  
  - The library normalizes trailing newlines before matching context.
  - Adding or removing a trailing newline is represented by an empty line with
    the appropriate prefix.

### Lines Starting with Marker Characters

- **Scenario:** A file line begins with a space, `-`, or `+` naturally.
- **Behavior:**  
  - The first character in each hunk line is reserved for its marker.
  - The library escapes such lines before diff generation and unescapes them
    when applying the patch.

### Overlapping and Adjacent Hunks

- **Scenario:** Two hunks are adjacent or have overlapping context.
- **Behavior:**  
  - The library merges adjacent hunks if their contexts overlap.
  - If merging is not possible, it returns an `OverlappingHunks` error.
