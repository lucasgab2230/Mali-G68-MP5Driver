//! Vulkan buffer management

/// Buffer usage flags
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BufferUsageFlags: u32 {
        const TRANSFER_SRC = 1 << 0;
        const TRANSFER_DST = 1 << 1;
        const UNIFORM_TEXEL_BUFFER = 1 << 2;
        const STORAGE_TEXEL_BUFFER = 1 << 3;
        const UNIFORM_BUFFER = 1 << 4;
        const STORAGE_BUFFER = 1 << 5;
        const INDEX_BUFFER = 1 << 6;
        const VERTEX_BUFFER = 1 << 7;
        const INDIRECT_BUFFER = 1 << 8;
        const SHADER_DEVICE_ADDRESS = 1 << 9;
    }
}

/// Vulkan buffer
pub struct VkBuffer {
    /// GPU address
    gpu_addr: u64,
    /// Buffer size
    size: u64,
    /// Usage flags
    usage: BufferUsageFlags,
    /// Bound memory
    memory: Option<super::memory::VkDeviceMemory>,
}

impl VkBuffer {
    /// Create a new buffer
    pub fn new(size: u64, usage: BufferUsageFlags) -> Self {
        Self { gpu_addr: 0, size, usage, memory: None }
    }

    /// Get the GPU address
    pub fn gpu_addr(&self) -> u64 { self.gpu_addr }

    /// Get the buffer size
    pub fn size(&self) -> u64 { self.size }

    /// Bind memory to the buffer
    pub fn bind_memory(&mut self, memory: super::memory::VkDeviceMemory) {
        self.memory = Some(memory);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_creation() {
        let buf = VkBuffer::new(1024, BufferUsageFlags::VERTEX_BUFFER | BufferUsageFlags::TRANSFER_DST);
        assert_eq!(buf.size(), 1024);
        assert!(buf.usage.contains(BufferUsageFlags::VERTEX_BUFFER));
    }
}