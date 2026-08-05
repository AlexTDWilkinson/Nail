//! Line diffs and patches - showing what changed, and replaying it.

/// The unified diff between two texts, the format git and code review read.
/// Two equal texts diff to an empty patch body.
pub fn lines(old: String, new: String) -> String {
    return diffy::create_patch(&old, &new).to_string();
}

/// Apply a patch diff_lines made (or git did) to a text, giving the new text.
/// A patch that does not fit the text says where it failed instead of guessing.
pub fn apply(text: String, patch: String) -> Result<String, String> {
    let parsed = diffy::Patch::from_str(&patch).map_err(|e| format!("diff_apply: this is not a unified diff: {}", e))?;
    if parsed.hunks().is_empty() {
        let only_headers = patch.lines().all(|line| line.trim().is_empty() || line.starts_with("--- ") || line.starts_with("+++ "));
        if !only_headers {
            return Err("diff_apply: this is not a unified diff - it has no @@ hunks".to_string());
        }
    }
    return diffy::apply(&text, &parsed).map_err(|e| format!("diff_apply: the patch does not fit this text: {}", e));
}

/// Whether two texts differ at all - cheaper to ask than to render the diff.
pub fn changed(old: String, new: String) -> bool {
    return old != new;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_diffs_and_the_diff_applies_back() {
        let old = "one\ntwo\nthree\n".to_string();
        let new = "one\n2\nthree\n".to_string();
        let patch = lines(old.clone(), new.clone());
        assert!(patch.contains("-two"));
        assert!(patch.contains("+2"));
        assert_eq!(apply(old.clone(), patch).unwrap(), new);
        assert!(changed(old.clone(), new));
        assert!(!changed(old.clone(), old));
    }

    #[test]
    fn a_patch_that_does_not_fit_says_so() {
        let patch = lines("one\ntwo\n".to_string(), "one\n2\n".to_string());
        assert!(apply("completely different text\n".to_string(), patch).unwrap_err().contains("does not fit"));
        assert!(apply("text".to_string(), "not a patch at all".to_string()).unwrap_err().contains("not a unified diff"));
    }
}
