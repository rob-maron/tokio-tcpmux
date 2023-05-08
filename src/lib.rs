pub mod macros;
pub mod errors;
mod mux;
mod messages;
mod muxstream;
mod state;

pub use mux::*;
pub use muxstream::*;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;