use serde::Serialize;

pub(crate) mod message_parser;

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


