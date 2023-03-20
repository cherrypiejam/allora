pub mod arena;
pub mod pool;
mod utils;

pub use pool::PAGE_SIZE;
pub use utils::{align_up, align_down};

enum Error {
    PageNotFound,
    PageExists,
}
