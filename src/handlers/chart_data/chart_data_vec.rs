use axum_ctx::{RespErr, Status};
use serde::Serialize;

use super::DataPoint;

pub struct ChartDataVec<T>(Vec<DataPoint<T>>)
where
    T: Serialize;

impl<T> ChartDataVec<T>
where
    T: Serialize,
{
    pub const MAX_LEN: usize = 60;

    pub fn into_inner(self) -> Vec<DataPoint<T>> {
        self.0
    }

    pub fn push(&mut self, value: DataPoint<T>) -> Result<(), RespErr> {
        if self.0.len() == Self::MAX_LEN {
            return Err(RespErr::new(Status::Internal)
                .log_msg("The maximum length of ChartDataVec is exceeded!"));
        }

        self.0.push(value);

        Ok(())
    }
}

impl<T> Default for ChartDataVec<T>
where
    T: Serialize,
{
    fn default() -> Self {
        Self(Vec::with_capacity(Self::MAX_LEN))
    }
}
