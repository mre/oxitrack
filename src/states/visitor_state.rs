use std::{
    mem, process,
    sync::{Mutex, MutexGuard},
    time::Instant,
};

use tracing::error;

pub type VisitorId = u16;
// +1 because the minimum index is 0 which has to be counted too.
const MAX_N_CONCURRENT_VISITORS: usize = VisitorId::MAX as usize + 1;

pub type VisitId = i64;
pub type PathId = i64;

#[derive(Default)]
enum VisitorState {
    #[default]
    None,
    Sleeping {
        path_id: PathId,
        registerd_at: Instant,
    },
    PostSleep {
        visit_id: VisitId,
    },
}

struct VisitorStateStoreInner {
    last_id: VisitorId,
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
    min_secs: u64,
}

impl VisitorStateStore {
    #[must_use]
    pub fn new(min_secs: u64) -> Self {
        let visitors = (0..MAX_N_CONCURRENT_VISITORS)
            .map(|_| VisitorState::None)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap_or_else(|_| panic!("Conversion into Box<[_, N]> should not fail!"));

        Self {
            min_secs,
            inner: Mutex::new(VisitorStateStoreInner {
                last_id: 0,
                visitor_states: visitors,
            }),
        }
    }

    fn locked(&self) -> MutexGuard<VisitorStateStoreInner> {
        self.inner.lock().unwrap_or_else(|_| {
            error!("Visitor state store mutex poisoned!");
            process::exit(1);
        })
    }

    #[must_use]
    pub fn register(&self, path_id: PathId) -> VisitorId {
        let state = VisitorState::Sleeping {
            path_id,
            registerd_at: Instant::now(),
        };

        let mut inner = self.locked();
        let id = inner.last_id;

        *inner.get_mut(id) = state;
        inner.last_id = inner.last_id.wrapping_add(1);

        id
    }

    /// Returns the DB path ID if the visitor waited at least the minimum delay.
    /// Returns `None` otherwise after clearing the visitor state.
    #[must_use]
    pub fn post_sleep(&self, visitor_id: VisitorId) -> Option<PathId> {
        let state = mem::take(self.locked().get_mut(visitor_id));

        let VisitorState::Sleeping {
            path_id,
            registerd_at,
        } = state
        else {
            return None;
        };

        let elapsed = registerd_at.elapsed().as_secs();
        let slept_well = elapsed >= self.min_secs;

        slept_well.then_some(path_id)
    }

    pub fn post_visit_insertion(&self, visitor_id: VisitorId, visit_id: VisitId) {
        *self.locked().get_mut(visitor_id) = VisitorState::PostSleep { visit_id };
    }

    /// Returns the DB visit ID if the visitor already successfully called `post_delay`.
    /// Returns `None` otherwise after clearning the visitor state.
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
    use super::{VisitorId, VisitorStateStore};

    #[test]
    fn ids() {
        let store = VisitorStateStore::new(0);

        assert_eq!(store.register(42), 0);
        assert_eq!(store.register(42), 1);

        store.locked().last_id = VisitorId::MAX;

        assert_eq!(store.register(42), VisitorId::MAX);
        assert_eq!(store.register(42), 0);
    }

    #[test]
    fn no_delay() {
        let store = VisitorStateStore::new(0);

        let path_id = 42;
        let id = store.register(path_id);

        assert_eq!(store.post_sleep(id), Some(path_id));
        assert_eq!(store.post_sleep(id), None);
    }

    #[test]
    fn pre_min_delay() {
        let store = VisitorStateStore::new(100);

        let path_id = 42;
        let id = store.register(path_id);

        assert_eq!(store.post_sleep(id), None);
    }

    #[test]
    fn post_min_delay() {
        let min_delay = 1;
        let store = VisitorStateStore::new(min_delay);

        let path_id = 42;
        let id = store.register(path_id);

        std::thread::sleep(std::time::Duration::new(min_delay, 1));

        assert_eq!(store.post_sleep(id), Some(path_id));
        assert_eq!(store.post_sleep(id), None);
    }

    #[test]
    fn page_left() {
        let store = VisitorStateStore::new(0);

        let path_id = 42;
        let visit_id = 13;
        let id = store.register(path_id);

        assert_eq!(store.post_sleep(id), Some(path_id));
        store.post_visit_insertion(id, visit_id);
        assert_eq!(store.page_left(id), Some(visit_id));
    }

    #[test]
    fn no_post_visit_insertion() {
        let store = VisitorStateStore::new(0);

        let path_id = 42;
        let id = store.register(path_id);

        assert_eq!(store.post_sleep(id), Some(path_id));
        assert_eq!(store.page_left(id), None);
    }
}
