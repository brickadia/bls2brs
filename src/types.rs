pub type BrickMapping = Vec<BrickDesc>;

#[derive(Debug, Clone)]
pub struct BrickDesc {
    pub asset: &'static str,
    pub size: (u32, u32, u32),
    pub offset: (i32, i32, i32),
    pub rotation_offset: u8,
    pub color_override: Option<brs::Color>,
    pub direction_override: Option<brs::Direction>,
}

impl BrickDesc {
    pub const fn new(asset: &'static str) -> Self {
        Self {
            asset,
            size: (0, 0, 0),
            offset: (0, 0, 0),
            rotation_offset: 1,
            color_override: None,
            direction_override: None,
        }
    }

    pub fn size(mut self, size: (u32, u32, u32)) -> Self {
        self.size = size;
        self
    }

    pub fn offset(mut self, offset: (i32, i32, i32)) -> Self {
        self.offset = offset;
        self
    }

    pub fn rotation_offset(mut self, rotation: u8) -> Self {
        self.rotation_offset = rotation;
        self
    }

    pub fn color_override(mut self, color_override: brs::Color) -> Self {
        self.color_override = Some(color_override);
        self
    }

    pub fn direction_override(mut self, direction_override: brs::Direction) -> Self {
        self.direction_override = Some(direction_override);
        self
    }
}

pub trait AsBrickDescVec<'s> {
    fn as_brick_mapping_vec(self) -> BrickMapping;
}

impl<'s> AsBrickDescVec<'s> for BrickDesc {
    fn as_brick_mapping_vec(self) -> BrickMapping {
        vec![self]
    }
}

impl<'s> AsBrickDescVec<'s> for BrickMapping {
    fn as_brick_mapping_vec(self) -> BrickMapping {
        self
    }
}
