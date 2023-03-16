use alloc::collections::BTreeMap;

use crate::bitmap::Bitmap;

use super::PAGE_SIZE;
use super::Error;

const PAGE_IS_FREE: bool  = false;
const PAGE_START:   usize = 0;

struct PagedPool {
    pool: Bitmap,
    start: usize,
    size: usize,
}

impl PagedPool {
    pub fn new(start: usize, size: usize) -> Self {
        Self {
            pool: Bitmap::new(size / PAGE_SIZE), // heap alloc
            start,
            size,
        }
    }

    pub fn get_multiple_pages(&mut self, count: usize) -> Result<usize, Error> {
        self.pool
            .find_and_flip(PAGE_START, count, PAGE_IS_FREE)
            .ok_or_else(|| Error::PageNotFound)
    }

    pub fn put_multiple_pages(&mut self, start: usize, count: usize) -> Result<(), Error> {
        if self.pool.is_full(start, count) {
            self.pool.set_multiple(start, count, PAGE_IS_FREE);
            Ok(())
        } else {
            Err(Error::PageExists)
        }
    }
}

struct PageSet {
    inner: BTreeMap<usize, bool>,
}

impl PageSet {
    pub fn new() -> Self {
        Self { inner: BTreeMap::new() }
    }

    pub fn push_mutiple_pages(&mut self, start: usize, count: usize) -> Result<(), Error> {
        if !self.contains_multiple_pages(start, count) {
            (start..(start+count))
                .for_each(|i| {
                    self.inner.insert(i, PAGE_IS_FREE);
                });
            Ok(())
        } else {
            Err(Error::PageExists)
        }
    }

    pub fn contains_multiple_pages(&mut self, start: usize, count: usize) -> bool {
        (start..(start+count))
            .all(|i| self.inner.contains_key(&i))
    }
}

