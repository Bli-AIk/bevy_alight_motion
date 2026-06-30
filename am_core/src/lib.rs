//! Pure Alight Motion project data extraction layer.

pub mod animation;
pub mod coord;
pub mod effects_registry;
pub mod error;
#[cfg(feature = "ffi")]
pub mod ffi;
pub mod loader;
pub mod schema;
pub mod validation;

pub use error::AmError;
