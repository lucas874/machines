use crate::types::typescript_types::EventType;

// Struct representation of an unordered pair of event types
// Works by always assigning the smallest element to a.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct UnordEventPair {
    a: EventType,
    b: EventType,
}

impl UnordEventPair {
    pub fn new(a: EventType, b: EventType) -> Self {
        if a <= b {
            Self { a, b }
        } else {
            Self { a: b, b: a }
        }
    }

    pub fn iter(&'_ self) -> UnordEventPairIterator<'_> {
        UnordEventPairIterator {
            unordered_event_pair: self,
            index: 0,
        }
    }
}

// https://stackoverflow.com/questions/30218886/how-to-implement-iterator-and-intoiterator-for-a-simple-struct
// https://dev.to/wrongbyte/implementing-iterator-and-intoiterator-in-rust-3nio
pub struct UnordEventPairIterator<'a> {
    unordered_event_pair: &'a UnordEventPair,
    index: usize,
}

impl<'a> Iterator for UnordEventPairIterator<'a> {
    type Item = &'a EventType;

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.index {
            0 => &self.unordered_event_pair.a,
            1 if self.unordered_event_pair.a != self.unordered_event_pair.b => {
                &self.unordered_event_pair.b
            }
            _ => return None,
        };

        self.index += 1;
        Some(result)
    }
}

pub struct UnordEventPairIntoIterator {
    unordered_event_pair: UnordEventPair,
    index: usize,
}


impl IntoIterator for UnordEventPair {
    type Item = EventType;
    type IntoIter = UnordEventPairIntoIterator;

    fn into_iter(self) -> Self::IntoIter {
        UnordEventPairIntoIterator {
            unordered_event_pair: self,
            index: 0,
        }
    }
}

impl Iterator for UnordEventPairIntoIterator {
    type Item = EventType;

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.index {
            0 => &self.unordered_event_pair.a,
            1 if self.unordered_event_pair.a != self.unordered_event_pair.b => {
                &self.unordered_event_pair.b
            }
            _ => return None,
        };

        self.index += 1;
        Some(result.clone())
    }
}

// Similar tests as https://github.com/myelin-ai/unordered-pair/blob/main/src/lib.rs
#[cfg(test)]
mod tests {
    use std::hash::{Hash, Hasher};

    use super::*;

    #[test]
    fn partial_eq_different_internal_order() {
        let pair = UnordEventPair::new(EventType::new("a"), EventType::new("b"));
        let reversed_pair = UnordEventPair::new(EventType::new("b"), EventType::new("a"));

        assert_eq!(pair, reversed_pair);

        let pair = UnordEventPair::new(EventType::new("b"), EventType::new("a"));
        let reversed_pair = UnordEventPair::new(EventType::new("a"), EventType::new("b"));

        assert_eq!(pair, reversed_pair);
    }

    #[test]
    fn partial_eq_same_internal_order() {
        let pair1 = UnordEventPair::new(EventType::new("a"), EventType::new("b"));
        let pair2 = UnordEventPair::new(EventType::new("a"), EventType::new("b"));
        assert_eq!(pair1, pair2);
    }

    #[test]
    fn neq() {
        let pair1 = UnordEventPair::new(EventType::new("c"), EventType::new("b"));
        let pair2 = UnordEventPair::new(EventType::new("a"), EventType::new("b"));
        assert_ne!(pair1, pair2);

        let pair1 = UnordEventPair::new(EventType::new("a"), EventType::new("c"));
        let pair2 = UnordEventPair::new(EventType::new("a"), EventType::new("b"));
        assert_ne!(pair1, pair2);
    }

    #[test]
    fn hash_different_internal_order() {
        use std::collections::hash_map::DefaultHasher as Hasher;

        let hash_pair = {
            let pair = UnordEventPair::new(EventType::new("a"), EventType::new("b"));
            let mut hasher = Hasher::new();
            pair.hash(&mut hasher);
            hasher.finish()
        };

        let hash_rev = {
            let pair = UnordEventPair::new(EventType::new("b"), EventType::new("a"));
            let mut hasher = Hasher::new();
            pair.hash(&mut hasher);
            hasher.finish()
        };

        assert_eq!(hash_rev, hash_pair);
    }

    #[test]
    fn hash_neq() {
        use std::collections::hash_map::DefaultHasher as Hasher;

        let hash_pair = {
            let pair = UnordEventPair::new(EventType::new("c"), EventType::new("b"));
            let mut hasher = Hasher::new();
            pair.hash(&mut hasher);
            hasher.finish()
        };

        let hash_rev = {
            let pair = UnordEventPair::new(EventType::new("a"), EventType::new("b"));
            let mut hasher = Hasher::new();
            pair.hash(&mut hasher);
            hasher.finish()
        };

        assert_ne!(hash_rev, hash_pair);
        let hash_pair = {
            let pair = UnordEventPair::new(EventType::new("a"), EventType::new("c"));
            let mut hasher = Hasher::new();
            pair.hash(&mut hasher);
            hasher.finish()
        };

        let hash_rev = {
            let pair = UnordEventPair::new(EventType::new("a"), EventType::new("b"));
            let mut hasher = Hasher::new();
            pair.hash(&mut hasher);
            hasher.finish()
        };

        assert_ne!(hash_rev, hash_pair);
    }

    #[test]
    fn test_iter() {
        let pair = UnordEventPair::new(EventType::new("a"), EventType::new("b"));

        let a: Vec<&EventType> = pair.iter().filter(|event_type| **event_type == EventType::new("a")).collect();

        assert_eq!(a.len(), 1);
        assert_eq!(a.first().cloned().unwrap(), &EventType::new("a"));
    }

    #[test]
    fn test_into_iter() {
        let pair = UnordEventPair::new(EventType::new("a"), EventType::new("b"));

        let a: Vec<EventType> = pair.into_iter().filter(|event_type| *event_type == EventType::new("a")).collect();

        assert_eq!(a.len(), 1);
        assert_eq!(a.first().cloned().unwrap(), EventType::new("a"));
    }
}
