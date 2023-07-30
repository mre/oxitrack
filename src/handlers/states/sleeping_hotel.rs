use std::time::Instant;

pub type SleepingHotelInd = u16;
const MAX_IND: usize = SleepingHotelInd::MAX as usize;

pub struct Bed<T> {
    sleeper: T,
    fell_asleep_at: Instant,
}

pub struct SleepingHotel<T, const MIN_SECS: u64, const MAX_SECS: u64> {
    last_ind: SleepingHotelInd,
    beds: Box<[Option<Bed<T>>; MAX_IND]>,
}

impl<T, const MIN_SECS: u64, const MAX_SECS: u64> Default for SleepingHotel<T, MIN_SECS, MAX_SECS> {
    fn default() -> Self {
        let beds = (0..MAX_IND)
            .map(|_| None)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap_or_else(|_| panic!("Conversion into Box<[_, N]> should not fail!"));

        Self { last_ind: 0, beds }
    }
}

impl<T, const MIN_SECS: u64, const MAX_SECS: u64> SleepingHotel<T, MIN_SECS, MAX_SECS> {
    pub fn reserve_bed(&mut self, sleeper: T) -> SleepingHotelInd {
        let registration = Bed {
            sleeper,
            fell_asleep_at: Instant::now(),
        };

        let ind = self.last_ind;

        // Safety: Valid index since `beds` has length Ind::MAX.
        *unsafe { self.beds.get_unchecked_mut(usize::from(ind)) } = Some(registration);

        self.last_ind = self.last_ind.wrapping_add(1);

        ind
    }

    /// Returns the sleeper if it was found in the bed and did sleep well.
    /// Tired sleepers are kicked out.
    pub fn wake_up(&mut self, bed_ind: SleepingHotelInd) -> Option<T> {
        // Safety: Valid index since `beds` has length Ind::MAX.
        let bed = unsafe { self.beds.get_unchecked_mut(usize::from(bed_ind)) }.take()?;

        let elapsed = bed.fell_asleep_at.elapsed().as_secs();
        let slept_well = elapsed <= MAX_SECS && elapsed >= MIN_SECS;

        slept_well.then_some(bed.sleeper)
    }
}
