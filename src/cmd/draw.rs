//! Draw command recording
//!
//! Records draw commands into the CSF command stream for the
//! Mali-G68 MP5 Valhall graphics pipeline.

use crate::csf::queue::CsfPacketType;

/// Draw call parameters for non-indexed draws
#[derive(Debug, Clone, Copy)]
pub struct DrawInfo {
    /// Number of vertices to draw
    pub vertex_count: u32,
    /// Number of instances to draw
    pub instance_count: u32,
    /// First vertex index
    pub first_vertex: u32,
    /// First instance index
    pub first_instance: u32,
}

impl Default for DrawInfo {
    fn default() -> Self {
        Self {
            vertex_count: 3,
            instance_count: 1,
            first_vertex: 0,
            first_instance: 0,
        }
    }
}

/// Draw call parameters for indexed draws
#[derive(Debug, Clone, Copy)]
pub struct DrawIndexedInfo {
    /// Number of indices to draw
    pub index_count: u32,
    /// Number of instances to draw
    pub instance_count: u32,
    /// First index offset
    pub first_index: u32,
    /// Vertex offset added to index
    pub vertex_offset: i32,
    /// First instance index
    pub first_instance: u32,
    /// Index buffer GPU address
    pub index_buf_addr: u64,
    /// Index type (16-bit or 32-bit)
    pub index_type: IndexType,
}

/// Index buffer element type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexType {
    /// 16-bit indices (uint16)
    U16,
    /// 32-bit indices (uint32)
    U32,
}

impl IndexType {
    /// Get the size in bytes per index
    pub fn size_bytes(&self) -> u32 {
        match self {
            IndexType::U16 => 2,
            IndexType::U32 => 4,
        }
    }
}

/// Primitive topology
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveTopology {
    /// Point list
    PointList,
    /// Line list
    LineList,
    /// Line strip
    LineStrip,
    /// Triangle list
    TriangleList,
    /// Triangle strip
    TriangleStrip,
    /// Triangle fan
    TriangleFan,
    /// Line list with adjacency
    LineListAdj,
    /// Line strip with adjacency
    LineStripAdj,
    /// Triangle list with adjacency
    TriangleListAdj,
    /// Triangle strip with adjacency
    TriangleStripAdj,
    /// Patch list (for tessellation)
    PatchList,
}

impl PrimitiveTopology {
    /// Get the Valhall hardware primitive mode encoding
    pub fn valhall_mode(&self) -> u32 {
        match self {
            PrimitiveTopology::PointList => 0,
            PrimitiveTopology::LineList => 1,
            PrimitiveTopology::LineStrip => 2,
            PrimitiveTopology::TriangleList => 3,
            PrimitiveTopology::TriangleStrip => 4,
            PrimitiveTopology::TriangleFan => 5,
            PrimitiveTopology::LineListAdj => 6,
            PrimitiveTopology::LineStripAdj => 7,
            PrimitiveTopology::TriangleListAdj => 8,
            PrimitiveTopology::TriangleStripAdj => 9,
            PrimitiveTopology::PatchList => 10,
        }
    }

    /// Get the number of vertices per primitive
    pub fn vertices_per_primitive(&self) -> u32 {
        match self {
            PrimitiveTopology::PointList => 1,
            PrimitiveTopology::LineList | PrimitiveTopology::LineStrip => 2,
            PrimitiveTopology::TriangleList | PrimitiveTopology::TriangleStrip | PrimitiveTopology::TriangleFan => 3,
            _ => 1,
        }
    }
}

/// Vertex attribute description
#[derive(Debug, Clone, Copy)]
pub struct VertexAttributeDesc {
    /// Binding index
    pub binding: u32,
    /// Location in the shader
    pub location: u32,
    /// Format
    pub format: VertexFormat,
    /// Offset within the vertex buffer
    pub offset: u32,
}

/// Vertex buffer format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexFormat {
    /// 1 x 32-bit float
    R32Sfloat,
    /// 2 x 32-bit float
    R32G32Sfloat,
    /// 3 x 32-bit float
    R32G32B32Sfloat,
    /// 4 x 32-bit float
    R32G32B32A32Sfloat,
    /// 4 x 8-bit unsigned normalized
    R8G8B8A8Unorm,
    /// 4 x 8-bit signed normalized
    R8G8B8A8Snorm,
    /// 2 x 16-bit float
    R16G16Sfloat,
    /// 4 x 16-bit float
    R16G16B16A16Sfloat,
    /// 32-bit unsigned integer
    R32Uint,
    /// 2 x 32-bit unsigned integer
    R32G32Uint,
    /// 4 x 32-bit unsigned integer
    R32G32B32A32Uint,
}

