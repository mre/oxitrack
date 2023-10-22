use std::slice::Iter;

pub trait Count {
    fn count(&self) -> i64;
}

pub struct CountRows<T> {
    counts: Vec<T>,
    mult_factor: f64,
}

impl<T> CountRows<T> {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }
}

impl<T> From<Vec<T>> for CountRows<T>
where
    T: Count,
{
    fn from(counts: Vec<T>) -> Self {
        let total_count = counts.iter().map(|c| c.count()).sum::<i64>();
        #[allow(clippy::cast_precision_loss)]
        let mult_factor = 100.0 / total_count as f64;

        Self {
            counts,
            mult_factor,
        }
    }
}

pub struct CountRowsIter<'a, T> {
    counts_iter: Iter<'a, T>,
    mult_factor: f64,
}

impl<'a, T> Iterator for CountRowsIter<'a, T>
where
    T: Count,
{
    type Item = (&'a T, f64);

    fn next(&mut self) -> Option<Self::Item> {
        #[allow(clippy::cast_precision_loss)]
        self.counts_iter
            .next()
            .map(|path_count| (path_count, path_count.count() as f64 * self.mult_factor))
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
            mult_factor: self.mult_factor,
        }
    }
}
