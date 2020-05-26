/// A Trie of only ASCII characters
use crate::Punctuator;

pub(super) struct Trie {
    value: Option<Punctuator>,
    edges: Vec<Option<Box<Trie>>>,
}

impl Trie {
    pub(super) fn build(punctuators: &[Punctuator]) -> Trie {
        let mut trie = Trie::empty();
        for sym in punctuators {
            trie.add(*sym);
        }
        trie
    }

    pub(super) fn empty() -> Trie {
        Trie {
            value: None,
            edges: {
                let mut edges = Vec::new();
                for _ in 0..128 {
                    edges.push(None);
                }
                edges
            },
        }
    }

    fn add(&mut self, s: Punctuator) {
        self.add_helper(s.str(), s);
    }

    fn add_helper(&mut self, remaining: &str, original: Punctuator) {
        if remaining.len() == 0 {
            self.value = Some(original)
        } else {
            let ch = remaining.chars().next().unwrap() as usize;
            let remaining = &remaining[1..];
            if self.edges[ch].is_none() {
                self.edges[ch] = Some(Trie::empty().into())
            }
            self.edges[ch]
                .as_mut()
                .unwrap()
                .add_helper(remaining, original);
        }
    }

    /// Finds the longest matching subsequence of the given string
    pub(super) fn find(&self, mut s: &str) -> Option<Punctuator> {
        let mut curr = self;
        let mut best = curr.value;
        loop {
            best = curr.value.or(best);
            if s.is_empty() {
                break;
            }
            let ch = s.chars().next().unwrap();
            s = &s[ch.len_utf8()..];
            let ch = ch as usize;
            if ch >= 128 || curr.edges[ch].is_none() {
                break;
            }
            curr = &curr.edges[ch].as_ref().unwrap();
        }
        best
    }

    /// Checks to see if there's a punctuator that matches the given
    /// string exactly
    pub(super) fn find_exact(&self, s: &str) -> Option<Punctuator> {
        self.find(s).filter(|sym| sym.str().len() == s.len())
    }
}