impl VertexFormat {
    /// Get the size in bytes
    pub fn size_bytes(&self) -> u32 {
        match self {
            VertexFormat::R32Sfloat => 4,
            VertexFormat::R32G32Sfloat => 8,
            VertexFormat::R32G32B32Sfloat => 12,
            VertexFormat::R32G32B32A32Sfloat => 16,
            VertexFormat::R8G8B8A8Unorm | VertexFormat::R8G8B8A8Snorm => 4,
            VertexFormat::R16G16Sfloat => 4,
            VertexFormat::R16G16B16A16Sfloat => 8,
            VertexFormat::R32Uint => 4,
            VertexFormat::R32G32Uint => 8,
            VertexFormat::R32G32B32A32Uint => 16,
        }
    }

    /// Get the number of components
    pub fn num_components(&self) -> u32 {
        match self {
            VertexFormat::R32Sfloat | VertexFormat::R32Uint => 1,
            VertexFormat::R32G32Sfloat | VertexFormat::R16G16Sfloat | VertexFormat::R32G32Uint => 2,
            VertexFormat::R32G32B32Sfloat => 3,
            VertexFormat::R32G32B32A32Sfloat
            | VertexFormat::R8G8B8A8Unorm
            | VertexFormat::R8G8B8A8Snorm
            | VertexFormat::R16G16B16A16Sfloat
            | VertexFormat::R32G32B32A32Uint => 4,
        }
    }
}

/// Vertex buffer binding description
#[derive(Debug, Clone, Copy)]
pub struct VertexBindingDesc {
    /// Binding index
    pub binding: u32,
    /// Stride between vertices in bytes
    pub stride: u32,
    /// Input rate (per-vertex or per-instance)
    pub input_rate: VertexInputRate,
    /// GPU address of the vertex buffer
    pub gpu_addr: u64,
    /// Size of the vertex buffer in bytes
    pub size: u64,
}

/// Vertex input rate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexInputRate {
    /// One attribute per vertex
    Vertex,
    /// One attribute per instance
    Instance,
}

/// Encode a draw command into CSF command stream words
pub fn encode_draw_cmd(info: &DrawInfo) -> [u32; 6] {
    [
        (CsfPacketType::Draw as u32) | (5 << 8), // header
        info.vertex_count,
        info.instance_count,
        info.first_vertex,
        info.first_instance,
        0, // padding
    ]
}

/// Encode an indexed draw command into CSF command stream words
pub fn encode_draw_indexed_cmd(info: &DrawIndexedInfo) -> [u32; 8] {
    let index_type_flag = if info.index_type == IndexType::U32 { 1u32 } else { 0 };
    [
        (CsfPacketType::DrawIndexed as u32) | (7 << 8), // header
        info.index_count,
        info.instance_count,
        info.first_index,
        info.vertex_offset as u32,
        info.first_instance,
        (info.index_buf_addr & 0xFFFFFFFF) as u32,
        ((info.index_buf_addr >> 32) & 0xFFFFFFFF) as u32 | (index_type_flag << 31),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_info_default() {
        let info = DrawInfo::default();
        assert_eq!(info.vertex_count, 3);
        assert_eq!(info.instance_count, 1);
    }

    #[test]
    fn test_primitive_topology() {
        assert_eq!(PrimitiveTopology::TriangleList.valhall_mode(), 3);
        assert_eq!(PrimitiveTopology::TriangleList.vertices_per_primitive(), 3);
    }

    #[test]
    fn test_vertex_format_sizes() {
        assert_eq!(VertexFormat::R32G32B32A32Sfloat.size_bytes(), 16);
        assert_eq!(VertexFormat::R8G8B8A8Unorm.size_bytes(), 4);
        assert_eq!(VertexFormat::R16G16B16A16Sfloat.size_bytes(), 8);
    }

    #[test]
    fn test_index_type_size() {
        assert_eq!(IndexType::U16.size_bytes(), 2);
        assert_eq!(IndexType::U32.size_bytes(), 4);
    }

    #[test]
    fn test_encode_draw_cmd() {
        let info = DrawInfo {
            vertex_count: 6,
            instance_count: 1,
            first_vertex: 0,
            first_instance: 0,
        };
        let words = encode_draw_cmd(&info);
        assert_eq!(words[1], 6); // vertex_count
        assert_eq!(words[2], 1); // instance_count
    }
}