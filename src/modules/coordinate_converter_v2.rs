use std::collections::HashMap;

pub struct CameraCalibration {
    pub fx: f32,
    pub fy: f32,
    pub cx: f32,
    pub cy: f32,
    pub fx_p: f32,
    pub fy_p: f32,
    pub cx_p: f32,
    pub cy_p: f32,
    pub k1: f32,
    pub k2: f32,
    pub p1: f32,
    pub p2: f32,
    pub k3: f32,
    pub has_new_matrix: bool,
}

pub struct UndistortionManager {
    pub calibrations: HashMap<i32, CameraCalibration>,
}

impl UndistortionManager {
    pub fn new() -> Self {
        Self { calibrations: HashMap::new() }
    }
    pub fn set_calibration(&mut self, camera_id: i32, calib: CameraCalibration) {
        self.calibrations.insert(camera_id, calib);
    }
    pub fn has_camera(&self, camera_id: i32) -> bool {
        self.calibrations.contains_key(&camera_id)
    }
    pub fn undistort_points(&self, camera_id: i32, points_pixel: &[[f32; 2]]) -> Vec<[f32; 2]> {
        if !self.has_camera(camera_id) { return points_pixel.to_vec(); }
        let calib = &self.calibrations[&camera_id];
        let mut out = Vec::with_capacity(points_pixel.len());
        for &pt in points_pixel {
            let mut x = (pt[0] - calib.cx) / calib.fx;
            let mut y = (pt[1] - calib.cy) / calib.fy;
            for _ in 0..5 {
                let r2 = x*x + y*y;
                let r4 = r2*r2;
                let r6 = r2*r4;
                let k_radial = 1.0 + calib.k1*r2 + calib.k2*r4 + calib.k3*r6;
                let delta_x = 2.0*calib.p1*x*y + calib.p2*(r2 + 2.0*x*x);
                let delta_y = calib.p1*(r2 + 2.0*y*y) + 2.0*calib.p2*x*y;
                if k_radial.abs() > 1e-10 {
                    x = (x - delta_x) / k_radial;
                    y = (y - delta_y) / k_radial;
                }
            }
            let x_proj = x * calib.fx_p + calib.cx_p;
            let y_proj = y * calib.fy_p + calib.cy_p;
            out.push([x_proj, y_proj]);
        }
        out
    }
}

pub struct HomographyManager {
    pub homographies: HashMap<i32, [[f64; 3]; 3]>,
}

impl HomographyManager {
    pub fn new() -> Self {
        Self { homographies: HashMap::new() }
    }
    pub fn set_homography(&mut self, camera_id: i32, matrix: [[f64; 3]; 3]) {
        self.homographies.insert(camera_id, matrix);
    }
    pub fn has_camera(&self, camera_id: i32) -> bool {
        self.homographies.contains_key(&camera_id)
    }
    pub fn get_homography(&self, camera_id: i32) -> Option<&[[f64; 3]; 3]> {
        self.homographies.get(&camera_id)
    }
}

/// Converts pixel coordinates to real-world coordinates using undistortion and homography.
pub struct CoordinateConverter {
    pub undistortion_manager: UndistortionManager,
    pub homography_manager: HomographyManager,
    points_pixel_buffer: Vec<[f32; 2]>,
    points_real_buffer: Vec<[f64; 2]>,
    point_to_object_buffer: Vec<usize>,
    point_kind_buffer: Vec<u8>,
}

impl CoordinateConverter {
    pub fn new() -> Self {
        Self {
            undistortion_manager: UndistortionManager::new(),
            homography_manager: HomographyManager::new(),
            points_pixel_buffer: Vec::new(),
            points_real_buffer: Vec::new(),
            point_to_object_buffer: Vec::new(),
            point_kind_buffer: Vec::new(),
        }
    }

    /// Convert a batch of pixel points to real-world coordinates for a given camera.
    pub fn convert_points(&mut self, camera_id: i32, points_pixel: &[[f32; 2]]) -> &[ [f64; 2] ] {
        let undistorted = self.undistortion_manager.undistort_points(camera_id, points_pixel);
        self.points_real_buffer.clear();
        if let Some(h) = self.homography_manager.get_homography(camera_id) {
            for &pt in undistorted.iter() {
                let x = pt[0] as f64;
                let y = pt[1] as f64;
                let vec = [x, y, 1.0];
                let mut proj = [0.0; 3];
                for i in 0..3 {
                    proj[i] = h[i][0] * vec[0] + h[i][1] * vec[1] + h[i][2] * vec[2];
                }
                let z = if proj[2].abs() < 1e-10 { 1e-10 } else { proj[2] };
                self.points_real_buffer.push([proj[0] / z, proj[1] / z]);
            }
        } else {
            for &pt in undistorted.iter() {
                self.points_real_buffer.push([pt[0] as f64, pt[1] as f64]);
            }
        }
        &self.points_real_buffer[..]
    }

