use std::{cmp::Ordering, collections::BTreeMap};
use serde::{Deserialize, Serialize};

pub trait Clock: PartialOrd + Clone + Send + Sync + 'static {
    fn reduce(&self) -> LamportClock;
}

pub type LamportClock = u32;

impl Clock for LamportClock {
    fn reduce(&self) -> LamportClock {
        *self
    }
}

/// clock key_id 
pub type KeyId = u64;

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Default, derive_more::Deref, Serialize, Deserialize,
)]
pub struct OrdinaryClock(pub BTreeMap<KeyId, u32>);

impl AsRef<OrdinaryClock> for OrdinaryClock {
    fn as_ref(&self) -> &OrdinaryClock {
        self
    }
}

impl OrdinaryClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_genesis(&self) -> bool {
        self.0.values().all(|n| *n == 0)
    }

    fn merge(&self, other: &Self) -> Self {
        let merged = self
            .0
            .keys()
            .chain(other.0.keys())
            .map(|id| {
                let n = match (self.0.get(id), other.0.get(id)) {
                    (Some(n), Some(other_n)) => (*n).max(*other_n),
                    (Some(n), None) | (None, Some(n)) => *n,
                    (None, None) => unreachable!(),
                };
                (*id, n)
            })
            .collect();
        Self(merged)
    }

    pub fn update<'a>(&'a self, others: impl Iterator<Item = &'a Self>, id: u64) -> Self {
        let mut updated = others.fold(self.clone(), |version, dep| version.merge(dep));
        *updated.0.entry(id).or_default() += 1;
        updated
    }
}

impl PartialOrd for OrdinaryClock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        fn ge(clock: &OrdinaryClock, other_clock: &OrdinaryClock) -> bool {
            for (other_id, other_n) in &other_clock.0 {
                if *other_n == 0 {
                    continue;
                }
                let Some(n) = clock.0.get(other_id) else {
                    return false;
                };
                if n < other_n {
                    return false;
                }
            }
            true
        }
        match (ge(self, other), ge(other, self)) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => None,
        }
    }
}

impl OrdinaryClock {
    pub fn dep_cmp(&self, other: &Self, id: KeyId) -> Ordering {
        match (self.0.get(&id), other.0.get(&id)) {
            // disabling this check after the definition of genesis clock has been extended
            // haven't revealed any bug with this assertion before, hopefully disabling it will not
            // hide any bug in the future as well
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            // this can happen on the startup insertion
            (None, None) => Ordering::Equal,
            (Some(n), Some(m)) => n.cmp(m),
        }
    }
}

impl Clock for OrdinaryClock {
    fn reduce(&self) -> LamportClock {
        self.0.values().copied().sum()
    }
}