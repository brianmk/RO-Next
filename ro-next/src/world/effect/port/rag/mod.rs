//! Ported effect bodies.
//!
//! One module per original dispatch region/method group. Start with the
//! classic `CRagEffect` methods that the pre-renewal skills use most.

pub mod bash;
pub mod cart;
pub mod magnum;
pub mod sonicblow;

pub use bash::Bash;
pub use cart::CartRevolution;
pub use magnum::MagnumBreak;
pub use sonicblow::SonicBlow;
