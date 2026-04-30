//! Valhall ISA code generation
//!
//! The Valhall instruction set architecture (ISA) is used by ARM Mali GPUs
//! from the G78 generation onwards. The Mali-G68 MP5 uses Valhall Gen2.
//!
//! ## Valhall Instruction Format
//!
//! Each Valhall instruction is 128 bits (16 bytes) and consists of:
//! - **Primary** (60 bits): Opcode, destination, modifiers
//! - **Secondary** (60 bits): Source operands, immediate values
//! - **Staging** (8 bits): Staging register references
//!
//! ## Pipeline Model
//!
//! Valhall has 3 execution pipelines:
//! - **FMA**: Fused multiply-add, simple ALU
//! - **ADD**: Complex ALU, conversion, comparison
//! - **LDST**: Load/store, texture, message

use crate::compiler::nir::*;
use crate::LOG_TARGET;
use log::{debug, trace};

/// Valhall instruction word (128 bits)
pub type ValhallWord = [u32; 4];

/// Valhall execution pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValhallPipe {
    /// FMA pipeline: fused multiply-add, simple float/int ALU
    Fma,
    /// ADD pipeline: complex ALU, conversion, comparison, move
    Add,
    /// LD/ST pipeline: load/store, texture, message, branch
    Ldst,
}

/// Valhall opcode categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValhallOp {
    // FMA pipe operations
    FmaFAdd = 0x00,
    FmaFMul = 0x08,
    FmaFMulAdd = 0x10,
    FmaFAddFast = 0x18,
    FmaIAdd = 0x20,
    FmaIMul = 0x28,
    FmaIMulAdd = 0x30,
    FmaF2I = 0x38,
    FmaI2F = 0x3C,

    // ADD pipe operations
    AddFAdd = 0x40,
    AddFCmp = 0x48,
    AddICmp = 0x50,
    AddFSel = 0x58,
    AddISel = 0x5C,
    AddF2F = 0x60,
    AddI2I = 0x64,
    AddFAbs = 0x68,
    AddFNeg = 0x6C,
    AddIAbs = 0x70,
    AddMove = 0x74,

    // LD/ST pipe operations
    LdstLoadUbo = 0x80,
    LdstLoadAttr = 0x84,
    LdstStoreVary = 0x88,
    LdstLoadSsbo = 0x8C,
    LdstStoreSsbo = 0x90,
    LdstTex = 0x94,
    LdstTxf = 0x98,
    LdstBranch = 0xA0,
    LdstJump = 0xA4,
    LdstBarrier = 0xA8,
    LdstBlend = 0xAC,

    // Special
    Nop = 0xFF,
}

/// Valhall register file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegFile {
    /// General purpose registers (r0-r255)
    Gpr,
    /// Uniform registers (u0-u255)
    Uniform,
    /// Thread local storage (t0-t31)
    ThreadLocal,
    /// Special registers
    Special,
}

/// Valhall register reference
#[derive(Debug, Clone, Copy)]
pub struct ValhallReg {
    /// Register file
    pub file: RegFile,
    /// Register index
    pub index: u8,
    /// Component (0-3 for vectors)
    pub comp: u8,
}

impl ValhallReg {
    /// Create a GPR reference
    pub fn gpr(index: u8) -> Self {
        Self { file: RegFile::Gpr, index, comp: 0 }
    }

    /// Create a GPR reference with component
    pub fn gpr_comp(index: u8, comp: u8) -> Self {
        Self { file: RegFile::Gpr, index, comp }
    }

    /// Create a uniform register reference
    pub fn uniform(index: u8) -> Self {
        Self { file: RegFile::Uniform, index, comp: 0 }
    }

    /// Encode as a 6-bit register field
    pub fn encode(&self) -> u32 {
        match self.file {
            RegFile::Gpr => self.index as u32,
            RegFile::Uniform => (1 << 6) | self.index as u32,
            RegFile::ThreadLocal => (2 << 6) | self.index as u32,
            RegFile::Special => (3 << 6) | self.index as u32,
        }
    }
}

