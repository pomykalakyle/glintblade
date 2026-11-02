//! Material microgeometry in world coordinates, independent of color and illumination.
//! Terrain painters compose this once with their retained native/masonry normal.
//! It changes light incidence only; collision, receiver altitude and casts stay geometric.

use crate::util::hash2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Material {
    Grass,
    Earth,
    Ash,
    Stone,
    Rock,
    WetStone,
    Water,
}

/// Compose a rooted material's small-scale slopes with its existing macro plane.
/// Samples use world pixels, so camera movement and native atlas boundaries cannot
/// make the detail swim or restart. Water's animated renderer owns its ripples;
/// near-vertical faces and stair risers retain their exact architectural normals.
pub(crate) fn normal(material: Material, x: i32, y: i32, base: [f32; 3]) -> [f32; 3] {
    if material == Material::Water || base[2] <= 0.25 {
        return base;
    }
    let length = base.iter().map(|v| v * v).sum::<f32>().sqrt();
    let n = if length > 0.001 { base.map(|v| v / length) } else { [0.0, 0.0, 1.0] };
    let [dx, dy] = slope(material, x as f32 + 0.5, y as f32 + 0.5);
    // Preserve the silhouette-scale plane. Only its tangent frame receives the
    // material relief; strongly tilted native bevels get less added texture.
    let fade = ((n[2] - 0.25) * 2.0).clamp(0.0, 1.0);
    let tx_len = n[0].hypot(n[2]);
    let tx = [n[2] / tx_len, 0.0, -n[0] / tx_len];
    let ty = [-n[1] * n[0] / tx_len, tx_len, -n[1] * n[2] / tx_len];
    let result = std::array::from_fn::<_, 3, _>(|i| n[i] - fade * (dx * tx[i] + dy * ty[i]));
    let length = result.iter().map(|v| v * v).sum::<f32>().sqrt();
    result.map(|v| v / length)
}

/// Analytic gradient of smooth value noise. Adjacent pixels share the same
/// surfaces, rather than receiving independent random normal directions.
fn field(x: f32, y: f32, axes: [f32; 4], seed: u32) -> (f32, [f32; 2]) {
    let [ax, ay, bx, by] = axes;
    let (u, v) = (x * ax + y * ay + 0.321, x * bx + y * by + 0.736);
    let (ix, iy) = (u.floor() as i32, v.floor() as i32);
    let (fx, fy) = (u - ix as f32, v - iy as f32);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let (dsx, dsy) = (6.0 * fx * (1.0 - fx), 6.0 * fy * (1.0 - fy));
    let (a, b, c, d) = (hash2(ix, iy, seed), hash2(ix + 1, iy, seed),
        hash2(ix, iy + 1, seed), hash2(ix + 1, iy + 1, seed));
    let du = ((b - a) * (1.0 - sy) + (d - c) * sy) * dsx;
    let dv = ((c - a) * (1.0 - sx) + (d - b) * sx) * dsy;
    (a + (b - a) * sx + (c - a) * sy + (a - b - c + d) * sx * sy,
        [du * ax + dv * bx, du * ay + dv * by])
}

fn slope(material: Material, x: f32, y: f32) -> [f32; 2] {
    if material == Material::Grass { return grass_slope(x, y); }
    let mut result = [0.0; 2];
    let mut add = |axes: [f32; 4], seed: u32, height: f32, ridged: bool| {
        let (value, gradient) = field(x, y, axes, seed);
        // A narrow rounded blade has two differently facing sides. Crossing
        // ridges form irregular rooted clumps, with no square clump boundaries.
        let gain = height * if ridged { if value < 0.5 { 2.0 } else { -2.0 } } else { 1.0 };
        for i in 0..2 { result[i] += gradient[i] * gain; }
    };
    match material {
        Material::Grass => unreachable!(),
        Material::Earth => {
            add([1.0 / 4.2, 0.025, -0.035, 1.0 / 3.4], 9201, 1.25, false);
            add([1.0 / 14.0, -0.012, 0.02, 1.0 / 11.0], 9202, 1.00, false);
        }
        Material::Ash => {
            add([1.0 / 5.2, 0.02, -0.015, 1.0 / 4.3], 9301, 1.55, false);
            add([1.0 / 17.0, 0.0, 0.0, 1.0 / 13.0], 9302, 1.10, false);
        }
        Material::Stone => {
            add([1.0 / 10.0, 0.013, -0.012, 1.0 / 8.0], 9401, 2.00, false);
            add([1.0 / 3.7, -0.02, 0.02, 1.0 / 5.3], 9402, 0.44, false);
        }
        Material::Rock => {
            add([1.0 / 9.0, 0.023, -0.018, 1.0 / 6.5], 9501, 2.10, false);
            add([1.0 / 4.7, -0.035, 0.016, 1.0 / 8.0], 9502, 0.50, true);
        }
        Material::WetStone => {
            add([1.0 / 11.0, 0.01, -0.014, 1.0 / 8.5], 9601, 2.15, false);
            add([1.0 / 4.5, 0.016, 0.01, 1.0 / 6.0], 9602, 0.32, false);
        }
        Material::Water => {}
    }
    result
}

