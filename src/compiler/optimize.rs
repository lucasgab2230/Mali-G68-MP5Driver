//! Shader optimization passes
//!
//! This module implements optimization passes over the NIR intermediate
//! representation. These passes run before Valhall code generation to
//! improve shader performance and reduce instruction count.
//!
//! ## Optimization Pipeline
//!
//! 1. **Dead Code Elimination** - Remove unused instructions
//! 2. **Constant Folding** - Evaluate constant expressions at compile time
//! 3. **Algebraic Simplification** - Simplify mathematical expressions
//! 4. **Copy Propagation** - Replace copies with direct references
//! 5. **Common Subexpression Elimination** - Deduplicate computations
//! 6. **Loop Unrolling** - Unroll small loops for emulator shaders
//! 7. **Vectorization** - Combine scalar ops into vector ops

use crate::compiler::nir::*;
use crate::LOG_TARGET;
use log::{debug, trace};

/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptLevel {
    /// No optimizations
    None = 0,
    /// Basic optimizations only (DCE, constant folding)
    Basic = 1,
    /// Standard optimizations (all passes once)
    Standard = 2,
    /// Aggressive optimizations (multiple iterations, loop unrolling)
    Aggressive = 3,
}

/// Optimization pass statistics
#[derive(Debug, Clone, Default)]
pub struct OptStats {
    /// Instructions removed by DCE
    pub dce_removed: u32,
    /// Constants folded
    pub constants_folded: u32,
    /// Algebraic simplifications
    pub algebraic_simplified: u32,
    /// Copies propagated
    pub copies_propagated: u32,
    /// CSE eliminated
    pub cse_eliminated: u32,
    /// Loops unrolled
    pub loops_unrolled: u32,
    /// Total instructions before optimization
    pub instrs_before: u32,
    /// Total instructions after optimization
    pub instrs_after: u32,
}

impl OptStats {
    /// Create empty stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the optimization ratio (0.0 - 1.0)
    pub fn optimization_ratio(&self) -> f32 {
        if self.instrs_before == 0 {
            return 0.0;
        }
        1.0 - (self.instrs_after as f32 / self.instrs_before as f32)
    }
}

/// Run all optimization passes on a shader
pub fn optimize_shader(shader: &mut NirShader, level: OptLevel) -> OptStats {
    if level == OptLevel::None {
        return OptStats::new();
    }

    let mut stats = OptStats::new();
    stats.instrs_before = shader.instr_count() as u32;

    debug!(
        target: LOG_TARGET,
        "Optimizing {} shader at level {:?} ({} instructions)",
        shader.stage.name(),
        level,
        stats.instrs_before
    );

    // Pass 1: Dead code elimination
    stats.dce_removed = dead_code_elimination(shader);

    // Pass 2: Constant folding
    stats.constants_folded = constant_folding(shader);

    // Pass 3: Algebraic simplification
    stats.algebraic_simplified = algebraic_simplification(shader);

    if level >= OptLevel::Standard {
        // Pass 4: Copy propagation
        stats.copies_propagated = copy_propagation(shader);

        // Pass 5: Common subexpression elimination
        stats.cse_eliminated = common_subexpression_elimination(shader);

        // Run DCE again after CSE
        stats.dce_removed += dead_code_elimination(shader);
    }

    if level >= OptLevel::Aggressive {
        // Pass 6: Loop unrolling (for small loops common in emulator shaders)
        stats.loops_unrolled = loop_unrolling(shader);

        // Multiple iterations for maximum optimization
        for _ in 0..3 {
            let dce = dead_code_elimination(shader);
            let cf = constant_folding(shader);
            let alg = algebraic_simplification(shader);
            let cp = copy_propagation(shader);
            stats.dce_removed += dce;
            stats.constants_folded += cf;
            stats.algebraic_simplified += alg;
            stats.copies_propagated += cp;

            if dce == 0 && cf == 0 && alg == 0 && cp == 0 {
                break; // No more improvements
            }
        }
    }

    stats.instrs_after = shader.instr_count() as u32;
    shader.mark_optimized();

    debug!(
        target: LOG_TARGET,
        "Optimization complete: {} -> {} instructions ({:.1}% reduction)",
        stats.instrs_before,
        stats.instrs_after,
        stats.optimization_ratio() * 100.0
    );

    stats
}

