pub mod mesh;
pub mod camera;
pub mod picking;
pub mod gizmo;

pub use mesh::{Mesh, Vertex};
pub use camera::{Camera, CameraUniform};
pub use picking::{Ray, AABB};
pub use gizmo::{
    GizmoMode, GizmoAxis, GizmoState, GizmoVertex,
    create_arrow_vertices, create_plane_vertices, create_circle_vertices,
    create_scale_axis_vertices, create_center_box_vertices,
};

// Re-export glam types for consistent version usage
pub use glam;
