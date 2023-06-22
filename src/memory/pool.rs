use core::ops::Deref;
use core::ops::DerefMut;

use alloc::collections::BTreeMap;

use crate::bitmap::Bitmap;
use crate::label::Label;

use super::Error;

const PAGE_BITS: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_BITS;
const PAGE_MASK: usize = (1 << PAGE_BITS) - 1;

const PAGE_IS_FREE: bool  = false;
const PAGE_START:   usize = 0;


fn is_aligned_to_page(addr: usize) -> bool {
    (addr & PAGE_MASK) == 0
}

fn convert_page_index_to_addr(start: usize, index: usize) -> usize {
    assert!(is_aligned_to_page(start));
    start + index * PAGE_SIZE
}

fn convert_page_addr_to_index(start: usize, addr: usize) -> usize {
    assert!(addr >= start);
    assert!(is_aligned_to_page(start));
    assert!(is_aligned_to_page(addr));
    (addr - start) >> PAGE_BITS
}

pub struct PageMap {
    pool: Bitmap,
    start: usize,
    size: usize,
}

impl PageMap {
    pub fn new(start: usize, size: usize) -> Self {
        Self {
            pool: Bitmap::new(size / PAGE_SIZE), // heap alloc
            start,
            size,
        }
    }

    // Return the starting address of mutiple pages
    pub fn get_multiple(&mut self, count: usize) -> Result<usize, Error> {
        self.pool
            .find_and_flip(PAGE_START, count, PAGE_IS_FREE)
            .map(|index| convert_page_index_to_addr(self.start, index))
            .ok_or_else(|| Error::PageNotFound)
    }

    // Take in the starting address of mutiple pages
    pub fn put_multiple(&mut self, start: usize, count: usize) -> Result<(), Error> {
        let start_index = convert_page_addr_to_index(self.start, start);
        if self.pool.is_full(start_index, count) {
            self.pool.set_multiple(start_index, count, PAGE_IS_FREE);
            Ok(())
        } else {
            Err(Error::PageExists)
        }
    }
}

macro_rules! page_range {
    ($start:expr,$n:expr) => {
        ($start..($start+$n*PAGE_SIZE)).step_by(PAGE_SIZE)
    };
}

pub struct PageSet(BTreeMap<usize, bool>); // (addr, dirty bit)

impl PageSet {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn get_multiple(&mut self, count: usize) -> Result<usize, Error> {
        let start = self.0
            .keys()
            .find(|&&p| self.all(p, count, PAGE_IS_FREE))
            .map(|&p| p)
            .ok_or_else(|| Error::PageNotFound);
        start.map(|start| {
            page_range!(start, count)
                .for_each(|p| {
                    // assert!(self.0.contains_key(&i));
                    // self.0.entry(i).and_modify(|e| *e = !PAGE_IS_FREE);
                    self.0.get_mut(&p).map(|e| *e = !PAGE_IS_FREE).unwrap();
                });
            start
        })
    }

    pub fn put_mutiple(&mut self, start: usize, count: usize) -> Result<(), Error> {
        if !self.any(start, count, PAGE_IS_FREE) {
            page_range!(start, count)
                .for_each(|p| {
                    self.0.insert(p, PAGE_IS_FREE);
                });
            Ok(())
        } else {
            Err(Error::PageExists)
        }
    }

    // fn contains_multiple(&self, start: usize, count: usize) -> bool {
        // page_range!(start, count)
            // .all(|i| self.0.contains_key(&i))
    // }

    fn all(&self, start: usize, count: usize, value: bool) -> bool {
        page_range!(start, count)
            .all(|p| self.0.get_key_value(&p) == Some((&p, &value)))
    }

    fn any(&self, start: usize, count: usize, value: bool) -> bool {
        page_range!(start, count)
            .any(|p| self.0.get_key_value(&p) == Some((&p, &value)))
    }
}

pub struct LabeledPageSet {
    inner: PageSet,
    label: Label,
}

impl LabeledPageSet {
    pub fn new(label: Label) -> Self {
        Self {
            inner: PageSet::new(),
            label,
        }
    }

    pub fn borrow(&self) -> &PageSet {
        &self.inner
    }

    pub fn borrow_mut(&mut self) -> &mut PageSet {
        &mut self.inner
    }

    pub fn label(&self) -> Label {
        self.label.clone()
    }
}

impl Deref for LabeledPageSet {
    type Target = PageSet;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for LabeledPageSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