    fn ensure_buffer_capacity(&mut self, max_points: usize) {
        if self.points_pixel_buffer.len() < max_points {
            self.points_pixel_buffer.resize(max_points, [0.0; 2]);
        }
        if self.points_real_buffer.len() < max_points {
            self.points_real_buffer.resize(max_points, [0.0; 2]);
        }
        if self.point_to_object_buffer.len() < max_points {
            self.point_to_object_buffer.resize(max_points, 0);
        }
        if self.point_kind_buffer.len() < max_points {
            self.point_kind_buffer.resize(max_points, 0);
        }
    }


    /// Convert all objects' pixel coordinates to real-world coordinates, grouped by camera.
    pub fn convert_batch_objects(&mut self, objects: &mut [crate::ObjectData]) {
        use std::collections::HashMap;
        let mut camera_indices: HashMap<i32, Vec<usize>> = HashMap::new();
        for (i, obj) in objects.iter().enumerate() {
            camera_indices.entry(obj.camera_id).or_default().push(i);
        }
        for (cam_id, indices) in camera_indices {
            let num_objects = indices.len();
            let num_points = num_objects * 5;
            self.ensure_buffer_capacity(num_points);
            // Copy pixel points, object indices, and types out before calling convert_points
            let mut points_pixel_copy: Vec<[f32; 2]> = Vec::with_capacity(num_points);
            let mut point_to_object_copy: Vec<usize> = Vec::with_capacity(num_points);
            let mut point_type_copy: Vec<u8> = Vec::with_capacity(num_points);
            for &idx in &indices {
                let obj = &objects[idx];
                points_pixel_copy.push(obj.center_pixel);
                point_to_object_copy.push(idx);
                point_type_copy.push(0);
                for (c, &corner) in obj.corners_pixel.iter().enumerate() {
                    points_pixel_copy.push(corner);
                    point_to_object_copy.push(idx);
                    point_type_copy.push((c + 1) as u8);
                }
            }
            let points_real = self.convert_points(cam_id, &points_pixel_copy);
            for (i, &obj_idx) in point_to_object_copy.iter().enumerate() {
                let t = point_type_copy[i];
                let real = points_real[i];
                let obj = &mut objects[obj_idx];
                if t == 0 {
                    obj.center_real = Some([real[0] as f32, real[1] as f32]);
                } else if t >= 1 && t <= 4 {
                    if let Some(ref mut corners) = obj.corners_real {
                        corners[(t - 1) as usize] = [real[0] as f32, real[1] as f32];
                    } else {
                        let mut arr = [[0.0; 2]; 4];
                        arr[(t - 1) as usize] = [real[0] as f32, real[1] as f32];
                        obj.corners_real = Some(arr);
                    }
                }
            }
        }
    }

    /// Convert all AprilTag pixel coordinates to real-world coordinates, grouped by camera.
    pub fn convert_batch_apriltags(&mut self, tags: &mut [crate::AprilTagData]) {
        use std::collections::HashMap;
        let mut camera_indices: HashMap<i32, Vec<usize>> = HashMap::new();
        for (i, tag) in tags.iter().enumerate() {
            camera_indices.entry(tag.camera_id).or_default().push(i);
        }
        for (cam_id, indices) in camera_indices {
            let num_tags = indices.len();
            let num_points = num_tags * 5;
            self.ensure_buffer_capacity(num_points);
            // Copy pixel points, tag indices, and types out before calling convert_points
            let mut points_pixel_copy: Vec<[f32; 2]> = Vec::with_capacity(num_points);
            let mut point_to_tag_copy: Vec<usize> = Vec::with_capacity(num_points);
            let mut point_type_copy: Vec<u8> = Vec::with_capacity(num_points);
            for &idx in &indices {
                let tag = &tags[idx];
                points_pixel_copy.push(tag.center_pixel);
                point_to_tag_copy.push(idx);
                point_type_copy.push(0);
                for (c, &corner) in tag.corners_pixel.iter().enumerate() {
                    points_pixel_copy.push(corner);
                    point_to_tag_copy.push(idx);
                    point_type_copy.push((c + 1) as u8);
                }
            }
            let points_real = self.convert_points(cam_id, &points_pixel_copy);
            for (i, &tag_idx) in point_to_tag_copy.iter().enumerate() {
                let t = point_type_copy[i];
                let real = points_real[i];
                let tag = &mut tags[tag_idx];
                if t == 0 {
                    tag.center_real = Some([real[0] as f32, real[1] as f32]);
                } else if t >= 1 && t <= 4 {
                    if let Some(ref mut corners) = tag.corners_real {
                        corners[(t - 1) as usize] = [real[0] as f32, real[1] as f32];
                    } else {
                        let mut arr = [[0.0; 2]; 4];
                        arr[(t - 1) as usize] = [real[0] as f32, real[1] as f32];
                        tag.corners_real = Some(arr);
                    }
                }
            }
        }
    }
}
