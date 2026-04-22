use axum_ctx::{RespErr, RespResult, StatusCode};

use super::{DataPoint, contiguous_date_part::ContiguousDatePart};

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
    pub const MAX_LEN: usize = 400;

    pub fn new(next_date_part: D) -> Self {
        Self {
            inner: Vec::with_capacity(Self::MAX_LEN),
            next_date_part,
        }
    }

    pub fn into_inner(self) -> Vec<DataPoint<D>> {
        self.inner
    }

    pub const fn next_date_part(&self) -> D {
        self.next_date_part
    }

    pub fn push(&mut self, count: u64) -> RespResult<()> {
        if self.inner.len() == Self::MAX_LEN {
            return Err(RespErr::new(StatusCode::INTERNAL_SERVER_ERROR)
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