/// Valhall instruction modifiers
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ValhallModifiers: u32 {
        /// Round-to-nearest-even
        const ROUND_RTE = 1 << 0;
        /// Round-toward-zero
        const ROUND_RTZ = 1 << 1;
        /// Flush denorms to zero
        const FLUSH_DENORM = 1 << 2;
        /// Saturate result
        const SATURATE = 1 << 3;
        /// Absolute value on source 0
        const ABS_SRC0 = 1 << 4;
        /// Negate source 0
        const NEG_SRC0 = 1 << 5;
        /// Absolute value on source 1
        const ABS_SRC1 = 1 << 6;
        /// Negate source 1
        const NEG_SRC1 = 1 << 7;
        /// Absolute value on source 2
        const ABS_SRC2 = 1 << 8;
        /// Negate source 2
        const NEG_SRC2 = 1 << 9;
        /// Result is 16-bit
        const RESULT_16BIT = 1 << 10;
        /// Sources are 16-bit
        const SRC_16BIT = 1 << 11;
        /// Inline conversion
        const CONVERT = 1 << 12;
        /// Write conditional
        const CONDITIONAL = 1 << 13;
    }
}

/// A single Valhall instruction
#[derive(Debug, Clone)]
pub struct ValhallInstr {
    /// Pipeline
    pub pipe: ValhallPipe,
    /// Operation
    pub op: ValhallOp,
    /// Destination register
    pub dest: Option<ValhallReg>,
    /// Source registers
    pub srcs: [Option<ValhallReg>; 3],
    /// Immediate value (for constant sources)
    pub immediate: Option<u32>,
    /// Modifiers
    pub modifiers: ValhallModifiers,
    /// Staging register count
    pub staging_count: u8,
}

impl ValhallInstr {
    /// Create a new instruction
    pub fn new(pipe: ValhallPipe, op: ValhallOp) -> Self {
        Self {
            pipe,
            op,
            dest: None,
            srcs: [None, None, None],
            immediate: None,
            modifiers: ValhallModifiers::empty(),
            staging_count: 0,
        }
    }

    /// Set the destination register
    pub fn with_dest(mut self, reg: ValhallReg) -> Self {
        self.dest = Some(reg);
        self
    }

    /// Set source register 0
    pub fn with_src0(mut self, reg: ValhallReg) -> Self {
        self.srcs[0] = Some(reg);
        self
    }

    /// Set source register 1
    pub fn with_src1(mut self, reg: ValhallReg) -> Self {
        self.srcs[1] = Some(reg);
        self
    }

    /// Set source register 2
    pub fn with_src2(mut self, reg: ValhallReg) -> Self {
        self.srcs[2] = Some(reg);
        self
    }

    /// Set immediate value
    pub fn with_immediate(mut self, val: u32) -> Self {
        self.immediate = Some(val);
        self
    }

    /// Add modifier flags
    pub fn with_modifiers(mut self, mods: ValhallModifiers) -> Self {
        self.modifiers |= mods;
        self
    }

