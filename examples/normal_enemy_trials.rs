//! Native, headless pose study of the real Frame normal/shadow renderer.
//!
//! Each of the 18 looks is rendered independently, then its native crop is
//! copied into a 384px-wide study board. This is authored idle-pose playback,
//! not simulated gameplay: 1 tick = 1 frame = 1/60 second, no interpolation.
//! Fixed lighting is the default so a changing normal cannot hide in an orbit.
//!
//! cargo run --release --example normal_enemy_trials -- OUT [240] [fixed|orbit]
use glintblade::gfx::frame::Surface;
use glintblade::gfx::{self, Anim, DrawOpts, Frame, Grade, H, Rgb, Sheet, W, rgb};
use serde_json::json;
use std::path::{Path, PathBuf};

const TILE_W: usize = 128;
const TILE_H: usize = 56;
const COLS: usize = 3;
const ROWS: usize = 6;
const OUT_W: usize = TILE_W * COLS;
const OUT_H: usize = TILE_H * ROWS;
const CROP_X: usize = 128;
const CROP_Y: usize = 84;
const ACTOR_X: i32 = 192;
const GROUND_Y: i32 = 124;

struct Case {
    sheet: &'static Sheet,
    anim: &'static Anim,
    species: &'static str,
    label: &'static str,
    variant: usize,
    row: usize,
    radius: f32,
    depth: i32,
    full_width: bool,
    elevation: f32,
}

fn cases() -> Vec<Case> {
    let mut result = Vec::with_capacity(18);
    for (row, (species, label, preferred, radius, depth, full_width, elevation)) in [
        ("sproutling", "SPROUT", "idle_right", 4.0, 3, false, 0.0),
        ("thorn_imp", "IMP", "idle_right", 4.5, 3, false, 0.0),
        ("crystal_beetle", "BEETLE", "idle_right", 6.5, 8, true, 0.0),
        ("wisp", "WISP", "idle", 4.5, 2, false, 7.0),
        // The down idle contains the near-static eye-pulse pair whose raw
        // PixelLab normals formerly changed the whole skull's illumination.
        ("ember_hound", "HOUND", "idle_down", 6.0, 4, false, 0.0),
        ("spitting_totem", "TOTEM", "idle_down", 6.0, 5, false, 0.0),
    ]
    .into_iter()
    .enumerate()
    {
        for variant in 0..3 {
            let name = if variant == 0 {
                species.to_owned()
            } else {
                format!("{species}_v{variant}")
            };
            let sheet = gfx::sheet(&name).unwrap_or_else(|| panic!("missing study atlas {name}"));
            let anim = [preferred, "idle_right", "idle", "idle_down"]
                .into_iter()
                .find_map(|name| sheet.anim(name))
                .unwrap_or_else(|| panic!("missing authored idle for {name}"));
            assert!(
                anim.frames > 1,
                "study must show the actual pose clock: {name}"
            );
            assert_eq!(anim.durations.len(), anim.frames);
            result.push(Case {
                sheet,
                anim,
                species,
                label,
                variant,
                row,
                radius,
                depth,
                full_width,
                elevation,
            });
        }
    }
    result
}

fn draw_ground(f: &mut Frame) {
    // A matte flat receiving plane isolates pose/light changes. No generated
    // art, painted lighting, luminance-derived height or synthetic sprite.
    for y in 0..H {
        for x in 0..W {
            let i = y * W + x;
            f.albedo[i] = rgb(0x59615b);
            f.surfaces[i] = Surface {
                normal: [0.0, 0.0, 1.0],
                height: 0.0,
                owner: 0,
                kind: 1,
            };
        }
    }
}

fn copy_native_crop(frame: &Frame, board: &mut [u8], column: usize, row: usize) {
    for y in 0..TILE_H {
        for x in 0..TILE_W {
            let src = (CROP_Y + y) * W + CROP_X + x;
            let dst = ((row * TILE_H + y) * OUT_W + column * TILE_W + x) * 4;
            board[dst..dst + 4].copy_from_slice(&frame.out[src]);
        }
    }
}

