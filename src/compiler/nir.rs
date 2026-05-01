//! NIR-like Intermediate Representation
//!
//! This module provides an NIR-like intermediate representation for
//! shader programs. NIR is the shader IR used by Mesa, and our
//! simplified version supports the same optimization passes.
//!
//! ## IR Structure
//!
//! - `NirShader`: Top-level shader object containing functions
//! - `NirFunction`: A single shader function (main, sub-functions)
//! - `NirBlock`: A basic block of instructions
//! - `NirInstr`: A single instruction with SSA destinations and sources

use smallvec::SmallVec;

/// Shader stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    /// Vertex shader
    Vertex,
    /// Tessellation control shader
    TessControl,
    /// Tessellation evaluation shader
    TessEval,
    /// Geometry shader
    Geometry,
    /// Fragment shader
    Fragment,
    /// Compute shader
    Compute,
}

impl ShaderStage {
    /// Get the stage name
    pub fn name(&self) -> &'static str {
        match self {
            ShaderStage::Vertex => "vertex",
            ShaderStage::TessControl => "tess_ctrl",
            ShaderStage::TessEval => "tess_eval",
            ShaderStage::Geometry => "geometry",
            ShaderStage::Fragment => "fragment",
            ShaderStage::Compute => "compute",
        }
    }
}

/// SSA value type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsaType {
    /// 32-bit float
    Float32,
    /// 16-bit float
    Float16,
    /// 32-bit integer
    Int32,
    /// 16-bit integer
    Int16,
    /// 8-bit integer
    Int8,
    /// Boolean
    Bool,
    /// Sampler
    Sampler,
    /// Texture
    Texture,
    /// Image
    Image,
    /// 64-bit float (not supported on Mali-G68)
    Float64,
    /// 64-bit integer (not supported on Mali-G68)
    Int64,
}

impl SsaType {
    /// Get the size in bytes
    pub fn size_bytes(&self) -> u32 {
        match self {
            SsaType::Float32 | SsaType::Int32 => 4,
            SsaType::Float16 | SsaType::Int16 => 2,
            SsaType::Int8 => 1,
            SsaType::Bool => 4,
            SsaType::Float64 | SsaType::Int64 => 8,
            _ => 4,
        }
    }

    /// Check if this type is supported on Mali-G68
    pub fn is_supported_on_mali_g68(&self) -> bool {
        !matches!(self, SsaType::Float64 | SsaType::Int64)
    }
}

/// SSA value reference (index + component)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SsaRef {
    /// Value index
    pub index: u32,
    /// Component (for vectors: 0=x, 1=y, 2=z, 3=w)
    pub comp: u8,
}

impl SsaRef {
    /// Create a new SSA reference
    pub fn new(index: u32) -> Self {
        Self { index, comp: 0 }
    }

    /// Create an SSA reference with a specific component
    pub fn with_comp(index: u32, comp: u8) -> Self {
        Self { index, comp }
    }
}

