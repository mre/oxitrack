use axum_ctx::{RespErr, Status};

use super::{contiguous_date_part::ContiguousDatePart, DataPoint};

pub struct ChartDataAggregator<D>
where
    D: ContiguousDatePart,
{
    inner: Vec<DataPoint<D>>,
    next_date_part: D,
}

impl<D> ChartDataAggregator<D>
where
    D: ContiguousDatePart,
{
    pub const MAX_LEN: usize = 60;

    pub fn new(next_date_part: D) -> Self {
        Self {
            inner: Vec::with_capacity(Self::MAX_LEN),
            next_date_part,
        }
    }

    pub fn into_inner(self) -> Vec<DataPoint<D>> {
        self.inner
    }

    pub fn next_date_part(&self) -> D {
        self.next_date_part
    }

    pub fn push(&mut self, count: u64) -> Result<(), RespErr> {
        if self.inner.len() == Self::MAX_LEN {
            return Err(RespErr::new(Status::Internal)
                .log_msg("The maximum length of ChartDataVec is exceeded!"));
        }

        self.inner.push(DataPoint {
            x: self.next_date_part,
            y: count,
        });
        self.next_date_part.next()?;

        Ok(())
    }
}