fn write_json(path: &Path, value: &serde_json::Value) {
    let mut encoded = serde_json::to_vec_pretty(value).expect("serialize study manifest");
    encoded.push(b'\n');
    std::fs::write(path, encoded).expect("write study manifest");
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(
        args.next()
            .unwrap_or_else(|| "out/pixellab/round7/enemy-pose-study".to_owned()),
    );
    let ticks: u32 = args
        .next()
        .map(|s| s.parse().expect("frame count must be an integer"))
        .unwrap_or(240);
    assert!(ticks > 0 && ticks <= 3600, "expected 1..3600 native ticks");
    let mode = args.next().unwrap_or_else(|| "fixed".to_owned());
    assert!(
        matches!(mode.as_str(), "fixed" | "orbit"),
        "mode is fixed or orbit"
    );
    assert!(args.next().is_none(), "unexpected extra argument");
    let frames = out.join("frames");
    std::fs::create_dir_all(&frames).expect("create study output");
    assert!(CROP_X + TILE_W <= W && CROP_Y + TILE_H <= H);
    let cases = cases();
    let grade = Grade {
        vignette: 0.0,
        bloom: 0.12,
        light_steps: 32.0,
        ..Default::default()
    };
    let ambient = Rgb::new(0.42, 0.46, 0.55);
    let fixed_light = (144.0, 156.0, 44.0);
    let mut f = Frame::new();
    let mut timeline = Vec::with_capacity(ticks as usize);
    let mut board = vec![0u8; OUT_W * OUT_H * 4];
    for tick in 0..ticks {
        let mut pose_indices = Vec::with_capacity(cases.len());
        for case in &cases {
            f.begin(rgb(0x161f2d), ambient);
            f.surface_lighting = true;
            f.t = tick as u64;
            draw_ground(&mut f);
            let pose = case.anim.frame_at(tick, true);
            let cell = case.sheet.cell(case.anim, pose);
            assert_eq!(cell.surface.len(), (cell.w * cell.h) as usize);
            assert!(
                !cell.is_empty(),
                "empty study pose: {} {pose}",
                case.sheet.name
            );
            let opts = DrawOpts {
                actor: true,
                ground_y: Some(GROUND_Y as f32),
                elevation: case.elevation,
                foot_half_w: case.radius,
                foot_depth: case.depth,
                foot_full_width: case.full_width,
                ..Default::default()
            };
            f.sprite(
                cell,
                case.sheet.anchor_of(case.anim),
                ACTOR_X,
                GROUND_Y - case.elevation as i32,
                &opts,
            );
            let light = if mode == "orbit" {
                let angle = tick as f32 / ticks as f32 * std::f32::consts::TAU;
                (
                    ACTOR_X as f32 + angle.cos() * 48.0,
                    GROUND_Y as f32 + angle.sin() * 32.0,
                    44.0,
                )
            } else {
                fixed_light
            };
            f.add_light_3d_w(light.0, light.1, light.2, 110.0, rgb(0xffe5c3), 0.82);
            gfx::surface::apply(&mut f);
            gfx::post::compose(&mut f, &grade);
            // Real native UI font labels the diagnostic without rescaling art.
            gfx::font::text(
                &mut f,
                CROP_X as i32 + 3,
                CROP_Y as i32 + 2,
                &format!("{} V{} F{}", case.label, case.variant, pose),
                rgb(0xc3c8c5),
                1,
                0.0,
            );
            gfx::post::finish(&mut f, &grade);
            copy_native_crop(&f, &mut board, case.variant, case.row);
            pose_indices.push(pose);
        }
        let filename = format!("f{tick:05}.png");
        glintblade::assets::write_png(&frames.join(&filename), OUT_W as u32, OUT_H as u32, &board)
            .expect("write actual renderer pose board");
        timeline.push(json!({"tick":tick,"frame":filename,"pose_indices":pose_indices}));
        if tick % 60 == 0 {
            eprintln!(
                "normal enemy study: {tick}/{ticks} ticks, {} authored looks",
                cases.len()
            );
        }
    }
    let entries: Vec<_> = cases
        .iter()
        .map(|c| {
            json!({
                "sheet":c.sheet.name,"species":c.species,"variant":c.variant,
                "animation":c.anim.name,"source_row":c.anim.row,"frames":c.anim.frames,
                "durations_60hz":c.anim.durations,"cycle_ticks":c.anim.total_ticks(),
                "anchor":c.sheet.anchor_of(c.anim),"cell":[c.sheet.cell_w,c.sheet.cell_h],
                "crop_on_board":[c.variant*TILE_W,c.row*TILE_H,TILE_W,TILE_H],
                "body_radius":c.radius,"foot_depth":c.depth,"foot_full_width":c.full_width,
                "elevation":c.elevation,
            })
        })
        .collect();
    write_json(
        &out.join("manifest.json"),
        &json!({
            "kind":"actual Frame normal/shadow renderer authored-pose study",
            "renderer_binary":std::env::current_exe().ok(),
            "fps":60,"simulation_hz":60,"frame_count":ticks,"retimed":false,
            "native_pixels":[OUT_W,OUT_H],"integer_scale":1,"source_frame_pixels":[W,H],
            "frame_pattern":"frames/f%05d.png","first_tick":0,"last_tick":ticks-1,
            "light_mode":mode,"fixed_light_xyz":fixed_light,"light_radius":110.0,
            "light_intensity":0.82,"light_color":"#ffe5c3","ambient_rgb":[ambient.r,ambient.g,ambient.b],
            "position_stays_fixed":true,"wisp_hover_stays_fixed":true,
            "normal_data":"embedded current atlas PNGs; no synthetic relighting or luminance-derived height",
            "source_pose_clock":"Anim.frame_at(tick,true), original durations, one tick per output frame",
            "layout":"Rows: sprout, imp, beetle, wisp, hound, totem. Columns: base, v1, v2.",
            "important_pairs":["thorn_imp idle_right changes","ember_hound idle_down eye pulse"],
            "limits":"Pose study, not AI/combat gameplay. No independent procedural bob, particles, self light or behavior state simulation.",
            "cases":entries,
        }),
    );
    write_json(&out.join("timeline.json"), &json!(timeline));
    println!(
        "{} actual native frames written to {}",
        ticks,
        out.display()
    );
}
