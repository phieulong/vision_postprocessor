use serde::Deserialize;
use log::{error};
use crate::modules::{ObjectData, AprilTagData};

pub struct ParseResult {
    pub success: bool,
    pub object_count: usize,
    pub apriltag_count: usize,
}

impl Default for ParseResult {
    fn default() -> Self {
        Self {
             success: false, object_count: 0, apriltag_count: 0
        }
    }
}

#[derive(Deserialize, Debug)]
struct RawObject {
    #[serde(alias = "id")]
    object_id: Option<String>,
    camera_id: Option<i32>,
    timestamp: Option<String>,
    #[serde(alias = "class")]
    class_type: Option<String>,
    #[serde(alias = "conf")]
    confidence: Option<f32>,
    #[serde(alias = "center", deserialize_with = "deserialize_f32_array2")] 
    center_pixel: Option<[f32; 2]>,
    #[serde(alias = "corners", deserialize_with = "deserialize_f32_array4x2")] 
    corners_pixel: Option<[[f32; 2]; 4]>,
}

#[derive(Deserialize, Debug)]
struct RawAprilTag {
    #[serde(alias = "object_id", alias = "tag_id")]
    tag_id: Option<String>,
    camera_id: Option<i32>,
    timestamp: Option<String>,
    yaw: Option<f32>,
    #[serde(alias = "center", deserialize_with = "deserialize_f32_array2")]
    center_pixel: Option<[f32; 2]>,
    #[serde(alias = "corners", deserialize_with = "deserialize_f32_array4x2")] 
    corners_pixel: Option<[[f32; 2]; 4]>,
}

#[derive(Deserialize, Debug)]
struct RawMessage {
    #[serde(default)]
    objects: Vec<RawObject>,
    #[serde(default, alias = "apriltags")]
    apriltags: Vec<RawAprilTag>,
}

fn parse_f32(val: &serde_json::Value) -> Option<f32> {
    match val {
        serde_json::Value::Number(n) => n.as_f64().map(|v| v as f32),
        serde_json::Value::String(s) => s.parse::<f32>().ok(),
        _ => None,
    }
}

fn deserialize_f32_array2<'de, D>(deserializer: D) -> Result<Option<[f32; 2]>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let arr: Option<Vec<serde_json::Value>> = Option::deserialize(deserializer)?;
    match arr {
        Some(arr) => {
            if arr.len() != 2 {
                return Err(serde::de::Error::custom("Expected array of length 2"));
            }
            let x = parse_f32(&arr[0]).ok_or_else(|| serde::de::Error::custom("Invalid f32"))?;
            let y = parse_f32(&arr[1]).ok_or_else(|| serde::de::Error::custom("Invalid f32"))?;
            Ok(Some([x, y]))
        }
        None => Ok(None),
    }
}

fn deserialize_f32_array4x2<'de, D>(deserializer: D) -> Result<Option<[[f32; 2]; 4]>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let arr: Option<Vec<Vec<serde_json::Value>>> = Option::deserialize(deserializer)?;
    match arr {
        Some(arr) => {
            if arr.len() != 4 {
                return Err(serde::de::Error::custom("Expected array of length 4"));
            }
            let mut out = [[0.0; 2]; 4];
            for (i, row) in arr.iter().enumerate() {
                if row.len() != 2 {
                    return Err(serde::de::Error::custom("Expected inner array of length 2"));
                }
                out[i][0] = parse_f32(&row[0]).ok_or_else(|| serde::de::Error::custom("Invalid f32"))?;
                out[i][1] = parse_f32(&row[1]).ok_or_else(|| serde::de::Error::custom("Invalid f32"))?;
            }
            Ok(Some(out))
        }
        None => Ok(None),
    }
}



pub fn parse_message(json: &str, objects: &mut [ObjectData], apriltags: &mut [AprilTagData]) -> ParseResult {
    let parsed: Result<RawMessage, _> = serde_json::from_str(json);
    if parsed.is_err() {
        error!("Failed to parse JSON");
        return ParseResult { success: false, object_count: 0, apriltag_count: 0 };
    }
    let parsed = parsed.unwrap();
    let mut obj_count = 0;
    let mut tag_count = 0;
    for raw in parsed.objects.iter() {
        if obj_count >= objects.len() { break; }
        // Validate required fields
        if let (Some(object_id), Some(camera_id), Some(timestamp), Some(class_type), Some(confidence), Some(center_pixel), Some(corners_pixel)) = (
            &raw.object_id, raw.camera_id, &raw.timestamp, &raw.class_type, raw.confidence, raw.center_pixel, raw.corners_pixel
        ) {
            if camera_id < 0 { continue; }
            objects[obj_count] = ObjectData {
                object_id: object_id.clone(),
                camera_id,
                timestamp: timestamp.clone(),
                class_type: class_type.clone(),
                confidence,
                center_pixel,
                corners_pixel,
                center_real: None,
                corners_real: None,
            };
            obj_count += 1;
        }
    }
    for raw in parsed.apriltags.iter() {
        if tag_count >= apriltags.len() { break; }
        if let (Some(tag_id), Some(camera_id), Some(timestamp), Some(yaw), Some(center_pixel), Some(corners_pixel)) = (
            &raw.tag_id, raw.camera_id, &raw.timestamp, raw.yaw, raw.center_pixel, raw.corners_pixel
        ) {
            if camera_id < 0 { continue; }
            apriltags[tag_count] = AprilTagData {
                tag_id: tag_id.clone(),
                camera_id,
                timestamp: timestamp.clone(),
                yaw,
                center_pixel,
                corners_pixel,
                center_real: None,
                corners_real: None,
            };
            tag_count += 1;
        }
    }
    ParseResult { success: true, object_count: obj_count, apriltag_count: tag_count }
}
