//! Shader compiler: SPIR-V → Valhall ISA
//!
//! The shader compiler translates Vulkan SPIR-V shaders into native
//! Valhall ISA machine code for the Mali-G68 MP5.
//!
//! ## Compilation Pipeline
//!
//! 1. **SPIR-V Parse**: Parse SPIR-V binary into in-memory representation
//! 2. **NIR Lower**: Convert to NIR-like intermediate representation
//! 3. **Optimize**: Apply optimization passes (especially for emulator patterns)
//! 4. **Schedule**: Instruction scheduling for Valhall pipelines
//! 5. **Register Allocate**: Assign physical registers
//! 6. **Emit**: Generate Valhall ISA binary
//!
//! ## Emulator Optimizations
//!
//! - Special optimization for common emulator shader patterns
//! - Aggressive constant folding for UBO-based uniforms
//! - Texture sampling optimization for decoded textures
//! - Compute shader optimization for texture decoding (BCn, ASTC)

pub mod nir;
pub mod valhall;
pub mod optimize;
pub mod emulator_pass;

pub use nir::NirShader;
pub use valhall::ValhallCompiler;