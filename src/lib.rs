//! Implementation of the Fuzzy Unified Diff Format

#[cfg(test)]
mod tests;

/// Error type for FuDiff
#[derive(Debug)]
pub enum Error {
    /// Failed to parse the diff format
    Parse(String),
    /// Failed to apply the patch
    Apply(String),
    /// Multiple possible matches found for context
    AmbiguousMatch(String),
}

/// Result type alias for diff operations
pub type Result<T> = std::result::Result<T, Error>;

/// Creates a diff between two strings.
pub fn diff(old: &str, new: &str) -> FuDiff {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let mut hunks = Vec::new();
    let mut current_hunk = Hunk {
        context_before: Vec::new(),
        deletions: Vec::new(),
        additions: Vec::new(),
        context_after: Vec::new(),
    };

    let mut i = 0;
    let mut j = 0;

    while i < old_lines.len() && j < new_lines.len() && old_lines[i] == new_lines[j] {
        current_hunk.context_before.push(old_lines[i].to_string());
        i += 1;
        j += 1;
    }

    while i < old_lines.len() || j < new_lines.len() {
        let look_ahead = 3;
        let mut next_match = None;

        // Look for nearest match within look_ahead window
        for offset in 0..=look_ahead {
            let _max_i = usize::min(i + offset, old_lines.len());
            let _max_j = usize::min(j + offset, new_lines.len());

            for di in 0..=offset {
                for dj in 0..=offset {
                    if i + di < old_lines.len()
                        && j + dj < new_lines.len()
                        && old_lines[i + di] == new_lines[j + dj]
                        && (di > 0 || dj > 0)
                    {
                        next_match = Some((di, dj));
                        break;
                    }
                }
                if next_match.is_some() {
                    break;
                }
            }
            if next_match.is_some() {
                break;
            }
        }

        match next_match {
            Some((di, dj)) => {
                // Add differing lines as changes
                current_hunk
                    .deletions
                    .extend(old_lines[i..i + di].iter().map(|s| s.to_string()));
                current_hunk
                    .additions
                    .extend(new_lines[j..j + dj].iter().map(|s| s.to_string()));
                i += di;
                j += dj;

                // Add matching context lines
                let mut matches = 0;
                while i + matches < old_lines.len()
                    && j + matches < new_lines.len()
                    && old_lines[i + matches] == new_lines[j + matches]
                    && matches < look_ahead
                {
                    current_hunk
                        .context_after
                        .push(old_lines[i + matches].to_string());
                    matches += 1;
                }
                i += matches;
                j += matches;

                // Finalize current hunk and start new one if needed
                if !current_hunk.deletions.is_empty() || !current_hunk.additions.is_empty() {
                    let mut new_hunk = Hunk {
                        context_before: Vec::new(),
                        deletions: Vec::new(),
                        additions: Vec::new(),
                        context_after: Vec::new(),
                    };
                    std::mem::swap(
                        &mut new_hunk.context_before,
                        &mut current_hunk.context_after,
                    );
                    hunks.push(current_hunk);
                    current_hunk = new_hunk;
                }
            }
            None => {
                // Add all remaining lines
                current_hunk
                    .deletions
                    .extend(old_lines[i..].iter().map(|s| s.to_string()));
                current_hunk
                    .additions
                    .extend(new_lines[j..].iter().map(|s| s.to_string()));
                break;
            }
        }
    }

    // Add final hunk if it contains changes
    if !current_hunk.deletions.is_empty() || !current_hunk.additions.is_empty() {
        hunks.push(current_hunk);
    }

    FuDiff { hunks }
}

/// Parse a fuzzy diff from a string.
pub fn parse(input: &str) -> Result<FuDiff> {
    let mut hunks = Vec::new();
    let mut current_hunk = None;

    // Empty input is valid for a diff with no changes
    if input.trim().is_empty() {
        return Ok(FuDiff { hunks: vec![] });
    }

    // Non-empty input must contain hunk markers
    if !input.contains("@@") {
        return Err(Error::Parse("No hunks found in diff".to_string()));
    }

    for line in input.lines() {
        if line.starts_with("@@") && line[2..].contains("@@") {
            // Finalize current hunk and start new one, ignoring text between @@ markers
            if let Some(hunk) = current_hunk.take() {
                hunks.push(hunk);
            }
            current_hunk = Some(Hunk {
                context_before: Vec::new(),
                deletions: Vec::new(),
                additions: Vec::new(),
                context_after: Vec::new(),
            });
            continue;
        }

        // Skip irrelevant lines
        if line.is_empty() || line.starts_with("---") || line.starts_with("+++") {
            continue;
        }

        // Require lines to be in a hunk context
        let hunk = current_hunk
            .as_mut()
            .ok_or_else(|| Error::Parse("Line found outside of hunk".to_string()))?;

        let (marker, content) = line.split_at(1);
        match marker {
            " " => {
                if hunk.deletions.is_empty() && hunk.additions.is_empty() {
                    hunk.context_before.push(content.to_string());
                } else {
                    hunk.context_after.push(content.to_string());
                }
            }
            "-" => hunk.deletions.push(content.to_string()),
            "+" => hunk.additions.push(content.to_string()),
            _ => return Err(Error::Parse(format!("Invalid line prefix: {}", marker))),
        }
    }

    // Capture final hunk if present
    if let Some(hunk) = current_hunk.take() {
        hunks.push(hunk);
    }

    Ok(FuDiff { hunks })
}

