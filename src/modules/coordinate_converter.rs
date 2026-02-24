/// Coordinate converter: batch homography for all cameras
/// Input: matrices (N_cams, 3, 3), points (N_cams, N_points, 2)
/// Output: (N_cams, N_points, 2) real-world coordinates

pub fn batch_homography_all_cameras(
    matrices: &[ [[f64; 3]; 3] ], // (N_cams, 3, 3)
    points: &[ Vec<[f64; 2]> ]   // (N_cams, N_points, 2)
) -> Vec<Vec<[f64; 2]>> {
    // Validate input shapes
    let n_cams = matrices.len();
    assert_eq!(points.len(), n_cams, "Number of cameras mismatch");
    for m in matrices {
        assert!(m.len() == 3 && m[0].len() == 3, "Each matrix must be 3x3");
    }
    for cam_points in points {
        for p in cam_points {
            assert_eq!(p.len(), 2, "Each point must be 2D");
        }
    }

    let mut result = Vec::with_capacity(n_cams);
    for (cam_idx, (h, cam_points)) in matrices.iter().zip(points.iter()).enumerate() {
        let mut cam_result = Vec::with_capacity(cam_points.len());
        for &pt in cam_points {
            // Convert to homogeneous
            let x = pt[0];
            let y = pt[1];
            let vec = [x, y, 1.0];
            // Matrix multiplication
            let mut proj = [0.0; 3];
            for i in 0..3 {
                proj[i] = h[i][0]*vec[0] + h[i][1]*vec[1] + h[i][2]*vec[2];
            }
            // Perspective division
            let z = if proj[2].abs() < 1e-10 { 1e-10 } else { proj[2] };
            cam_result.push([proj[0]/z, proj[1]/z]);
        }
        result.push(cam_result);
    }
    result
}