/// NIR instruction opcodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NirOp {
    // Arithmetic
    /// fadd: float addition
    FAdd,
    /// fsub: float subtraction
    FSub,
    /// fmul: float multiplication
    FMul,
    /// fdiv: float division
    FDiv,
    /// ffma: float fused multiply-add
    FFma,
    /// fneg: float negation
    FNeg,
    /// fabs: float absolute value
    FAbs,
    /// fsat: float saturate
    FSat,
    /// fmin: float minimum
    FMin,
    /// fmax: float maximum
    FMax,
    /// iadd: integer addition
    IAdd,
    /// isub: integer subtraction
    ISub,
    /// imul: integer multiplication
    IMul,
    /// imod: integer modulo
    IMod,
    /// ineg: integer negation
    INeg,
    /// iabs: integer absolute value
    IAbs,
    /// imin: integer minimum
    IMin,
    /// imax: integer maximum
    IMax,

    // Bitwise
    /// iand: bitwise AND
    IAnd,
    /// ior: bitwise OR
    IOr,
    /// ixor: bitwise XOR
    IXor,
    /// inot: bitwise NOT
    INot,
    /// ishl: shift left
    IShl,
    /// ishr: shift right (arithmetic)
    IShr,
    /// ishr_unsigned: shift right (logical)
    IShrUnsigned,

    // Comparison
    /// feq: float equal
    FEq,
    /// fne: float not equal
    FNe,
    /// flt: float less than
    FLt,
    /// fge: float greater than or equal
    FGe,
    /// ieq: integer equal
    IEq,
    /// ine: integer not equal
    INe,
    /// ilt: integer less than
    ILt,
    /// ige: integer greater than or equal
    IGe,
    /// ult: unsigned integer less than
    ULt,
    /// uge: unsigned integer greater than or equal
    UGe,

    // Conversion
    /// f2i: float to int
    F2I,
    /// i2f: int to float
    I2F,
    /// f2f: float to float (precision change)
    F2F,
    /// i2i: int to int (size change)
    I2I,
    /// b2f: bool to float
    B2F,
    /// b2i: bool to int
    B2I,

    // Selection
    /// bcsel: boolean conditional select
    Bcsel,

    // Interpolation
    /// load_interpolated_input: interpolated varying
    LoadInterpolatedInput,

    // Memory
    /// load_ubo: load from uniform buffer
    LoadUbo,
    /// load_ssbo: load from shader storage buffer
    LoadSsbo,
    /// store_ssbo: store to shader storage buffer
    StoreSsbo,
    /// load_push_constant: load push constant
    LoadPushConstant,
    /// load_shared: load from shared memory
    LoadShared,
    /// store_shared: store to shared memory
    StoreShared,

    // Texture
    /// tex: texture sampling
    Tex,
    /// txf: texel fetch
    Txf,
    /// txd: texture sampling with derivatives
    Txd,
    /// txb: texture sampling with bias
    Txb,
    /// txl: texture sampling with LOD
    Txl,
    /// txs: texture size query
    Txs,

    // I/O
    /// load_input: load vertex input
    LoadInput,
    /// store_output: store vertex/fragment output
    StoreOutput,
    /// load_output: load from previous fragment output (blending)
    LoadOutput,

    // Derivatives
    /// fddx: float derivative in X
    Fddx,
    /// fddy: float derivative in Y
    Fddy,
    /// fddx_fine: float derivative in X (fine)
    FddxFine,
    /// fddy_fine: float derivative in Y (fine)
    FddyFine,
    /// fddx_coarse: float derivative in X (coarse)
    FddxCoarse,
    /// fddy_coarse: float derivative in Y (coarse)
    FddyCoarse,

    // Control flow
    /// jump: unconditional jump
    Jump,
    /// branch: conditional branch
    Branch,
    /// phi: phi node (SSA merge)
    Phi,

    // Special
    /// nop: no operation
    Nop,
    /// discards: fragment discard
    Discard,
    /// demote: fragment demote to helper
    Demote,
    /// barrier: execution/memory barrier
    Barrier,
    /// emit_vertex: geometry shader vertex emit
    EmitVertex,
    /// end_primitive: geometry shader end primitive
    EndPrimitive,
    /// debug: debug marker
    Debug,
}

/// NIR instruction
#[derive(Debug, Clone)]
pub struct NirInstr {
    /// Instruction opcode
    pub op: NirOp,
    /// SSA destinations
    pub dests: SmallVec<[SsaRef; 2]>,
    /// SSA sources
    pub srcs: SmallVec<[SsaRef; 4]>,
    /// Source types (for type checking)
    pub src_types: SmallVec<[SsaType; 4]>,
    /// Destination type
    pub dest_type: Option<SsaType>,
    /// Constant values (for immediate operands)
    pub constants: SmallVec<[f32; 4]>,
    /// Whether this instruction can be predicated
    pub predicable: bool,
}

impl NirInstr {
    /// Create a new instruction
    pub fn new(op: NirOp) -> Self {
        Self {
            op,
            dests: SmallVec::new(),
            srcs: SmallVec::new(),
            src_types: SmallVec::new(),
            dest_type: None,
            constants: SmallVec::new(),
            predicable: false,
        }
    }