/// Represents a complete fuzzy diff
#[derive(Debug)]
pub struct FuDiff {
    hunks: Vec<Hunk>,
}

impl std::fmt::Display for FuDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.render())
    }
}

impl FuDiff {
    /// Reverts this diff from a string where it was previously applied.
    pub fn revert(&self, input: &str) -> Result<String> {
        // Create a new diff with swapped additions/deletions
        let reverted = FuDiff {
            hunks: self
                .hunks
                .iter()
                .map(|h| Hunk {
                    context_before: h.context_before.clone(),
                    deletions: h.additions.clone(),
                    additions: h.deletions.clone(),
                    context_after: h.context_after.clone(),
                })
                .collect(),
        };

        reverted.patch(input)
    }

    /// Applies this diff to the given input text, producing the patched result.
    /// Returns an error if the patch cannot be applied cleanly.
    pub fn patch(&self, input: &str) -> Result<String> {
        if self.hunks.is_empty() {
            return Ok(input.to_string());
        }

        let lines: Vec<&str> = input.lines().collect();
        // Allow empty input if we only have additions
        if lines.is_empty() && self.hunks.iter().any(|h| !h.deletions.is_empty()) {
            return Err(Error::Apply(
                "Cannot apply patch to empty input".to_string(),
            ));
        }

        let mut result = Vec::new();
        let mut pos = 0;

        for hunk in &self.hunks {
            // Find position in input to apply this hunk
            let hunk_pos = if hunk.context_before.is_empty() {
                pos
            } else {
                let mut found_pos = None;
                'outer: for i in pos..=lines.len().saturating_sub(hunk.context_before.len()) {
                    for (j, line) in hunk.context_before.iter().enumerate() {
                        if i + j >= lines.len() || lines[i + j] != line {
                            continue 'outer;
                        }
                    }
                    if found_pos.is_some() {
                        return Err(Error::AmbiguousMatch(format!(
                            "Multiple matches for context: {:?}",
                            hunk.context_before
                        )));
                    }
                    found_pos = Some(i);
                }
                found_pos.ok_or_else(|| {
                    Error::Apply(format!("Could not find context: {:?}", hunk.context_before))
                })?
            };

            // Verify deletions match
            let deletion_start = hunk_pos + hunk.context_before.len();
            if !hunk.deletions.is_empty() {
                if deletion_start + hunk.deletions.len() > lines.len() {
                    return Err(Error::Apply(
                        "Deletion extends past end of file".to_string(),
                    ));
                }
                for (i, deletion) in hunk.deletions.iter().enumerate() {
                    if lines[deletion_start + i] != deletion {
                        return Err(Error::Apply(format!(
                            "Deletion mismatch at line {} - expected '{}', found '{}'",
                            deletion_start + i + 1,
                            deletion,
                            lines[deletion_start + i]
                        )));
                    }
                }
            }

            // Copy unchanged lines up to this hunk
            if pos < hunk_pos {
                result.extend(lines[pos..hunk_pos].iter().map(|s| s.to_string()));
            }

            // Add the context lines from input and the new additions
            result.extend(
                hunk.context_before
                    .iter()
                    .enumerate()
                    .map(|(i, _)| lines[hunk_pos + i].to_string()),
            );
            result.extend(hunk.additions.iter().cloned());

            // Move past this hunk's changes
            pos = hunk_pos + hunk.context_before.len() + hunk.deletions.len();
        }

        // Copy remaining lines
        if pos < lines.len() {
            result.extend(lines[pos..].iter().map(|s| s.to_string()));
        }

        // Handle empty result case
        if result.is_empty() {
            return Ok(String::new());
        }

        // Join with newlines and handle trailing newline
        let mut output = result.join("\n");
        if !result.is_empty() {
            let has_input_newline = input.ends_with('\n');
            let mut has_output_newline = false;

            if let Some(last_hunk) = self.hunks.last() {
                if !last_hunk.context_after.is_empty() || !last_hunk.additions.is_empty() {
                    has_output_newline = has_input_newline;
                }
            } else {
                has_output_newline = has_input_newline;
            }

            if has_output_newline {
                output.push('\n');
            }
        }
        Ok(output)
    }

    /// Renders the diff back to the unified diff format.
    pub fn render(&self) -> String {
        let mut output = String::new();

        for (i, hunk) in self.hunks.iter().enumerate() {
            output.push_str("@@ @@\n");

            for line in &hunk.context_before {
                output.push(' ');
                output.push_str(line);
                output.push('\n');
            }

            for line in &hunk.deletions {
                output.push('-');
                output.push_str(line);
                output.push('\n');
            }

            for line in &hunk.additions {
                output.push('+');
                output.push_str(line);
                output.push('\n');
            }

            for (j, line) in hunk.context_after.iter().enumerate() {
                output.push(' ');
                output.push_str(line);
                if i < self.hunks.len() - 1 || j < hunk.context_after.len() - 1 {
                    output.push('\n');
                }
            }
        }

        output
    }
}

/// Represents a single hunk within a diff
#[derive(Debug, Clone, PartialEq)]
pub struct Hunk {
    pub context_before: Vec<String>,
    pub deletions: Vec<String>,
    pub additions: Vec<String>,
    pub context_after: Vec<String>,
}
