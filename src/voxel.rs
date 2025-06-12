pub const VOXEL_PRECISION: u32 = 1;
pub const VOXEL_SIZE: f32 = 1.0 / VOXEL_PRECISION as f32;

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
