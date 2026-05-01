//! Vulkan API implementation for Mali-G68 MP5
//!
//! This module implements the Vulkan 1.3 API on top of the Mali-G68 MP5
//! hardware driver. It provides the standard Vulkan entry points that
//! emulators and applications call to render graphics and compute.
//!
//! ## Implementation Status
//!
//! | Feature                    | Status       |
//! |---------------------------|--------------|
//! | Instance creation         | ✓ Partial    |
//! | Physical device           | ✓ Partial    |
//! | Device creation           | ✓ Partial    |
//! | Memory allocation         | ✓ Partial    |
//! | Buffer management         | ✓ Partial    |
//! | Image management          | ✓ Partial    |
//! | Shader modules            | ✓ Partial    |
//! | Pipeline creation         | ✓ Partial    |
//! | Command buffers           | ✓ Partial    |
//! | Render passes             | ✓ Partial    |
//! | Descriptor sets           | ✓ Partial    |
//! | Synchronization           | ✓ Partial    |
//! | Swapchain (WSI)           | Planned      |
//! | Dynamic rendering         | Planned      |

pub mod instance;
pub mod physical;
pub mod device;
pub mod memory;
pub mod buffer;
pub mod image;
pub mod pipeline;
pub mod shader;
pub mod descriptor;
pub mod render_pass;
pub mod sync;
pub mod swapchain;

pub use instance::VkInstance;
pub use physical::VkPhysicalDevice;
pub use device::VkDevice;