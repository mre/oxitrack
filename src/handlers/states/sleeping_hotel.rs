use std::time::Instant;

pub type SleepingHotelInd = u16;
const MAX_N_BEDS: usize = SleepingHotelInd::MAX as usize + 1;

pub struct Bed<T> {
    sleeper: T,
    fell_asleep_at: Instant,
}

pub struct SleepingHotel<T> {
    last_ind: SleepingHotelInd,
    min_secs: u64,
    beds: Box<[Option<Bed<T>>; MAX_N_BEDS]>,
}

impl<T> SleepingHotel<T> {
    #[must_use]
    pub fn new(min_secs: u64) -> Self {
        let beds = (0..MAX_N_BEDS)
            .map(|_| None)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap_or_else(|_| panic!("Conversion into Box<[_, N]> should not fail!"));

        Self {
            last_ind: 0,
            min_secs,
            beds,
        }
    }

    #[must_use]
    pub fn reserve_bed(&mut self, sleeper: T) -> SleepingHotelInd {
        let registration = Bed {
            sleeper,
            fell_asleep_at: Instant::now(),
        };

        let ind = self.last_ind;

        // Safety: Valid index since `beds` has length `SleepingHotelInd::MAX + 1` with `SleepingHotelInd::MIN = 0`.
        *unsafe { self.beds.get_unchecked_mut(usize::from(ind)) } = Some(registration);
        self.last_ind = self.last_ind.wrapping_add(1);

        ind
    }

    /// Returns the sleeper if it was found in the bed and did sleep well.
    /// Tired sleepers are kicked out.
    #[must_use]
    pub fn wake_up(&mut self, bed_ind: SleepingHotelInd) -> Option<T> {
        // Safety: See `reserve_bed`.
        let bed = unsafe { self.beds.get_unchecked_mut(usize::from(bed_ind)) }.take()?;

        let elapsed = bed.fell_asleep_at.elapsed().as_secs();
        let slept_well = elapsed >= self.min_secs;

        slept_well.then_some(bed.sleeper)
    }
}
