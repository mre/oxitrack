use rand::{rng, seq::SliceRandom};
use std::{
    mem, process,
    sync::{Mutex, MutexGuard},
};
use time::OffsetDateTime;
use tracing::error;

pub type VisitorId = u16;
// +1 because the minimum index is 0 which has to be counted too.
const MAX_N_CONCURRENT_VISITORS: usize = VisitorId::MAX as usize + 1;

pub type VisitId = i64;
pub type PathId = i64;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SleepingState {
    pub path_id: PathId,
    pub registered_at: OffsetDateTime,
}

#[cfg(test)]
impl SleepingState {
    fn new(path_id: PathId) -> Self {
        Self {
            path_id,
            registered_at: OffsetDateTime::now_utc(),
        }
    }
}

#[derive(Default)]
enum VisitorState {
    #[default]
    None,
    Sleeping(SleepingState),
    PostSleep {
        visit_id: VisitId,
    },
}

struct VisitorStateStoreInner {
    last_id_ind: VisitorId,
    visitor_states: Box<[VisitorState; MAX_N_CONCURRENT_VISITORS]>,
}

impl VisitorStateStoreInner {
    #[must_use]
    fn get_mut(&mut self, id: VisitorId) -> &mut VisitorState {
        // Safety: Valid index since `visitors` has length `VisitorIdType::MAX + 1` with `VisitorIdType::MIN = 0`.
        unsafe { self.visitor_states.get_unchecked_mut(usize::from(id)) }
    }
}

pub struct VisitorStateStore {
    inner: Mutex<VisitorStateStoreInner>,
    ind_to_id_map: Box<[VisitorId; MAX_N_CONCURRENT_VISITORS]>,
    min_secs: i64,
}

impl VisitorStateStore {
    #[must_use]
    pub fn new(min_secs: u16) -> Self {
        let visitor_states = (0..MAX_N_CONCURRENT_VISITORS)
            .map(|_| VisitorState::None)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap_or_else(|_| panic!("Conversion into Box<[_, N]> should not fail!"));

        let inner = Mutex::new(VisitorStateStoreInner {
            last_id_ind: 0,
            visitor_states,
        });

        let mut ind_to_id_map: Box<[VisitorId; MAX_N_CONCURRENT_VISITORS]> = (VisitorId::MIN
            ..=VisitorId::MAX)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap_or_else(|_| panic!("Conversion into Box<[_, N]> should not fail!"));

        // No protection needed during development.
        if !cfg!(debug_assertions) {
            let mut rng = rng();
            ind_to_id_map.shuffle(&mut rng);
        }

        Self {
            inner,
            ind_to_id_map,
            min_secs: i64::from(min_secs),
        }
    }

    fn locked(&self) -> MutexGuard<'_, VisitorStateStoreInner> {
        self.inner.lock().unwrap_or_else(|_| {
            error!("Visitor state store mutex poisoned!");
            process::exit(1);
        })
    }

    #[must_use]
    pub fn register(&self, sleeping_state: SleepingState) -> VisitorId {
        let state = VisitorState::Sleeping(sleeping_state);

        let mut inner = self.locked();
        // Safety: Valid index since `ind_to_id_map` has length `VisitorIdType::MAX + 1` with `VisitorIdType::MIN = 0`.
        let id = *unsafe {
            self.ind_to_id_map
                .get_unchecked(usize::from(inner.last_id_ind))
        };

        *inner.get_mut(id) = state;
        inner.last_id_ind = inner.last_id_ind.wrapping_add(1);

        id
    }

    /// Returns the DB path ID and datetime of registration if the visitor
    /// waited at least the minimum delay.
    /// Returns `None` otherwise after clearing the visitor state.
    #[must_use]
    pub fn post_sleep(&self, visitor_id: VisitorId) -> Option<SleepingState> {
        let state = mem::take(self.locked().get_mut(visitor_id));

        let VisitorState::Sleeping(sleeping_state) = state else {
            return None;
        };

        let elapsed = (OffsetDateTime::now_utc() - sleeping_state.registered_at).whole_seconds();
        let slept_well = elapsed >= self.min_secs;

        slept_well.then_some(sleeping_state)
    }

    pub fn post_visit_insertion(&self, visitor_id: VisitorId, visit_id: VisitId) {
        *self.locked().get_mut(visitor_id) = VisitorState::PostSleep { visit_id };
    }

    /// Returns the DB visit ID if the visitor already successfully called `post_delay`.
    /// Returns `None` otherwise after clearing the visitor state.
    #[must_use]
    pub fn page_left(&self, visitor_id: VisitorId) -> Option<VisitId> {
        let state = mem::take(self.locked().get_mut(visitor_id));

        let VisitorState::PostSleep { visit_id } = state else {
            return None;
        };

        Some(visit_id)
    }
}

#[cfg(test)]
mod tests {
    use super::{SleepingState, VisitorId, VisitorStateStore};

    #[test]
    fn ids() {
        let store = VisitorStateStore::new(0);
        let sleeping_state = SleepingState::new(42);

        assert_eq!(store.register(sleeping_state.clone()), 0);
        assert_eq!(store.register(sleeping_state.clone()), 1);

        store.locked().last_id_ind = VisitorId::MAX;

        assert_eq!(store.register(sleeping_state.clone()), VisitorId::MAX);
        assert_eq!(store.register(sleeping_state), 0);
    }

    #[test]
    fn no_delay() {
        let store = VisitorStateStore::new(0);
        let sleeping_state = SleepingState::new(42);

        let id = store.register(sleeping_state.clone());

        assert_eq!(store.post_sleep(id), Some(sleeping_state));
        assert_eq!(store.post_sleep(id), None);
    }

    #[test]
    fn pre_min_delay() {
        let store = VisitorStateStore::new(100);
        let sleeping_state = SleepingState::new(42);

        let id = store.register(sleeping_state);

        assert_eq!(store.post_sleep(id), None);
    }

    #[test]
    fn post_min_delay() {
        let min_delay = 1;
        let store = VisitorStateStore::new(min_delay);
        let sleeping_state = SleepingState::new(42);

        let id = store.register(sleeping_state.clone());

        std::thread::sleep(std::time::Duration::new(u64::from(min_delay), 1));

        assert_eq!(store.post_sleep(id), Some(sleeping_state));
        assert_eq!(store.post_sleep(id), None);
    }

    #[test]
    fn page_left() {
        let store = VisitorStateStore::new(0);
        let sleeping_state = SleepingState::new(42);

        let visit_id = 13;
        let id = store.register(sleeping_state.clone());

        assert_eq!(store.post_sleep(id), Some(sleeping_state));
        store.post_visit_insertion(id, visit_id);
        assert_eq!(store.page_left(id), Some(visit_id));
    }

    #[test]
    fn no_post_visit_insertion() {
        let store = VisitorStateStore::new(0);
        let sleeping_state = SleepingState::new(42);

        let id = store.register(sleeping_state.clone());

        assert_eq!(store.post_sleep(id), Some(sleeping_state));
        assert_eq!(store.page_left(id), None);
    }
}
