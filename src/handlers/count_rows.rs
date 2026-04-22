use std::slice::Iter;

use crate::formatters::PercentageFormatter;

pub trait Count {
    fn count(&self) -> i64;
}

pub struct CountRows<T> {
    counts: Vec<T>,
    perc_factor: f64,
}

impl<T> CountRows<T> {
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }
}

impl<T> From<Vec<T>> for CountRows<T>
where
    T: Count,
{
    fn from(counts: Vec<T>) -> Self {
        #[allow(clippy::cast_precision_loss)]
        let total_count = counts.iter().map(|c| c.count() as f64).sum::<f64>();
        let perc_factor = if total_count > 0.0 {
            100.0 / total_count
        } else {
            0.0
        };

        Self {
            counts,
            perc_factor,
        }
    }
}

pub struct CountRowsIter<'a, T> {
    counts_iter: Iter<'a, T>,
    perc_factor: f64,
}

impl<'a, T> Iterator for CountRowsIter<'a, T>
where
    T: Count,
{
    type Item = (&'a T, PercentageFormatter);

    fn next(&mut self) -> Option<Self::Item> {
        #[allow(clippy::cast_precision_loss)]
        self.counts_iter.next().map(|path_count| {
            (
                path_count,
                PercentageFormatter(path_count.count() as f64 * self.perc_factor),
            )
        })
    }
}

impl<'a, T> IntoIterator for &'a CountRows<T>
where
    T: Count,
{
    type IntoIter = CountRowsIter<'a, T>;
    type Item = <Self::IntoIter as Iterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        CountRowsIter {
            counts_iter: self.counts.iter(),
            perc_factor: self.perc_factor,
        }
    }
}
