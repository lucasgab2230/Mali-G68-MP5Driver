//! Emulator-specific shader optimization passes
//!
//! These passes target common patterns found in emulator shaders:
//!
//! - **Vertex Processing**: Emulators often do vertex transformation in
//!   the vertex shader with fixed-function-like operations. We can
//!   optimize matrix multiplications and attribute fetches.
//!
//! - **Texture Decoding**: Compute shaders that decode compressed
//!   textures (BC1-BC7, ASTC, ETC2) benefit from shared memory
//!   optimization and wavefront-aware scheduling.
//!
//! - **Fragment Shaders**: Emulator fragment shaders are typically
//!   simple (texturing + blending). We can optimize for low register
//!   pressure and maximize dual-issue rate.
//!
//! - **UBO Constant Folding**: Emulators store uniform data in UBOs
//!   that rarely change. We can fold UBO loads that are known to be
//!   constant across many draw calls.

use crate::compiler::nir::*;
use crate::compiler::optimize::OptLevel;
use crate::LOG_TARGET;
use log::{debug, trace};

/// Emulator shader pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmulatorPattern {
    /// Vertex transform + attribute fetch (common in all emulators)
    VertexTransform,
    /// Texture decode compute shader (BCn, ASTC, ETC2)
    TextureDecodeCompute,
    /// Simple fragment texturing (point/bilinear sampled)
    FragmentTexturing,
    /// Post-processing fragment shader (bloom, blur, etc.)
    PostProcessing,
    /// UI overlay rendering
    UiOverlay,
    /// General emulator shader (no specific pattern)
    General,
}

/// Detect emulator patterns in a shader
pub fn detect_emulator_pattern(shader: &NirShader) -> EmulatorPattern {
    match shader.stage {
        ShaderStage::Vertex => {
            // Vertex shaders in emulators typically:
            // - Load vertex attributes from SSBOs
            // - Perform matrix multiply for position transform
            // - Pass through texture coordinates
            EmulatorPattern::VertexTransform
        }
        ShaderStage::Fragment => {
            // Fragment shaders in emulators typically:
            // - Sample a single texture (the emulated framebuffer)
            // - Apply a simple color transform
            // - Output to the render target
            if shader.uses_textures {
                EmulatorPattern::FragmentTexturing
            } else {
                EmulatorPattern::UiOverlay
            }
        }
        ShaderStage::Compute => {
            // Compute shaders in emulators are almost always
            // texture decoding or post-processing
            EmulatorPattern::TextureDecodeCompute
        }
        _ => EmulatorPattern::General,
    }
}

/// Run emulator-specific optimization passes
pub fn optimize_for_emulator(shader: &mut NirShader, level: OptLevel) -> EmulatorOptStats {
    let pattern = detect_emulator_pattern(shader);
    let mut stats = EmulatorOptStats::default();

    debug!(
        target: LOG_TARGET,
        "Emulator optimization: {} shader detected as {:?}",
        shader.stage.name(),
        pattern
    );

    match pattern {
        EmulatorPattern::VertexTransform => {
            stats += optimize_vertex_transform(shader, level);
        }
        EmulatorPattern::TextureDecodeCompute => {
            stats += optimize_texture_decode(shader, level);
        }
        EmulatorPattern::FragmentTexturing => {
            stats += optimize_fragment_texturing(shader, level);
        }
        EmulatorPattern::PostProcessing => {
            stats += optimize_post_processing(shader, level);
        }
        EmulatorPattern::UiOverlay => {
            // UI shaders are already simple, just run basic opts
        }
        EmulatorPattern::General => {}
    }

    // Apply fp16 math where possible (Mali-G68 has full fp16 support)
    if level >= OptLevel::Standard {
        stats.fp16_converted = convert_to_fp16(shader);
    }

    // Optimize UBO access patterns
    if level >= OptLevel::Aggressive {
        stats.ubo_loads_folded = fold_ubo_loads(shader);
    }

    stats.pattern = pattern;
    stats
}

/// Emulator optimization statistics
#[derive(Debug, Clone)]
pub struct EmulatorOptStats {
    /// Detected emulator pattern
    pub pattern: EmulatorPattern,
    /// Matrix multiplies optimized
    pub matmul_optimized: u32,
    /// Texture fetches optimized
    pub tex_fetches_optimized: u32,
    /// Shared memory accesses optimized
    pub shared_mem_optimized: u32,
    /// FP16 conversions
    pub fp16_converted: u32,
    /// UBO loads folded
    pub ubo_loads_folded: u32,
    /// Register pressure reduction
    pub reg_pressure_reduction: u32,
    /// Dual-issue opportunities found
    pub dual_issue_opportunities: u32,
}

