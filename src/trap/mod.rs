//! OSHIT Trap Handle unit.
mod trap_context;
mod trap_handler;

pub use trap_context::TrapContext;
pub use trap_handler::{init, user_trap, trap_return};