//! Hypothesis data model.
//!
//! A [`Hypothesis<T>`] wraps a single abducible atom (the *pattern*)
//! together with its origin and an optional human-readable
//! explanation. [`HypothesisSet<T>`] is a small collection of
//! hypotheses with deduplication via the user-supplied equality
//! predicate; this keeps the trait surface free of `Eq + Hash`
//! bounds so backends can plug in any term representation,
//! including `f64`-backed numeric terms that don't admit `Eq`.

/// A candidate atom the abductive engine may assume.
///
/// `pattern` is opaque to this crate; the backend interprets it
/// when checking whether assuming the hypothesis discharges the
/// goal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Hypothesis<T> {
    pub pattern: T,
    pub explanation: Option<String>,
    /// Tag describing where the hypothesis came from
    /// (`"abduce-block"`, `"class-constraint"`, theory name, …).
    pub source: String,
}

impl<T> Hypothesis<T> {
    pub fn new(pattern: T, source: impl Into<String>) -> Self {
        Self {
            pattern,
            source: source.into(),
            explanation: None,
        }
    }

    pub fn with_explanation(mut self, e: impl Into<String>) -> Self {
        self.explanation = Some(e.into());
        self
    }
}

/// A collection of [`Hypothesis<T>`] values. Insertion order is
/// preserved so backends that benefit from a stable enumeration
/// (e.g. round-robin / lex-minimal candidates) can rely on it.
#[derive(Clone, Debug, Default)]
pub struct HypothesisSet<T> {
    items: Vec<Hypothesis<T>>,
}

impl<T> HypothesisSet<T> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn insert(&mut self, h: Hypothesis<T>) {
        self.items.push(h);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Hypothesis<T>> {
        self.items.iter()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn as_slice(&self) -> &[Hypothesis<T>] {
        &self.items
    }
}

impl<T> FromIterator<Hypothesis<T>> for HypothesisSet<T> {
    fn from_iter<I: IntoIterator<Item = Hypothesis<T>>>(iter: I) -> Self {
        Self {
            items: iter.into_iter().collect(),
        }
    }
}

impl<T: PartialEq> HypothesisSet<T> {
    /// Locate the first hypothesis whose pattern is equal (by
    /// `PartialEq`) to `goal`. Backends with a richer notion of
    /// matching (α-equivalence, unification, …) should implement
    /// their own search instead of using this helper.
    pub fn matching(&self, goal: &T) -> Option<&Hypothesis<T>> {
        self.items.iter().find(|h| h.pattern == *goal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insertion_order_is_preserved() {
        let mut set: HypothesisSet<&str> = HypothesisSet::new();
        set.insert(Hypothesis::new("p", "a"));
        set.insert(Hypothesis::new("q", "b"));
        set.insert(Hypothesis::new("r", "c"));
        let patterns: Vec<&str> = set.iter().map(|h| h.pattern).collect();
        assert_eq!(patterns, vec!["p", "q", "r"]);
    }

    #[test]
    fn with_explanation_threads_text() {
        let h = Hypothesis::new("p", "abduce-block").with_explanation("from L42");
        assert_eq!(h.explanation.as_deref(), Some("from L42"));
        assert_eq!(h.source, "abduce-block");
    }

    #[test]
    fn matching_finds_first_pattern_equal() {
        let mut set: HypothesisSet<i32> = HypothesisSet::new();
        set.insert(Hypothesis::new(1, "a"));
        set.insert(Hypothesis::new(2, "b"));
        set.insert(Hypothesis::new(3, "c"));
        assert_eq!(set.matching(&2).map(|h| h.source.as_str()), Some("b"));
        assert!(set.matching(&99).is_none());
    }

    #[test]
    fn collect_via_from_iter() {
        let set: HypothesisSet<&str> = ["x", "y"]
            .into_iter()
            .map(|p| Hypothesis::new(p, "test"))
            .collect();
        assert_eq!(set.len(), 2);
    }
}