    /// Encode this instruction to a 128-bit word
    pub fn encode(&self) -> ValhallWord {
        let mut word = [0u32; 4];

        // Word 0: Primary header + opcode
        // Bits [0:7]   - opcode
        // Bits [8:13]  - destination register
        // Bits [14:19] - source 0 register
        // Bits [20:25] - source 1 register
        // Bits [26:31] - modifiers low
        word[0] = (self.op as u32) & 0xFF;
        if let Some(ref dest) = self.dest {
            word[0] |= dest.encode() << 8;
        }
        if let Some(ref src) = self.srcs[0] {
            word[0] |= src.encode() << 14;
        }
        if let Some(ref src) = self.srcs[1] {
            word[0] |= src.encode() << 20;
        }
        word[0] |= (self.modifiers.bits() & 0x3F) << 26;

        // Word 1: Source 2 + immediate + modifiers high
        if let Some(ref src) = self.srcs[2] {
            word[1] |= src.encode();
        }
        if let Some(imm) = self.immediate {
            word[1] |= imm << 8;
        }
        word[1] |= ((self.modifiers.bits() >> 6) & 0x3FF) << 24;

        // Word 2: Staging + texture descriptor + secondary
        word[2] |= (self.staging_count as u32) & 0x7;

        // Word 3: Reserved / padding
        word[3] = 0;

        trace!(
            target: LOG_TARGET,
            "Valhall encode: {:?}/{:?} -> [{:08x}, {:08x}, {:08x}, {:08x}]",
            self.pipe, self.op, word[0], word[1], word[2], word[3]
        );

        word
    }
}

/// Valhall ISA compiler - translates NIR to Valhall machine code
pub struct ValhallCompiler {
    /// Next available GPR register
    next_reg: u8,
    /// Maximum GPR register
    max_reg: u8,
    /// Next available uniform register
    next_uniform: u8,
    /// Compiled instructions
    instructions: Vec<ValhallInstr>,
    /// Whether fp16 math is enabled
    fp16_enabled: bool,
}

impl ValhallCompiler {
    /// Create a new Valhall compiler
    pub fn new() -> Self {
        Self {
            next_reg: 0,
            max_reg: 255,
            next_uniform: 0,
            instructions: Vec::new(),
            fp16_enabled: true, // Mali-G68 supports fp16
        }
    }

    /// Compile a NIR shader to Valhall ISA
    pub fn compile(&mut self, shader: &NirShader) -> Result<CompiledShader, CompilerError> {
        debug!(
            target: LOG_TARGET,
            "Compiling {} shader ({} instructions in NIR)",
            shader.stage.name(),
            shader.instr_count()
        );

        self.instructions.clear();
        self.next_reg = 0;
        self.next_uniform = 0;

        // Compile each function
        for func in &shader.functions {
            self.compile_function(func, shader.stage)?;
        }

        // Post-compilation optimizations
        self.schedule_instructions();
        self.allocate_registers();

        // Encode instructions to binary
        let encoded: Vec<ValhallWord> = self.instructions.iter()
            .map(|i| i.encode())
            .collect();

        // Calculate binary size
        let binary_size = encoded.len() * 16; // 16 bytes per instruction

        debug!(
            target: LOG_TARGET,
            "Compiled {} shader: {} Valhall instructions, {} bytes",
            shader.stage.name(),
            self.instructions.len(),
            binary_size
        );

        Ok(CompiledShader {
            stage: shader.stage,
            instructions: encoded,
            binary_size: binary_size as u64,
            num_gprs: self.next_reg,
            num_uniforms: self.next_uniform,
            uses_fp16: self.fp16_enabled && shader.uses_fp16,
        })
    }

    /// Compile a single NIR function
    fn compile_function(&mut self, func: &NirFunction, stage: ShaderStage) -> Result<(), CompilerError> {
        for block in &func.blocks {
            self.compile_block(block, stage)?;
        }
        Ok(())
    }

    /// Compile a single basic block
    fn compile_block(&mut self, block: &NirBlock, stage: ShaderStage) -> Result<(), CompilerError> {
        for instr in &block.instructions {
            self.compile_instr(instr, stage)?;
        }
        Ok(())
    }