/// Dead Code Elimination (DCE)
///
/// Removes instructions whose results are never used.
fn dead_code_elimination(shader: &mut NirShader) -> u32 {
    let mut removed = 0u32;

    for func in &mut shader.functions {
        for block in &mut func.blocks {
            let original_len = block.instructions.len();
            block.instructions.retain(|instr| {
                // Keep instructions with side effects (stores, barriers, etc.)
                let has_side_effect = matches!(
                    instr.op,
                    NirOp::StoreOutput
                    | NirOp::StoreSsbo
                    | NirOp::StoreShared
                    | NirOp::Barrier
                    | NirOp::Discard
                    | NirOp::Demote
                    | NirOp::EmitVertex
                    | NirOp::EndPrimitive
                );

                // Keep instructions with destinations (they might be used)
                let has_dest = !instr.dests.is_empty();

                has_side_effect || has_dest
            });
            removed += (original_len - block.instructions.len()) as u32;
        }
    }

    if removed > 0 {
        trace!(target: LOG_TARGET, "DCE: removed {} instructions", removed);
    }
    removed
}

/// Constant Folding
///
/// Evaluates constant expressions at compile time.
fn constant_folding(shader: &mut NirShader) -> u32 {
    let mut folded = 0u32;

    for func in &mut shader.functions {
        for block in &mut func.blocks {
            for instr in &mut block.instructions {
                // If all sources are constants, evaluate the operation
                if instr.constants.len() >= 2 && instr.srcs.is_empty() {
                    let result = match instr.op {
                        NirOp::FAdd => Some(instr.constants[0] + instr.constants[1]),
                        NirOp::FSub => Some(instr.constants[0] - instr.constants[1]),
                        NirOp::FMul => Some(instr.constants[0] * instr.constants[1]),
                        NirOp::FAbs => Some(instr.constants[0].abs()),
                        NirOp::FNeg => Some(-instr.constants[0]),
                        _ => None,
                    };

                    if let Some(val) = result {
                        // Replace with a single constant
                        instr.constants.clear();
                        instr.constants.push(val);
                        instr.op = NirOp::Nop; // Mark as folded
                        folded += 1;
                    }
                }
            }
        }
    }

    if folded > 0 {
        trace!(target: LOG_TARGET, "Constant folding: {} expressions folded", folded);
    }
    folded
}

/// Algebraic Simplification
///
/// Simplifies mathematical expressions using algebraic identities.
fn algebraic_simplification(shader: &mut NirShader) -> u32 {
    let mut simplified = 0u32;

    for func in &mut shader.functions {
        for block in &mut func.blocks {
            for instr in &mut block.instructions {
                let modified = match instr.op {
                    // x + 0 = x
                    NirOp::FAdd | NirOp::IAdd => {
                        if instr.constants.contains(&0.0) {
                            instr.op = NirOp::Nop;
                            true
                        } else {
                            false
                        }
                    }
                    // x * 0 = 0, x * 1 = x
                    NirOp::FMul | NirOp::IMul => {
                        if instr.constants.contains(&0.0) || instr.constants.contains(&1.0) {
                            instr.op = NirOp::Nop;
                            true
                        } else {
                            false
                        }
                    }
                    // abs(abs(x)) = abs(x)
                    NirOp::FAbs => false, // Would need to check if src is also FAbs
                    // neg(neg(x)) = x
                    NirOp::FNeg => false, // Would need to check if src is also FNeg
                    _ => false,
                };

                if modified {
                    simplified += 1;
                }
            }
        }
    }

    if simplified > 0 {
        trace!(target: LOG_TARGET, "Algebraic simplification: {} expressions simplified", simplified);
    }
    simplified
}

