pub const VOXEL_PRECISION: u32 = 1;
pub const VOXEL_SIZE: f32 = 1.0 / VOXEL_PRECISION as f32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VoxelFace {
    NegativeX = 0, // Left face
    PositiveX = 1, // Right face
    NegativeY = 2, // Bottom face
    PositiveY = 3, // Top face
    NegativeZ = 4, // Back face
    PositiveZ = 5, // Front face
}

impl VoxelFace {
    pub fn get_normal(&self) -> bevy::prelude::Vec3 {
        use bevy::prelude::Vec3;
        match self {
            VoxelFace::NegativeX => Vec3::new(-1.0, 0.0, 0.0),
            VoxelFace::PositiveX => Vec3::new(1.0, 0.0, 0.0),
            VoxelFace::NegativeY => Vec3::new(0.0, -1.0, 0.0),
            VoxelFace::PositiveY => Vec3::new(0.0, 1.0, 0.0),
            VoxelFace::NegativeZ => Vec3::new(0.0, 0.0, -1.0),
            VoxelFace::PositiveZ => Vec3::new(0.0, 0.0, 1.0),
        }
    }
    
    pub fn get_offset(&self) -> (i32, i32, i32) {
        match self {
            VoxelFace::NegativeX => (-1, 0, 0),
            VoxelFace::PositiveX => (1, 0, 0),
            VoxelFace::NegativeY => (0, -1, 0),
            VoxelFace::PositiveY => (0, 1, 0),
            VoxelFace::NegativeZ => (0, 0, -1),
            VoxelFace::PositiveZ => (0, 0, 1),
        }
    }
    
    pub fn from_normal(normal: bevy::prelude::Vec3) -> Option<Self> {
        const EPSILON: f32 = 0.5;
        
        if normal.x < -EPSILON {
            Some(VoxelFace::NegativeX)
        } else if normal.x > EPSILON {
            Some(VoxelFace::PositiveX)
        } else if normal.y < -EPSILON {
            Some(VoxelFace::NegativeY)
        } else if normal.y > EPSILON {
            Some(VoxelFace::PositiveY)
        } else if normal.z < -EPSILON {
            Some(VoxelFace::NegativeZ)
        } else if normal.z > EPSILON {
            Some(VoxelFace::PositiveZ)
        } else {
            None
        }
    }
    
    pub fn get_vertices(&self, pos: bevy::prelude::Vec3, size: f32) -> [[f32; 3]; 4] {
        use bevy::prelude::Vec3;
        let Vec3 { x, y, z } = pos;
        
        // 确保所有面都使用正确的逆时针绕序（从外部看向面时）
        match self {
            VoxelFace::NegativeX => [
                // Left face (-X) - 从外部看逆时针
                [x, y, z + size],
                [x, y + size, z + size],
                [x, y + size, z],
                [x, y, z],
            ],
            VoxelFace::PositiveX => [
                // Right face (+X) - 从外部看逆时针
                [x + size, y, z],
                [x + size, y + size, z],
                [x + size, y + size, z + size],
                [x + size, y, z + size],
            ],
            VoxelFace::NegativeY => [
                // Bottom face (-Y) - 从外部看逆时针
                [x, y, z],
                [x + size, y, z],
                [x + size, y, z + size],
                [x, y, z + size],
            ],
            VoxelFace::PositiveY => [
                // Top face (+Y) - 从外部看逆时针
                [x, y + size, z + size],
                [x + size, y + size, z + size],
                [x + size, y + size, z],
                [x, y + size, z],
            ],
            VoxelFace::NegativeZ => [
                // Back face (-Z) - 从外部看逆时针
                [x, y, z],
                [x, y + size, z],
                [x + size, y + size, z],
                [x + size, y, z],
            ],
            VoxelFace::PositiveZ => [
                // Front face (+Z) - 从外部看逆时针
                [x + size, y, z + size],
                [x + size, y + size, z + size],
                [x, y + size, z + size],
                [x, y, z + size],
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VoxelType {
    Air,
    Stone,
    Dirt,
    Grass,
}

impl Default for VoxelType {
    fn default() -> Self {
        VoxelType::Air
    }
}

impl VoxelType {
    pub fn is_solid(&self) -> bool {
        match self {
            VoxelType::Air => false,
            _ => true,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Voxel {
    pub voxel_type: VoxelType,
}

impl Default for Voxel {
    fn default() -> Self {
        Self {
            voxel_type: VoxelType::Air,
        }
    }
}

impl Voxel {
    pub fn new(voxel_type: VoxelType) -> Self {
        Self { voxel_type }
    }
    
    pub fn is_solid(&self) -> bool {
        self.voxel_type.is_solid()
    }
}