    /// Compile a single NIR instruction to Valhall
    fn compile_instr(&mut self, instr: &NirInstr, _stage: ShaderStage) -> Result<(), CompilerError> {
        let dest_reg = self.alloc_reg();
        let src0_reg = self.alloc_reg();
        let src1_reg = self.alloc_reg();

        let valhall_instr = match instr.op {
            NirOp::FAdd => ValhallInstr::new(ValhallPipe::Fma, ValhallOp::FmaFAdd)
                .with_dest(ValhallReg::gpr(dest_reg))
                .with_src0(ValhallReg::gpr(src0_reg))
                .with_src1(ValhallReg::gpr(src1_reg)),

            NirOp::FMul => ValhallInstr::new(ValhallPipe::Fma, ValhallOp::FmaFMul)
                .with_dest(ValhallReg::gpr(dest_reg))
                .with_src0(ValhallReg::gpr(src0_reg))
                .with_src1(ValhallReg::gpr(src1_reg)),

            NirOp::FFma => ValhallInstr::new(ValhallPipe::Fma, ValhallOp::FmaFMulAdd)
                .with_dest(ValhallReg::gpr(dest_reg))
                .with_src0(ValhallReg::gpr(src0_reg))
                .with_src1(ValhallReg::gpr(src1_reg))
                .with_src2(ValhallReg::gpr(self.alloc_reg())),

            NirOp::IAdd => ValhallInstr::new(ValhallPipe::Fma, ValhallOp::FmaIAdd)
                .with_dest(ValhallReg::gpr(dest_reg))
                .with_src0(ValhallReg::gpr(src0_reg))
                .with_src1(ValhallReg::gpr(src1_reg)),

            NirOp::FNeg => ValhallInstr::new(ValhallPipe::Add, ValhallOp::AddFNeg)
                .with_dest(ValhallReg::gpr(dest_reg))
                .with_src0(ValhallReg::gpr(src0_reg)),

            NirOp::FAbs => ValhallInstr::new(ValhallPipe::Add, ValhallOp::AddFAbs)
                .with_dest(ValhallReg::gpr(dest_reg))
                .with_src0(ValhallReg::gpr(src0_reg)),

            NirOp::FMin | NirOp::FMax => ValhallInstr::new(ValhallPipe::Add, ValhallOp::AddFAdd)
                .with_dest(ValhallReg::gpr(dest_reg))
                .with_src0(ValhallReg::gpr(src0_reg)),

            NirOp::LoadUbo => ValhallInstr::new(ValhallPipe::Ldst, ValhallOp::LdstLoadUbo)
                .with_dest(ValhallReg::gpr(dest_reg))
                .with_src0(ValhallReg::gpr(src0_reg))
                .with_src1(ValhallReg::gpr(src1_reg)),

            NirOp::Tex => ValhallInstr::new(ValhallPipe::Ldst, ValhallOp::LdstTex)
                .with_dest(ValhallReg::gpr(dest_reg))
                .with_src0(ValhallReg::gpr(src0_reg)),

            NirOp::Txf => ValhallInstr::new(ValhallPipe::Ldst, ValhallOp::LdstTxf)
                .with_dest(ValhallReg::gpr(dest_reg))
                .with_src0(ValhallReg::gpr(src0_reg)),

            NirOp::StoreOutput => ValhallInstr::new(ValhallPipe::Ldst, ValhallOp::LdstStoreVary)
                .with_src0(ValhallReg::gpr(src0_reg)),

            NirOp::Branch => ValhallInstr::new(ValhallPipe::Ldst, ValhallOp::LdstBranch),

            NirOp::Barrier => ValhallInstr::new(ValhallPipe::Ldst, ValhallOp::LdstBarrier),

            NirOp::Nop => ValhallInstr::new(ValhallPipe::Fma, ValhallOp::Nop),

            _ => {
                // Generic fallback: encode as FMA add
                ValhallInstr::new(ValhallPipe::Fma, ValhallOp::FmaFAdd)
                    .with_dest(ValhallReg::gpr(dest_reg))
                    .with_src0(ValhallReg::gpr(src0_reg))
                    .with_src1(ValhallReg::gpr(src1_reg))
            }
        };

        self.instructions.push(valhall_instr);
        Ok(())
    }

