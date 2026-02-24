use serde::Serialize;

pub(crate) mod message_parser;
pub mod coordinate_converter;
pub mod coordinate_converter_v2;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ObjectData {
    pub object_id: String,
    pub camera_id: i32,
    pub timestamp: String,
    pub class_type: String, // e.g., "static", "dynamic"
    pub confidence: f32,
    pub center_pixel: [f32; 2],
    pub corners_pixel: [[f32; 2]; 4],
    pub center_real: Option<[f32; 2]>,
    pub corners_real: Option<[[f32; 2]; 4]>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AprilTagData {
    pub tag_id: String,
    pub camera_id: i32,
    pub timestamp: String,
    pub yaw: f32,
    pub center_pixel: [f32; 2],
    pub corners_pixel: [[f32; 2]; 4],
    pub center_real: Option<[f32; 2]>,
    pub corners_real: Option<[[f32; 2]; 4]>,
}

pub trait ConvertibleData {
    fn camera_id(&self) -> i32;
    fn center_pixel(&self) -> [f32; 2];
    fn corners_pixel(&self) -> &[[f32; 2]; 4];
    fn set_real_coords(&mut self, center: Option<[f32; 2]>, corners: Option<[[f32; 2]; 4]>);
}

impl ConvertibleData for ObjectData {
    fn camera_id(&self) -> i32 { self.camera_id }
    fn center_pixel(&self) -> [f32; 2] { self.center_pixel }
    fn corners_pixel(&self) -> &[[f32; 2]; 4] { &self.corners_pixel }
    fn set_real_coords(&mut self, center: Option<[f32; 2]>, corners: Option<[[f32; 2]; 4]>) {
        if let Some(c) = center { self.center_real = Some(c); }
        if let Some(c) = corners { self.corners_real = Some(c); }
    }
}

impl ConvertibleData for AprilTagData {
    fn camera_id(&self) -> i32 { self.camera_id }
    fn center_pixel(&self) -> [f32; 2] { self.center_pixel }
    fn corners_pixel(&self) -> &[[f32; 2]; 4] { &self.corners_pixel }
    fn set_real_coords(&mut self, center: Option<[f32; 2]>, corners: Option<[[f32; 2]; 4]>) {
        if let Some(c) = center { self.center_real = Some(c); }
        if let Some(c) = corners { self.corners_real = Some(c); }
    }
}


