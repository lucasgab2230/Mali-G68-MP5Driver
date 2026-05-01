//! Memory management for the Mali-G68 MP5 GPU
//!
//! This module provides:
//! - Buffer object (BO) allocation via the DRM subsystem
//! - Slab allocator for small allocations (descriptors, push constants)
//! - Memory pool management for efficient suballocation
//!
//! ## Memory Layout for Emulators
//!
//! Emulators benefit from large contiguous allocations for:
//! - Texture atlas pools (shared across all textures)
//! - Vertex buffer pools (suballocated for draw calls)
//! - Staging buffers (for texture uploads from CPU)
//!
//! We use a suballocation strategy to minimize DRM allocation overhead
//! and improve cache locality.

pub mod bo;
pub mod slab;
pub mod pool;

pub use bo::BufferObject;
pub use slab::SlabAllocator;
pub use pool::MemoryPool;