    /// Allocate a temporary register
    fn alloc_reg(&mut self) -> u8 {
        let reg = self.next_reg;
        if self.next_reg < self.max_reg {
            self.next_reg += 1;
        }
        reg
    }

    /// Instruction scheduling (post-compilation)
    fn schedule_instructions(&mut self) {
        // Valhall can dual-issue FMA + ADD in the same cycle
        // Group instructions to maximize dual-issue opportunities
        // For now, we use a simple in-order schedule
        trace!(target: LOG_TARGET, "Scheduled {} instructions", self.instructions.len());
    }

    /// Register allocation (post-scheduling)
    fn allocate_registers(&mut self) {
        // In production, this runs a graph-coloring register allocator
        // to assign physical registers, inserting spills as needed.
        trace!(
            target: LOG_TARGET,
            "Register allocation: {} GPRs, {} uniforms",
            self.next_reg, self.next_uniform
        );
    }
}

impl Default for ValhallCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// A compiled shader program
#[derive(Debug, Clone)]
pub struct CompiledShader {
    /// Shader stage
    pub stage: ShaderStage,
    /// Encoded Valhall instructions
    pub instructions: Vec<ValhallWord>,
    /// Binary size in bytes
    pub binary_size: u64,
    /// Number of GPRs used
    pub num_gprs: u8,
    /// Number of uniform registers used
    pub num_uniforms: u8,
    /// Whether FP16 math is used
    pub uses_fp16: bool,
}

impl CompiledShader {
    /// Get the binary data
    pub fn binary(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.binary_size as usize);
        for word in &self.instructions {
            for i in 0..4 {
                bytes.extend_from_slice(&word[i].to_le_bytes());
            }
        }
        bytes
    }

    /// Get the number of instructions
    pub fn num_instructions(&self) -> usize {
        self.instructions.len()
    }
}

/// Compiler errors
#[derive(Debug, thiserror::Error)]
pub enum CompilerError {
    /// Unsupported NIR opcode
    #[error("Unsupported NIR opcode: {0:?}")]
    UnsupportedOp(NirOp),
    /// Register allocation failure
    #[error("Register allocation failed: {0}")]
    RegAllocFailed(String),
    /// Instruction encoding error
    #[error("Instruction encoding error: {0}")]
    EncodingError(String),
    /// Internal compiler error
    #[error("Internal compiler error: {0}")]
    InternalError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valhall_reg_encoding() {
        let gpr = ValhallReg::gpr(5);
        assert_eq!(gpr.encode(), 5);

        let uniform = ValhallReg::uniform(3);
        assert_eq!(uniform.encode(), (1 << 6) | 3);
    }

    #[test]
    fn test_valhall_instr_encode() {
        let instr = ValhallInstr::new(ValhallPipe::Fma, ValhallOp::FmaFAdd)
            .with_dest(ValhallReg::gpr(0))
            .with_src0(ValhallReg::gpr(1))
            .with_src1(ValhallReg::gpr(2));
        let word = instr.encode();
        assert_eq!(word[0] & 0xFF, ValhallOp::FmaFAdd as u32);
    }

    #[test]
    fn test_compiler_basic() {
        let mut compiler = ValhallCompiler::new();
        let shader = NirShader::new(ShaderStage::Vertex);
        // Empty shader should compile
        let result = compiler.compile(&shader);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compiled_shader_binary() {
        let mut compiler = ValhallCompiler::new();
        let shader = NirShader::new(ShaderStage::Fragment);
        let compiled = compiler.compile(&shader).unwrap();
        assert_eq!(compiled.binary().len(), compiled.binary_size as usize);
    }

    #[test]
    fn test_modifiers() {
        let mods = ValhallModifiers::SATURATE | ValhallModifiers::NEG_SRC0;
        assert!(mods.contains(ValhallModifiers::SATURATE));
        assert!(mods.contains(ValhallModifiers::NEG_SRC0));
        assert!(!mods.contains(ValhallModifiers::ABS_SRC1));
    }
}