//! Compute dispatch command recording
//!
//! Records compute dispatch commands for the CSF command stream.
//! Compute shaders are critical for emulator texture decoding
//! (BCn, ASTC, ETC2) and async compute work.

use crate::csf::queue::CsfPacketType;

/// Compute dispatch parameters
#[derive(Debug, Clone, Copy)]
pub struct DispatchInfo {
    /// Workgroup count in X dimension
    pub group_count_x: u32,
    /// Workgroup count in Y dimension
    pub group_count_y: u32,
    /// Workgroup count in Z dimension
    pub group_count_z: u32,
}

impl DispatchInfo {
    /// Create a new dispatch info
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Self {
            group_count_x: x,
            group_count_y: y,
            group_count_z: z,
        }
    }

    /// Create a 1D dispatch
    pub fn linear(group_count: u32) -> Self {
        Self::new(group_count, 1, 1)
    }

    /// Create a 2D dispatch (common for texture operations)
    pub fn twod(x: u32, y: u32) -> Self {
        Self::new(x, y, 1)
    }

    /// Total number of workgroups
    pub fn total_workgroups(&self) -> u32 {
        self.group_count_x * self.group_count_y * self.group_count_z
    }
}

/// Compute pipeline local size
#[derive(Debug, Clone, Copy)]
pub struct LocalSize {
    /// Workgroup size in X
    pub x: u32,
    /// Workgroup size in Y
    pub y: u32,
    /// Workgroup size in Z
    pub z: u32,
}

impl LocalSize {
    /// Create a new local size
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// Total invocations per workgroup
    pub fn total(&self) -> u32 {
        self.x * self.y * self.z
    }

    /// Get the optimal local size for Mali-G68 Valhall
    ///
    /// Valhall uses wavefronts of 8 threads (W8).
    /// Best performance when workgroup size is a multiple of 8.
    pub fn optimal_for_mali_g68(dimensions: u32) -> Self {
        match dimensions {
            1 => Self::new(64, 1, 1), // Good for linear texture decode
            2 => Self::new(8, 8, 1),  // Good for 2D texture decode
            3 => Self::new(4, 4, 4),  // Good for 3D operations
            _ => Self::new(64, 1, 1),
        }
    }

    /// Get optimal local size for texture decode on Mali-G68
    ///
    /// For texture decode, we want to maximize shared memory usage
    /// and wavefront occupancy.
    pub fn optimal_for_texture_decode(block_width: u32, block_height: u32) -> Self {
        // Each thread decodes one compressed block
        // Typical block sizes: 4x4 (BC1-BC7), 6x6 (ASTC), 4x4 (ETC2)
        let _threads_per_block = (block_width * block_height).min(8);
        Self::new(8, 1, 1) // One wavefront, 8 threads
    }
}

/// Encode a compute dispatch command into CSF command stream words
pub fn encode_dispatch_cmd(info: &DispatchInfo) -> [u32; 5] {
    [
        (CsfPacketType::Dispatch as u32) | (4 << 8), // header
        info.group_count_x,
        info.group_count_y,
        info.group_count_z,
        0, // reserved
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_info_creation() {
        let info = DispatchInfo::new(4, 4, 1);
        assert_eq!(info.total_workgroups(), 16);
    }

    #[test]
    fn test_dispatch_linear() {
        let info = DispatchInfo::linear(64);
        assert_eq!(info.group_count_x, 64);
        assert_eq!(info.group_count_y, 1);
    }

    #[test]
    fn test_local_size_total() {
        let size = LocalSize::new(8, 8, 1);
        assert_eq!(size.total(), 64);
    }

    #[test]
    fn test_optimal_local_size() {
        let size = LocalSize::optimal_for_mali_g68(2);
        assert_eq!(size.x, 8);
        assert_eq!(size.y, 8);
        assert!(size.total() % 8 == 0); // Wavefront-aligned
    }

    #[test]
    fn test_encode_dispatch() {
        let info = DispatchInfo::new(4, 2, 1);
        let words = encode_dispatch_cmd(&info);
        assert_eq!(words[1], 4); // group_count_x
        assert_eq!(words[2], 2); // group_count_y
        assert_eq!(words[3], 1); // group_count_z
    }
}