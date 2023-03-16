pub mod arena;
pub mod pool;
mod utils;

const PAGE_BITS: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_BITS;
const PAGE_MASK: usize = (1 << PAGE_BITS) - 1;

pub use utils::{align_up, align_down};

enum Error {
    PageNotFound,
    PageExists,
}