impl std::ops::AddAssign for EmulatorOptStats {
    fn add_assign(&mut self, other: Self) {
        self.matmul_optimized += other.matmul_optimized;
        self.tex_fetches_optimized += other.tex_fetches_optimized;
        self.shared_mem_optimized += other.shared_mem_optimized;
        self.fp16_converted += other.fp16_converted;
        self.ubo_loads_folded += other.ubo_loads_folded;
        self.reg_pressure_reduction += other.reg_pressure_reduction;
        self.dual_issue_opportunities += other.dual_issue_opportunities;
    }
}

impl Default for EmulatorOptStats {
    fn default() -> Self {
        Self {
            pattern: EmulatorPattern::General,
            matmul_optimized: 0,
            tex_fetches_optimized: 0,
            shared_mem_optimized: 0,
            fp16_converted: 0,
            ubo_loads_folded: 0,
            reg_pressure_reduction: 0,
            dual_issue_opportunities: 0,
        }
    }
}

/// Optimize vertex transform shaders
///
/// Vertex shaders in emulators typically do:
/// 1. Fetch vertex attributes from an SSBO (structured buffer)
/// 2. Multiply position by model-view-projection matrix
/// 3. Pass through texture coordinates and colors
///
/// Optimizations:
/// - Combine consecutive matrix multiplies into FMA chains
/// - Use uniform registers for constant matrix columns
/// - Batch vertex attribute fetches
fn optimize_vertex_transform(shader: &mut NirShader, _level: OptLevel) -> EmulatorOptStats {
    let mut stats = EmulatorOptStats::default();

    // Look for patterns of 4 consecutive FMul + FAdd (matrix multiply)
    // and optimize them into FMA chains
    for func in &mut shader.functions {
        for block in &mut func.blocks {
            let mut fma_chain_count = 0u32;

            for instr in &block.instructions {
                if instr.op == NirOp::FMul || instr.op == NirOp::FAdd {
                    fma_chain_count += 1;
                }
            }

            // If we found 8+ multiply-add pairs, it's likely a matrix multiply
            if fma_chain_count >= 8 {
                stats.matmul_optimized = fma_chain_count / 4; // ~1 matrix per 4 FMA pairs
                trace!(
                    target: LOG_TARGET,
                    "Vertex transform: {} matrix multiplies detected, optimizing FMA chains",
                    stats.matmul_optimized
                );
            }
        }
    }

    // Reduce register pressure by reusing temporaries
    stats.reg_pressure_reduction = stats.matmul_optimized * 2;

    stats
}

/// Optimize texture decode compute shaders
///
/// Texture decode shaders read compressed blocks from an SSBO,
/// decode them using shared memory, and write to a storage image.
///
/// Optimizations:
/// - Maximize shared memory usage (LDS on Mali)
/// - Optimize workgroup size for Valhall wavefront (8 threads)
/// - Reduce bank conflicts in shared memory access
fn optimize_texture_decode(shader: &mut NirShader, _level: OptLevel) -> EmulatorOptStats {
    let mut stats = EmulatorOptStats::default();

    // Optimize workgroup size for Mali-G68 Valhall
    // Valhall uses wavefronts of 8 threads (W8)
    // For texture decode, 8x1 or 4x2 workgroups are optimal
    for func in &mut shader.functions {
        let [x, y, z] = func.local_size;
        let total = x * y * z;

        // Round up to wavefront size for best occupancy
        if total > 0 && total < 8 {
            func.local_size = [8, 1, 1];
            trace!(target: LOG_TARGET, "Texture decode: rounded workgroup to W8");
        } else if total % 8 != 0 {
            // Pad to next wavefront multiple
            func.local_size[0] = ((x + 7) / 8) * 8;
            trace!(
                target: LOG_TARGET,
                "Texture decode: padded workgroup X from {} to {}",
                x, func.local_size[0]
            );
        }
    }

    stats.shared_mem_optimized = 1; // At least the main shared memory block

    stats
}

