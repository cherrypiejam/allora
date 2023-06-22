//! Heap alloc bitmap

#![allow(dead_code)]

use core::alloc::Allocator;
use alloc::{vec::Vec, vec};
use alloc::alloc::Global;

// pub struct Bitmap<const N: usize> {
    // bits: [usize; N],
// }

type Item = usize;

const ITEM_BITS: usize = Item::BITS as usize;

#[derive(Debug)]
pub struct Bitmap<A: Allocator = Global> {
    bit_count: usize,
    bits: Vec<Item, A>,
}

impl Bitmap {
    pub fn new(bit_count: usize) -> Self {
        Self {
            bit_count,
            bits: vec![0; Self::item_count(bit_count)]
        }
    }

    pub fn new_in<A: Allocator>(bit_count: usize, alloc: A) -> Bitmap<A> {
        let item_count = Self::item_count(bit_count);
        let mut bits = Vec::with_capacity_in(item_count, alloc);
        bits.resize(item_count, 0);
        Bitmap {
            bit_count,
            bits,
        }
    }

    pub fn len(&self) -> usize {
        self.bit_count
    }

    pub fn set(&mut self, bit_index: usize, value: bool) {
        if value {
            self.mark(bit_index);
        } else {
            self.reset(bit_index);
        }
    }

    pub fn set_multiple(&mut self, start: usize, count: usize, value: bool) {
        self.bound_check(start, count);
        (start..(start+count))
            .for_each(|i| self.set(i, value))
    }

    pub fn find_and_flip(&mut self, start: usize, count: usize, value: bool) -> Option<usize> {
        self.find(start, count, value)
            .map(|found| {
                (found..(found+count))
                    .for_each(|i| self.flip(i));
                found
            })
    }

    pub fn get(&self, bit_index: usize) -> bool {
        self.bits[Self::item_index(bit_index)] & Self::bit_mask(bit_index) != 0
    }

    pub fn is_full(&self, start: usize, count: usize) -> bool  {
        self.all(start, count, true)
    }

    pub fn is_none(&self, start: usize, count: usize) -> bool  {
        self.all(start, count, false)
    }

    fn find(&self, start: usize, count: usize, value: bool) -> Option<usize> {
        (start..(self.bit_count-(count-1)))
            .find(|&i| self.all(i, count, value))
    }

    // fn any(&self, start: usize, count: usize, value: bool) -> bool {
        // self.bound_check(start, count);
        // (start..(start + count))
            // .any(|i| self.get(i) == value)
    // }

    fn all(&self, start: usize, count: usize, value: bool) -> bool {
        self.bound_check(start, count);
        (start..(start+count))
            .all(|i| self.get(i) == value)
    }

    fn bound_check(&self, start: usize, count: usize) {
        assert!(start <= self.bit_count);
        assert!(start + count <= self.bit_count);
    }

    fn mark(&mut self, bit_index: usize) {
        self.bits[Self::item_index(bit_index)] |= Self::bit_mask(bit_index);
    }

    fn reset(&mut self, bit_index: usize) {
        self.bits[Self::item_index(bit_index)] &= !Self::bit_mask(bit_index);
    }

    fn flip(&mut self, bit_index: usize) {
        self.bits[Self::item_index(bit_index)] ^= Self::bit_mask(bit_index);
    }

    fn item_count(bit_count: usize) -> usize {
        (bit_count + (ITEM_BITS - 1)) / ITEM_BITS // may panic
    }

    fn item_index(bit_index: usize) -> usize {
        bit_index / ITEM_BITS
    }

    fn bit_mask(bit_index: usize) -> usize {
        1 << (bit_index % ITEM_BITS)
    }
}

impl PartialEq for Bitmap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() == other.len() {
            (0..self.len())
                .all(|i| self.get(i) == other.get(i))
        } else {
            false
        }
    }
}

impl Eq for Bitmap {}

impl From<&[bool]> for Bitmap {
    fn from(value: &[bool]) -> Self {
        let mut bitmap = Self::new(value.len());
        for (i, &e) in value.iter().enumerate() {
            bitmap.set(i, e);
        }
        bitmap
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test_case]
    fn test_bitmap_create() {
        let bitmap = Bitmap::new(3);
        assert_eq!(bitmap, Bitmap::from(&[false, false, false][..]));
        assert_ne!(bitmap, Bitmap::from(&[false, false, false, false][..]));
        assert_ne!(bitmap, Bitmap::from(&[false, false][..]));
        assert_ne!(bitmap, Bitmap::from(&[false, false, true][..]));
    }

    #[test_case]
    fn test_bitmap_single_bit_operation() {
        let mut bitmap = Bitmap::new(3);
        bitmap.set(1, true);
        assert_eq!(bitmap, Bitmap::from(&[false, true, false][..]));

        bitmap.set(1, false);
        assert_eq!(bitmap, Bitmap::from(&[false, false, false][..]));

        bitmap.flip(0);
        assert_eq!(bitmap, Bitmap::from(&[true, false, false][..]));

        bitmap.flip(0);
        assert_eq!(bitmap, Bitmap::from(&[false, false, false][..]));
    }

    #[test_case]
    fn test_bitmap_multiple_bits_operation() {
        let mut bitmap = Bitmap::new(3);
        bitmap.set_multiple(0, 3, true);
        assert_eq!(bitmap, Bitmap::from(&[true, true, true][..]));
        assert!(bitmap.is_full(0, 3));

        assert_eq!(bitmap.find_and_flip(0, 3, false), None);
        assert_eq!(bitmap.find_and_flip(0, 3, true), Some(0));
        assert_eq!(bitmap, Bitmap::from(&[false, false, false][..]));
        assert!(bitmap.is_none(0, 3));
    }

    #[test_case]
    fn test_bitmap_large() {
        let mut bitmap = Bitmap::new(300);
        let blist = &mut [false; 300][..];
        assert_eq!(bitmap, Bitmap::from(&*blist));

        bitmap.set(200, true);
        blist[200] = true;
        assert_eq!(bitmap, Bitmap::from(&*blist));

        bitmap.set_multiple(0, 300, true);
        assert_eq!(bitmap, Bitmap::from(&[true; 300][..]));
        assert!(bitmap.is_full(0, 300));

        assert_eq!(bitmap.find_and_flip(1, 3, false), None);
        assert_eq!(bitmap.find_and_flip(1, 3, true), Some(1));
        let blist = &mut [true; 300][..];
        (1..4).for_each(|i| blist[i] = false);
        assert_eq!(bitmap, Bitmap::from(&*blist));

        bitmap.set_multiple(0, 300, false);
        assert_eq!(bitmap, Bitmap::from(&[false; 300][..]));
    }
}