    /// Create a binary float operation (fadd, fmul, etc.)
    pub fn binop_float(op: NirOp, dest: SsaRef, src0: SsaRef, src1: SsaRef) -> Self {
        let mut instr = Self::new(op);
        instr.dests.push(dest);
        instr.srcs.push(src0);
        instr.srcs.push(src1);
        instr.dest_type = Some(SsaType::Float32);
        instr
    }

    /// Create a unary float operation (fneg, fabs, etc.)
    pub fn unop_float(op: NirOp, dest: SsaRef, src: SsaRef) -> Self {
        let mut instr = Self::new(op);
        instr.dests.push(dest);
        instr.srcs.push(src);
        instr.dest_type = Some(SsaType::Float32);
        instr
    }

    /// Create a constant load
    pub fn load_const(dest: SsaRef, value: f32) -> Self {
        let mut instr = Self::new(NirOp::FAdd); // Placeholder, actual load_const is different
        instr.dests.push(dest);
        instr.constants.push(value);
        instr.dest_type = Some(SsaType::Float32);
        instr
    }

    /// Create a texture sampling instruction
    pub fn tex(dest: SsaRef, coord: SsaRef, sampler: SsaRef, texture: SsaRef) -> Self {
        let mut instr = Self::new(NirOp::Tex);
        instr.dests.push(dest);
        instr.srcs.push(coord);
        instr.srcs.push(sampler);
        instr.srcs.push(texture);
        instr
    }

    /// Create a UBO load instruction
    pub fn load_ubo(dest: SsaRef, index: SsaRef, offset: SsaRef) -> Self {
        let mut instr = Self::new(NirOp::LoadUbo);
        instr.dests.push(dest);
        instr.srcs.push(index);
        instr.srcs.push(offset);
        instr.dest_type = Some(SsaType::Float32);
        instr
    }

    /// Get the number of sources
    pub fn num_srcs(&self) -> usize {
        self.srcs.len()
    }

    /// Get the number of destinations
    pub fn num_dests(&self) -> usize {
        self.dests.len()
    }
}

/// NIR basic block
#[derive(Debug, Clone)]
pub struct NirBlock {
    /// Block index
    pub index: u32,
    /// Instructions in this block
    pub instructions: Vec<NirInstr>,
    /// Predecessor block indices
    pub predecessors: Vec<u32>,
    /// Successor block indices
    pub successors: Vec<u32>,
}

impl NirBlock {
    /// Create a new basic block
    pub fn new(index: u32) -> Self {
        Self {
            index,
            instructions: Vec::new(),
            predecessors: Vec::new(),
            successors: Vec::new(),
        }
    }

    /// Push an instruction to this block
    pub fn push_instr(&mut self, instr: NirInstr) {
        self.instructions.push(instr);
    }

    /// Get the number of instructions
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    /// Check if the block is empty
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}

/// NIR function (shader entry point)
#[derive(Debug, Clone)]
pub struct NirFunction {
    /// Function name
    pub name: String,
    /// Shader stage
    pub stage: ShaderStage,
    /// Basic blocks (in CFG order)
    pub blocks: Vec<NirBlock>,
    /// Number of SSA values
    pub num_ssa_values: u32,
    /// Number of registers needed
    pub num_regs: u32,
    /// Local size (for compute shaders)
    pub local_size: [u32; 3],
    /// Input variables
    pub inputs: Vec<NirVariable>,
    /// Output variables
    pub outputs: Vec<NirVariable>,
    /// Uniform variables
    pub uniforms: Vec<NirVariable>,
}

/// NIR variable
#[derive(Debug, Clone)]
pub struct NirVariable {
    /// Variable name
    pub name: String,
    /// Variable type
    pub var_type: SsaType,
    /// Location (input/output index)
    pub location: u32,
    /// Number of components
    pub num_components: u8,
    /// Whether this variable is per-vertex
    pub per_vertex: bool,
    /// Descriptor set (for UBOs/SSBOs)
    pub descriptor_set: u32,
    /// Binding index
    pub binding: u32,
}

