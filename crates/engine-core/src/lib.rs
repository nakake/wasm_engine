pub mod ecs;
pub mod math;
pub mod components;

// Re-exports
pub use ecs::{EntityId, World};
pub use components::{Transform, ModelUniform, Name};