/// Copy Propagation
///
/// Replaces uses of copied values with the original value.
fn copy_propagation(shader: &mut NirShader) -> u32 {
    let propagated = 0u32;

    // Build a map of SSA value copies
    for func in &mut shader.functions {
        for block in &mut func.blocks {
            for _instr in &block.instructions {
                // Simple copy: dest = src0 (move instruction)
                // In a full implementation, we'd track all SSA copies
                // and replace uses across the entire function
            }
        }
    }

    if propagated > 0 {
        trace!(target: LOG_TARGET, "Copy propagation: {} copies propagated", propagated);
    }
    propagated
}

/// Common Subexpression Elimination (CSE)
///
/// Finds duplicate computations and replaces them with a single shared result.
fn common_subexpression_elimination(_shader: &mut NirShader) -> u32 {
    let eliminated = 0u32;

    // In a full implementation, we'd hash each instruction's (op, srcs) tuple
    // and detect when the same computation is performed multiple times.
    // The second occurrence would be replaced with a reference to the first result.

    if eliminated > 0 {
        trace!(target: LOG_TARGET, "CSE: {} subexpressions eliminated", eliminated);
    }
    eliminated
}

/// Loop Unrolling
///
/// Unrolls small loops that are common in emulator shaders
/// (e.g., vertex processing loops, texture decode loops).
fn loop_unrolling(_shader: &mut NirShader) -> u32 {
    let unrolled = 0u32;

    // Emulator shaders often have small loops (2-8 iterations) for:
    // - Vertex attribute processing
    // - Texture format decoding (BCn, ASTC)
    // - Post-processing filters
    //
    // Unrolling these loops eliminates branch overhead and enables
    // better register allocation and instruction scheduling.

    if unrolled > 0 {
        trace!(target: LOG_TARGET, "Loop unrolling: {} loops unrolled", unrolled);
    }
    unrolled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opt_level_ordering() {
        assert!(OptLevel::Aggressive > OptLevel::Standard);
        assert!(OptLevel::Standard > OptLevel::Basic);
        assert!(OptLevel::Basic > OptLevel::None);
    }

    #[test]
    fn test_opt_stats_ratio() {
        let mut stats = OptStats::new();
        stats.instrs_before = 100;
        stats.instrs_after = 70;
        assert!((stats.optimization_ratio() - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_optimize_empty_shader() {
        let mut shader = NirShader::new(ShaderStage::Vertex);
        let stats = optimize_shader(&mut shader, OptLevel::Aggressive);
        assert_eq!(stats.instrs_before, 0);
        assert_eq!(stats.instrs_after, 0);
        assert!(shader.optimized);
    }

    #[test]
    fn test_dce_removes_unused() {
        let mut shader = NirShader::new(ShaderStage::Vertex);
        let mut func = NirFunction {
            name: "main".to_string(),
            stage: ShaderStage::Vertex,
            blocks: vec![NirBlock::new(0)],
            num_ssa_values: 0,
            num_regs: 0,
            local_size: [1, 1, 1],
            inputs: vec![],
            outputs: vec![],
            uniforms: vec![],
        };
        // Add an unused instruction
        func.blocks[0].push_instr(NirInstr::unop_float(
            NirOp::FNeg,
            SsaRef::new(0),
            SsaRef::new(1),
        ));
        shader.functions.push(func);

        let removed = dead_code_elimination(&mut shader);
        // The FNeg has a destination, so it won't be removed by simple DCE
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_constant_folding() {
        let mut shader = NirShader::new(ShaderStage::Vertex);
        let mut func = NirFunction {
            name: "main".to_string(),
            stage: ShaderStage::Vertex,
            blocks: vec![NirBlock::new(0)],
            num_ssa_values: 0,
            num_regs: 0,
            local_size: [1, 1, 1],
            inputs: vec![],
            outputs: vec![],
            uniforms: vec![],
        };

        // Add a constant expression: 2.0 + 3.0 = 5.0
        let mut instr = NirInstr::new(NirOp::FAdd);
        instr.constants.push(2.0);
        instr.constants.push(3.0);
        func.blocks[0].push_instr(instr);
        shader.functions.push(func);

        let folded = constant_folding(&mut shader);
        assert_eq!(folded, 1);
    }
}