/// Short blades grow in overlapping, irregularly oriented fans. Their compact
/// support prevents a local blade direction from turning into lawn-wide grooves.
/// Smooth low turf mounds connect clumps without a visible placement grid.
fn grass_slope(x: f32, y: f32) -> [f32; 2] {
    let (_, mound) = field(x, y, [1.0/12.0, 0.012, -0.008, 1.0/10.0], 9103);
    let (_, turf) = field(x, y, [1.0/4.7, -0.034, 0.031, 1.0/4.1], 9104);
    let mut result = std::array::from_fn(|i| mound[i] * 1.5 + turf[i] * 1.05);
    let (cx, cy) = ((x / 6.0).floor() as i32, (y / 6.0).floor() as i32);
    for gy in cy-1..=cy+1 { for gx in cx-1..=cx+1 {
        let root = [gx as f32 * 6.0 + 1.0 + hash2(gx,gy,9110)*4.0,
            gy as f32 * 6.0 + 1.0 + hash2(gx,gy,9111)*4.0];
        let angle = hash2(gx, gy, 9112) * std::f32::consts::TAU;
        let (s, c) = angle.sin_cos();
        for (blade, (rc, rs)) in [(0.36236, -0.93204), (0.92106, -0.38942),
            (0.92106, 0.38942), (0.36236, 0.93204)].into_iter().enumerate() {
            let axis = [c * rc - s * rs, s * rc + c * rs];
            let half = 1.8 + hash2(gx, gy, 9120 + blade as u32) * 0.9;
            let width = 0.85 + hash2(gx, gy, 9130 + blade as u32) * 0.30;
            let height = 0.27 + hash2(gx, gy, 9140 + blade as u32) * 0.10;
            let dx = x - root[0] - axis[0] * half * 0.75;
            let dy = y - root[1] - axis[1] * half * 0.75;
            let along = dx * axis[0] + dy * axis[1];
            let across = -dx * axis[1] + dy * axis[0];
            let radius = (along / half).powi(2) + (across / width).powi(2);
            if radius >= 1.0 { continue; }
            let gain = -4.0 * height * (1.0 - radius);
            let du = gain * along / (half * half);
            let dv = gain * across / (width * width);
            result[0] += du * axis[0] - dv * axis[1];
            result[1] += du * axis[1] + dv * axis[0];
        }
    }}
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_materials_have_spatially_coherent_relief_and_directional_light_response() {
        for material in [Material::Grass, Material::Earth, Material::Ash, Material::Stone, Material::Rock, Material::WetStone] {
            let mut slopes = Vec::new();
            let mut response = Vec::new();
            let (mut neighbor, mut separated) = (0.0, 0.0);
            for y in 480..608 { for x in 256..448 {
                let n = normal(material, x, y, [0.0, 0.0, 1.0]);
                assert!(n.iter().all(|v| v.is_finite()));
                assert!((n.iter().map(|v| v * v).sum::<f32>() - 1.0).abs() < 0.00001);
                slopes.push(n[0].hypot(n[1]));
                response.push(n[0] * -0.56 + n[1] * -0.32 + n[2] * 0.76);
                let near = normal(material, x + 1, y, [0.0, 0.0, 1.0]);
                let far = normal(material, x + 29, y + 17, [0.0, 0.0, 1.0]);
                neighbor += n.iter().zip(near).map(|(a, b)| (a-b).powi(2)).sum::<f32>();
                separated += n.iter().zip(far).map(|(a, b)| (a-b).powi(2)).sum::<f32>();
            }}
            slopes.sort_by(f32::total_cmp);
            response.sort_by(f32::total_cmp);
            let size = slopes.len();
            let covered = slopes.iter().filter(|&&s| s > 0.05).count();
            let contrast = response[size * 95 / 100] - response[size * 5 / 100];
            println!("MATERIAL_RELIEF {material:?}: median {:.4}, >.05 {:.2}%, dot response {:.4}, adjacent/distant {:.3}",
                slopes[size / 2], covered as f32 * 100.0 / size as f32, contrast, neighbor/separated);
            assert!(covered * 100 > size * 80 && slopes[size / 2] > 0.10, "{material:?} still shades flat");
            assert!(contrast > 0.12, "{material:?} produces no visible incidence contrast");
            assert!(neighbor < separated * 0.75, "{material:?} became per-pixel random glitter");
        }
    }

    #[test]
    fn water_and_architectural_faces_keep_their_macro_normals() {
        for base in [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.8, 0.56, 0.2]] {
            assert_eq!(normal(Material::Stone, 130, 87, base), base);
        }
        let up = [0.0, 0.0, 1.0];
        assert_eq!(normal(Material::Water, 130, 87, up), up);
        let native = [0.3, -0.2, 0.9327379];
        let detail = normal(Material::Stone, 130, 87, native);
        assert!(detail.iter().zip(native).map(|(a,b)| a*b).sum::<f32>() > 0.9);
        assert_eq!(normal(Material::Grass, -130, -87, up), normal(Material::Grass, -130, -87, up));
    }

    #[test]
    fn grass_clumps_have_no_lawn_wide_groove_direction() {
        let (mut xx, mut yy, mut xy) = (0.0, 0.0, 0.0);
        for y in 480..608 { for x in 256..448 {
            let n = normal(Material::Grass, x, y, [0.0, 0.0, 1.0]);
            xx += n[0] * n[0];
            yy += n[1] * n[1];
            xy += n[0] * n[1];
        }}
        let anisotropy = xx / yy;
        let diagonal = xy.abs() / (xx * yy).sqrt();
        println!("GRASS_CLUMP_DIRECTIONS: X/Y energy {anisotropy:.4}, diagonal correlation {diagonal:.4}");
        assert!((0.70..1.40).contains(&anisotropy), "turf became parallel grooves");
        assert!(diagonal < 0.15, "turf acquired a dominant diagonal groove");
    }
}