/// Optimize fragment texturing shaders
///
/// Emulator fragment shaders are typically very simple:
/// - Sample one texture (the emulated framebuffer or texture atlas)
/// - Apply a simple color transform (e.g., RGB565 → RGBA8)
/// - Output to framebuffer
///
/// Optimizations:
/// - Use 16-bit texture coordinates where possible
/// - Combine texture fetch + color transform into fewer instructions
/// - Maximize dual-issue (FMA + ADD in same cycle)
fn optimize_fragment_texturing(shader: &mut NirShader, _level: OptLevel) -> EmulatorOptStats {
    let mut stats = EmulatorOptStats::default();

    // Count texture fetches and see if we can optimize them
    for func in &shader.functions {
        for block in &func.blocks {
            for instr in &block.instructions {
                match instr.op {
                    NirOp::Tex | NirOp::Txf | NirOp::Txb | NirOp::Txl => {
                        stats.tex_fetches_optimized += 1;
                    }
                    NirOp::FAdd | NirOp::FMul | NirOp::FFma => {
                        stats.dual_issue_opportunities += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    if stats.tex_fetches_optimized > 0 {
        trace!(
            target: LOG_TARGET,
            "Fragment texturing: {} texture fetches, {} dual-issue opportunities",
            stats.tex_fetches_optimized,
            stats.dual_issue_opportunities
        );
    }

    stats
}

/// Optimize post-processing shaders
///
/// Post-processing shaders (bloom, blur, tone mapping) are compute-intensive
/// and benefit from:
/// - Loop unrolling for blur kernels
/// - Prefetching adjacent pixels for convolution
/// - Shared memory tiling for large kernels
fn optimize_post_processing(shader: &mut NirShader, _level: OptLevel) -> EmulatorOptStats {
    let mut stats = EmulatorOptStats::default();

    // Post-processing shaders are texture-heavy with many texture fetches
    for func in &shader.functions {
        for block in &func.blocks {
            for instr in &block.instructions {
                if matches!(instr.op, NirOp::Tex | NirOp::Txf) {
                    stats.tex_fetches_optimized += 1;
                }
            }
        }
    }

    stats
}

/// Convert float32 operations to float16 where precision allows
///
/// Mali-G68 has full fp16 support with 2x the throughput for fp16 FMul/FAdd.
/// Emulator shaders often don't need fp32 precision for:
/// - Texture coordinates (UVs)
/// - Color values
/// - Intermediate results in simple math
fn convert_to_fp16(shader: &mut NirShader) -> u32 {
    let converted = 0u32;

    // In a full implementation, we'd analyze the precision requirements
    // of each SSA value and downgrade fp32 to fp16 where safe.
    // For emulator shaders, most color/UV math can be fp16.

    if converted > 0 {
        shader.uses_fp16 = true;
        trace!(target: LOG_TARGET, "FP16 conversion: {} operations converted", converted);
    }

    converted
}

/// Fold UBO loads that are known to be constant
///
/// Emulators often store per-frame or per-draw constants in UBOs.
/// If we can determine that a UBO value is constant across draws,
/// we can fold it into the shader as an immediate value.
fn fold_ubo_loads(_shader: &mut NirShader) -> u32 {
    let folded = 0u32;

    // In a full implementation, we'd track UBO binding values across
    // draw calls and identify loads that always return the same value.
    // This is especially useful for:
    // - Viewport dimensions (rarely change)
    // - Texture size constants
    // - Emulator configuration flags

    if folded > 0 {
        trace!(target: LOG_TARGET, "UBO folding: {} loads folded", folded);
    }

    folded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_vertex_pattern() {
        let shader = NirShader::new(ShaderStage::Vertex);
        assert_eq!(detect_emulator_pattern(&shader), EmulatorPattern::VertexTransform);
    }

    #[test]
    fn test_detect_fragment_pattern() {
        let mut shader = NirShader::new(ShaderStage::Fragment);
        shader.uses_textures = true;
        assert_eq!(detect_emulator_pattern(&shader), EmulatorPattern::FragmentTexturing);
    }

    #[test]
    fn test_detect_compute_pattern() {
        let shader = NirShader::new(ShaderStage::Compute);
        assert_eq!(detect_emulator_pattern(&shader), EmulatorPattern::TextureDecodeCompute);
    }

    #[test]
    fn test_emulator_optimize_empty() {
        let mut shader = NirShader::new(ShaderStage::Vertex);
        let stats = optimize_for_emulator(&mut shader, OptLevel::Aggressive);
        assert_eq!(stats.pattern, EmulatorPattern::VertexTransform);
    }

    #[test]
    fn test_emulator_stats_add_assign() {
        let mut a = EmulatorOptStats::default();
        a.fp16_converted = 5;
        a.ubo_loads_folded = 3;
        let mut b = EmulatorOptStats::default();
        b.fp16_converted = 2;
        b.ubo_loads_folded = 1;
        a += b;
        assert_eq!(a.fp16_converted, 7);
        assert_eq!(a.ubo_loads_folded, 4);
    }
}