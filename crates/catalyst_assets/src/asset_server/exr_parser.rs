use exr::prelude::read_first_rgba_layer_from_file;
use glam::Vec3;

type ExtPayload = (Vec<f32>, u32, u32);

struct HdrBuffer {
    width: usize,
    data: Vec<f32>,
}

pub fn parse_exr(
    path: &str,
    face_size: u32, /* e.g., 1024 or 2048 */
) -> Result<ExtPayload, exr::error::Error> {
    let image = read_first_rgba_layer_from_file(
        path,
        |resolution, _| {
            let width = resolution.width();
            let height = resolution.height();
            HdrBuffer {
                width, // Store width so we can use it later
                data: vec![0.0; width * height * 4],
            }
        },
        |buffer, position, (r, g, b, a): (f32, f32, f32, f32)| {
            let idx = (position.y() * buffer.width + position.x()) * 4;
            buffer.data[idx] = r;
            buffer.data[idx + 1] = g;
            buffer.data[idx + 2] = b;
            buffer.data[idx + 3] = a;
        },
    )?;

    // Extract the data from the returned image structure
    // The type structure is: Image { layer_data: Layer { channel_data: Pixels { pixels: T } } }
    let raw_buffer = image.layer_data.channel_data.pixels;

    let src_width = raw_buffer.width;
    let src_height = raw_buffer.data.len() / 4 / raw_buffer.width;
    let src_pixels = &raw_buffer.data;

    // 2. Prepare 6 Faces
    // Order: +X, -X, +Y, -Y, +Z, -Z
    let mut cubemap_data = Vec::with_capacity((face_size * face_size * 4 * 6) as usize);

    // Define the basis vectors for the 6 faces
    // (Right, Left, Top, Bottom, Front, Back)
    // These match the standard Cubemap orientation
    let targets = [
        (Vec3::Z, Vec3::Y, Vec3::X),         // +X (Right) -> Forward Z, Up Y
        (Vec3::NEG_Z, Vec3::Y, Vec3::NEG_X), // -X (Left)
        (Vec3::X, Vec3::NEG_Z, Vec3::Y),     // +Y (Top)
        (Vec3::X, Vec3::Z, Vec3::NEG_Y),     // -Y (Bottom)
        (Vec3::NEG_X, Vec3::Y, Vec3::Z),     // +Z (Front)
        (Vec3::X, Vec3::Y, Vec3::NEG_Z),     // -Z (Back)
    ];

    // 3. Process each Face
    for (forward, up, face_dir) in targets {
        let right = up.cross(forward); // Calculate tangent

        for y in 0..face_size {
            for x in 0..face_size {
                // Normalize x,y to -1.0 .. +1.0
                let u_local = (x as f32 / face_size as f32) * 2.0 - 1.0;
                let v_local = (y as f32 / face_size as f32) * 2.0 - 1.0;
                // Flip Y because texture coordinates go down, but 3D goes up
                let v_local = -v_local;

                // Compute the direction vector for this pixel
                // Center of face + offset * right + offset * up
                let dir = (face_dir + right * u_local + up * v_local).normalize();

                // Sample the EXR
                let (u_eq, v_eq) = direction_to_uv(dir.x, dir.y, dir.z);
                let color = sample_equirectangular(src_pixels, src_width, src_height, u_eq, v_eq);

                cubemap_data.extend_from_slice(&color);
            }
        }
    }

    Ok((cubemap_data, face_size, face_size))
}

// Helper to sample pixels with bilinear interpolation
fn sample_equirectangular(pixels: &[f32], width: usize, height: usize, u: f32, v: f32) -> [f32; 4] {
    let u = u.fract(); // Wrap UVs
    let v = v.clamp(0.0, 1.0); // Clamp V to avoid poles issues

    let x = u * (width as f32 - 1.0);
    let y = v * (height as f32 - 1.0);

    let x_l = x.floor() as usize;
    let x_r = (x_l + 1).min(width - 1);
    let y_t = y.floor() as usize;
    let y_b = (y_t + 1).min(height - 1);

    let w_x = x - x.floor();
    let w_y = y - y.floor();

    // Sample 4 neighbors
    let idx = |px, py| (py * width + px) * 4;
    let p00 = &pixels[idx(x_l, y_t)..];
    let p10 = &pixels[idx(x_r, y_t)..];
    let p01 = &pixels[idx(x_l, y_b)..];
    let p11 = &pixels[idx(x_r, y_b)..];

    // Bilinear blend
    let mut result = [0.0; 4];
    for i in 0..4 {
        let top = p00[i] * (1.0 - w_x) + p10[i] * w_x;
        let bot = p01[i] * (1.0 - w_x) + p11[i] * w_x;
        result[i] = top * (1.0 - w_y) + bot * w_y;
    }
    result
}

// Convert 3D direction to Equirectangular UV
// (This is the inverse of the shader code we looked at earlier)
fn direction_to_uv(x: f32, y: f32, z: f32) -> (f32, f32) {
    let u = (z.atan2(x) / (std::f32::consts::PI * 2.0)) + 0.5;
    let v = (y.asin() / std::f32::consts::PI) + 0.5;
    (u, v)
}
