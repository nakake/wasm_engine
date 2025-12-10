use bytemuck::{Pod, Zeroable};
use wgpu::{VertexBufferLayout, VertexAttribute, VertexFormat, VertexStepMode, BufferAddress};

/// 頂点構造体
/// 位置、法線、色を含む
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    /// 新しい頂点を作成
    pub fn new(position: [f32; 3], normal: [f32; 3], color: [f32; 3]) -> Self {
        Self { position, normal, color }
    }

    /// 頂点バッファレイアウトを取得
    pub fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                // position
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                // normal
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x3,
                },
                // color
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Float32x3,
                },
            ],
        }
    }
}

/// メッシュ構造体
/// 頂点とインデックスを含む
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    /// 空のメッシュを作成
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// キューブメッシュを生成（1x1x1、原点中心）
    pub fn cube() -> Self {
        // 各面に異なる色を設定（視認性のため）
        let colors = [
            [1.0, 0.0, 0.0], // +X red
            [0.0, 1.0, 1.0], // -X cyan
            [0.0, 1.0, 0.0], // +Y green
            [1.0, 0.0, 1.0], // -Y magenta
            [0.0, 0.0, 1.0], // +Z blue
            [1.0, 1.0, 0.0], // -Z yellow
        ];

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // +X face (right)
        let base = vertices.len() as u32;
        vertices.extend_from_slice(&[
            Vertex::new([0.5, -0.5, -0.5], [1.0, 0.0, 0.0], colors[0]),
            Vertex::new([0.5,  0.5, -0.5], [1.0, 0.0, 0.0], colors[0]),
            Vertex::new([0.5,  0.5,  0.5], [1.0, 0.0, 0.0], colors[0]),
            Vertex::new([0.5, -0.5,  0.5], [1.0, 0.0, 0.0], colors[0]),
        ]);
        indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

        // -X face (left)
        let base = vertices.len() as u32;
        vertices.extend_from_slice(&[
            Vertex::new([-0.5, -0.5,  0.5], [-1.0, 0.0, 0.0], colors[1]),
            Vertex::new([-0.5,  0.5,  0.5], [-1.0, 0.0, 0.0], colors[1]),
            Vertex::new([-0.5,  0.5, -0.5], [-1.0, 0.0, 0.0], colors[1]),
            Vertex::new([-0.5, -0.5, -0.5], [-1.0, 0.0, 0.0], colors[1]),
        ]);
        indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

        // +Y face (top)
        let base = vertices.len() as u32;
        vertices.extend_from_slice(&[
            Vertex::new([-0.5, 0.5, -0.5], [0.0, 1.0, 0.0], colors[2]),
            Vertex::new([-0.5, 0.5,  0.5], [0.0, 1.0, 0.0], colors[2]),
            Vertex::new([ 0.5, 0.5,  0.5], [0.0, 1.0, 0.0], colors[2]),
            Vertex::new([ 0.5, 0.5, -0.5], [0.0, 1.0, 0.0], colors[2]),
        ]);
        indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

        // -Y face (bottom)
        let base = vertices.len() as u32;
        vertices.extend_from_slice(&[
            Vertex::new([-0.5, -0.5,  0.5], [0.0, -1.0, 0.0], colors[3]),
            Vertex::new([-0.5, -0.5, -0.5], [0.0, -1.0, 0.0], colors[3]),
            Vertex::new([ 0.5, -0.5, -0.5], [0.0, -1.0, 0.0], colors[3]),
            Vertex::new([ 0.5, -0.5,  0.5], [0.0, -1.0, 0.0], colors[3]),
        ]);
        indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

        // +Z face (front)
        let base = vertices.len() as u32;
        vertices.extend_from_slice(&[
            Vertex::new([-0.5, -0.5, 0.5], [0.0, 0.0, 1.0], colors[4]),
            Vertex::new([ 0.5, -0.5, 0.5], [0.0, 0.0, 1.0], colors[4]),
            Vertex::new([ 0.5,  0.5, 0.5], [0.0, 0.0, 1.0], colors[4]),
            Vertex::new([-0.5,  0.5, 0.5], [0.0, 0.0, 1.0], colors[4]),
        ]);
        indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

        // -Z face (back)
        let base = vertices.len() as u32;
        vertices.extend_from_slice(&[
            Vertex::new([ 0.5, -0.5, -0.5], [0.0, 0.0, -1.0], colors[5]),
            Vertex::new([-0.5, -0.5, -0.5], [0.0, 0.0, -1.0], colors[5]),
            Vertex::new([-0.5,  0.5, -0.5], [0.0, 0.0, -1.0], colors[5]),
            Vertex::new([ 0.5,  0.5, -0.5], [0.0, 0.0, -1.0], colors[5]),
        ]);
        indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

        Self { vertices, indices }
    }

    /// 頂点数を取得
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// インデックス数を取得
    pub fn index_count(&self) -> usize {
        self.indices.len()
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_new() {
        let v = Vertex::new([1.0, 2.0, 3.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]);
        assert_eq!(v.position, [1.0, 2.0, 3.0]);
        assert_eq!(v.normal, [0.0, 1.0, 0.0]);
        assert_eq!(v.color, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_vertex_size() {
        // 3 floats * 3 attributes * 4 bytes = 36 bytes
        assert_eq!(std::mem::size_of::<Vertex>(), 36);
    }

    #[test]
    fn test_cube_vertices() {
        let cube = Mesh::cube();
        // 6 faces * 4 vertices = 24 vertices
        assert_eq!(cube.vertex_count(), 24);
    }

    #[test]
    fn test_cube_indices() {
        let cube = Mesh::cube();
        // 6 faces * 2 triangles * 3 indices = 36 indices
        assert_eq!(cube.index_count(), 36);
    }

    #[test]
    fn test_empty_mesh() {
        let mesh = Mesh::new();
        assert_eq!(mesh.vertex_count(), 0);
        assert_eq!(mesh.index_count(), 0);
    }
}
