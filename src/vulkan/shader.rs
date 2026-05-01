//! Vulkan shader module

use crate::compiler::nir::{NirShader, ShaderStage};
use crate::compiler::valhall::{ValhallCompiler, CompiledShader, CompilerError};
use crate::compiler::optimize::{optimize_shader, OptLevel};
use crate::compiler::emulator_pass::optimize_for_emulator;
use crate::emulator::cache::hash_spirv;
use crate::LOG_TARGET;
use log::debug;

/// Vulkan shader module
pub struct VkShaderModule {
    /// SPIR-V bytecode
    spirv: Vec<u32>,
    /// SPIR-V hash
    spirv_hash: u64,
    /// Shader stage (if single-stage module)
    stage: Option<ShaderStage>,
    /// Compiled shader (lazy)
    compiled: Option<CompiledShader>,
}

impl VkShaderModule {
    /// Create a shader module from SPIR-V bytecode
    pub fn from_spirv(spirv: Vec<u32>) -> Result<Self, ShaderError> {
        if spirv.len() < 5 {
            return Err(ShaderError::InvalidSpirv("SPIR-V too short".to_string()));
        }
        if spirv[0] != 0x07230203 {
            return Err(ShaderError::InvalidSpirv("Bad SPIR-V magic".to_string()));
        }

        let spirv_hash = hash_spirv(&spirv);
        Ok(Self {
            spirv,
            spirv_hash,
            stage: None,
            compiled: None,
        })
    }

    /// Compile the shader module for a specific stage
    pub fn compile(&mut self, _stage: ShaderStage, opt_level: OptLevel) -> Result<CompiledShader, ShaderError> {
        // Parse SPIR-V to NIR
        let mut nir = NirShader::from_spirv(&self.spirv)?;

        // Run standard optimization passes
        optimize_shader(&mut nir, opt_level);

        // Run emulator-specific optimization passes
        optimize_for_emulator(&mut nir, opt_level);

        // Compile NIR to Valhall ISA
        let mut compiler = ValhallCompiler::new();
        let compiled = compiler.compile(&nir)?;

        debug!(
            target: LOG_TARGET,
            "Shader compiled: {} instructions, {} bytes, {} GPRs",
            compiled.num_instructions(),
            compiled.binary_size,
            compiled.num_gprs
        );

        self.compiled = Some(compiled.clone());
        Ok(compiled)
    }

    /// Get the SPIR-V hash
    pub fn spirv_hash(&self) -> u64 {
        self.spirv_hash
    }
}

/// Shader errors
#[derive(Debug, thiserror::Error)]
pub enum ShaderError {
    /// Invalid SPIR-V
    #[error("Invalid SPIR-V: {0}")]
    InvalidSpirv(String),
    /// Compilation failed
    #[error("Shader compilation failed: {0}")]
    CompilationFailed(#[from] CompilerError),
    /// NIR error
    #[error("NIR error: {0}")]
    NirError(#[from] crate::compiler::nir::NirError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_module_invalid_magic() {
        let result = VkShaderModule::from_spirv(vec![0xDEADBEEF]);
        assert!(result.is_err());
    }

    #[test]
    fn test_shader_module_too_short() {
        let result = VkShaderModule::from_spirv(vec![0x07230203]);
        assert!(result.is_err());
    }
}