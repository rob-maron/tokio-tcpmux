pub mod macros;
pub mod errors;
pub mod mux;
mod messages;
mod muxstream;
mod state;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;