/// NIR Shader - the top-level shader object
#[derive(Debug, Clone)]
pub struct NirShader {
    /// Shader stage
    pub stage: ShaderStage,
    /// Functions (usually just one: main)
    pub functions: Vec<NirFunction>,
    /// Total SSA values across all functions
    pub total_ssa_values: u32,
    /// Whether this shader has been optimized
    pub optimized: bool,
    /// Whether this shader uses 16-bit math
    pub uses_fp16: bool,
    /// Whether this shader uses texture sampling
    pub uses_textures: bool,
    /// Whether this shader uses compute
    pub uses_compute: bool,
    /// Estimated instruction count (post-optimization)
    pub estimated_instr_count: u32,
}

impl NirShader {
    /// Create a new empty shader
    pub fn new(stage: ShaderStage) -> Self {
        Self {
            stage,
            functions: Vec::new(),
            total_ssa_values: 0,
            optimized: false,
            uses_fp16: false,
            uses_textures: false,
            uses_compute: stage == ShaderStage::Compute,
            estimated_instr_count: 0,
        }
    }

    /// Parse from SPIR-V binary
    pub fn from_spirv(spirv: &[u32]) -> Result<Self, NirError> {
        // In production, this uses a full SPIR-V parser
        // For now, we create a placeholder shader
        if spirv.len() < 5 {
            return Err(NirError::InvalidSpirv("SPIR-V too short".to_string()));
        }

        // Check SPIR-V magic
        if spirv[0] != 0x07230203 {
            return Err(NirError::InvalidSpirv("Bad SPIR-V magic".to_string()));
        }

        Ok(Self::new(ShaderStage::Vertex))
    }

    /// Get the main function
    pub fn main_function(&self) -> Option<&NirFunction> {
        self.functions.first()
    }

    /// Get the total instruction count
    pub fn instr_count(&self) -> usize {
        self.functions
            .iter()
            .map(|f| f.blocks.iter().map(|b| b.len()).sum::<usize>())
            .sum()
    }

    /// Mark the shader as optimized
    pub fn mark_optimized(&mut self) {
        self.optimized = true;
    }
}

/// NIR errors
#[derive(Debug, thiserror::Error)]
pub enum NirError {
    /// Invalid SPIR-V
    #[error("Invalid SPIR-V: {0}")]
    InvalidSpirv(String),
    /// Unsupported feature
    #[error("Unsupported feature: {0}")]
    Unsupported(String),
    /// Type error
    #[error("Type error: {0}")]
    TypeError(String),
    /// Register allocation failure
    #[error("Register allocation failed: {0}")]
    RegAllocFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nir_shader_creation() {
        let shader = NirShader::new(ShaderStage::Vertex);
        assert_eq!(shader.stage, ShaderStage::Vertex);
        assert!(!shader.optimized);
    }

    #[test]
    fn test_nir_instr_binop() {
        let dest = SsaRef::new(0);
        let src0 = SsaRef::new(1);
        let src1 = SsaRef::new(2);
        let instr = NirInstr::binop_float(NirOp::FAdd, dest, src0, src1);
        assert_eq!(instr.op, NirOp::FAdd);
        assert_eq!(instr.num_srcs(), 2);
        assert_eq!(instr.num_dests(), 1);
    }

    #[test]
    fn test_nir_block() {
        let mut block = NirBlock::new(0);
        assert!(block.is_empty());
        let instr = NirInstr::unop_float(NirOp::FNeg, SsaRef::new(0), SsaRef::new(1));
        block.push_instr(instr);
        assert_eq!(block.len(), 1);
    }

    #[test]
    fn test_spirv_magic_validation() {
        let bad_spirv = [0xDEADBEEFu32];
        let result = NirShader::from_spirv(&bad_spirv);
        assert!(result.is_err());
    }

    #[test]
    fn test_ssa_type_support() {
        assert!(SsaType::Float32.is_supported_on_mali_g68());
        assert!(SsaType::Float16.is_supported_on_mali_g68());
        assert!(!SsaType::Float64.is_supported_on_mali_g68());
    }
}