#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

#[cfg(not(target_arch = "wasm32"))]
fn regen_presets() {


    let preset_names = ["wisetree", "blackhole", "cat", "cat2", "colorful"];
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let presets_dir = manifest_dir.join("presets");

    // Load target + weights (same files the app embeds at compile time)
    let target_bytes = include_bytes!("app/calculate/target256.png");
    let weights_bytes = include_bytes!("app/calculate/weights256.png");
    let target_img = image::load_from_memory(target_bytes).expect("target").to_rgb8();
    let weights_img = image::load_from_memory(weights_bytes).expect("weights").to_rgb8();

    // Replicate GenerationSettings defaults (sidelen=128, proximity_importance=13, genetic)
    let sidelen: u32 = 128;
    let proximity_importance: i64 = 13;

    for name in &preset_names {
        let source_path = presets_dir.join(name).join("source.png");
        let out_path = presets_dir.join(name).join("assignments.json");

        print!("Processing {name}... ");
        std::io::Write::flush(&mut std::io::stdout()).ok();
        let t0 = std::time::Instant::now();

        let source_img = image::open(&source_path)
            .unwrap_or_else(|e| panic!("failed to open {source_path:?}: {e}"))
            .to_rgb8();

        // Resize source to sidelen×sidelen
        let source_resized = image::imageops::resize(
            &source_img,
            sidelen,
            sidelen,
            image::imageops::FilterType::Lanczos3,
        );


        let source_pixels: Vec<(u8, u8, u8)> = source_resized
            .pixels()
            .map(|p| (p[0], p[1], p[2]))
            .collect();

        // Resize target + weights to sidelen
        let target_resized = image::imageops::resize(
            &target_img,
            sidelen,
            sidelen,
            image::imageops::FilterType::Lanczos3,
        );
        let target_pixels: Vec<(u8, u8, u8)> = target_resized
            .pixels()
            .map(|p| (p[0], p[1], p[2]))
            .collect();

        let weights_resized = image::imageops::resize(
            &weights_img,
            sidelen,
            sidelen,
            image::imageops::FilterType::Lanczos3,
        );
        let weights: Vec<i64> = weights_resized
            .pixels()
            .map(|p| p[0] as i64)
            .collect();

        let n = (sidelen * sidelen) as usize;
        assert_eq!(source_pixels.len(), n);
        assert_eq!(target_pixels.len(), n);

        // === Genetic algorithm (exact copy of process_genetic in calculate/mod.rs) ===
        #[inline(always)]
        fn heuristic(
            apos: (u16, u16), bpos: (u16, u16),
            a: (u8, u8, u8), b: (u8, u8, u8),
            color_weight: i64, spatial_weight: i64,
        ) -> i64 {
            let spatial = (apos.0 as i64 - bpos.0 as i64).pow(2)
                + (apos.1 as i64 - bpos.1 as i64).pow(2);
            let color = (a.0 as i64 - b.0 as i64).pow(2)
                + (a.1 as i64 - b.1 as i64).pow(2)
                + (a.2 as i64 - b.2 as i64).pow(2);
            color * color_weight + (spatial * spatial_weight).pow(2)
        }

        #[derive(Clone, Copy)]
        struct Pixel {
            src_x: u16, src_y: u16,
            rgb: (u8, u8, u8),
            h: i64,
        }

        let mut pixels: Vec<Pixel> = source_pixels
            .iter()
            .enumerate()
            .map(|(i, &(r, g, b))| {
                let x = (i as u32 % sidelen) as u16;
                let y = (i as u32 / sidelen) as u16;
                let h = heuristic(
                    (x, y), (x, y),
                    (r, g, b), target_pixels[i],
                    weights[i], proximity_importance,
                );
                Pixel { src_x: x, src_y: y, rgb: (r, g, b), h }
            })
            .collect();

        let mut rng = frand::Rand::with_seed(12345);
        let swaps_per_gen = 128 * pixels.len();
        let mut max_dist = sidelen;

        loop {
            let mut swaps_made = 0;
            for _ in 0..swaps_per_gen {
                let apos = rng.gen_range(0..pixels.len() as u32) as usize;
                let ax = apos as u16 % sidelen as u16;
                let ay = apos as u16 / sidelen as u16;
                let bx = (ax as i16
                    + rng.gen_range(-(max_dist as i16)..(max_dist as i16 + 1)))
                    .clamp(0, sidelen as i16 - 1) as u16;
                let by = (ay as i16
                    + rng.gen_range(-(max_dist as i16)..(max_dist as i16 + 1)))
                    .clamp(0, sidelen as i16 - 1) as u16;
                let bpos = by as usize * sidelen as usize + bx as usize;

                let t_a = target_pixels[apos];
                let t_b = target_pixels[bpos];

                let a_on_b = heuristic(
                    (pixels[apos].src_x, pixels[apos].src_y), (bx, by),
                    pixels[apos].rgb, t_b, weights[bpos], proximity_importance,
                );
                let b_on_a = heuristic(
                    (pixels[bpos].src_x, pixels[bpos].src_y), (ax, ay),
                    pixels[bpos].rgb, t_a, weights[apos], proximity_importance,
                );

                if pixels[apos].h - b_on_a + pixels[bpos].h - a_on_b > 0 {
                    pixels.swap(apos, bpos);
                    pixels[apos].h = b_on_a;
                    pixels[bpos].h = a_on_b;
                    swaps_made += 1;
                }
            }

            if max_dist < 4 && swaps_made < 10 {
                break;
            }
            max_dist = (max_dist as f32 * 0.99).max(2.0) as u32;
        }

        // Build assignments: assignments[dst_idx] = src_idx
        let assignments: Vec<usize> = pixels
            .iter()
            .map(|p| p.src_y as usize * sidelen as usize + p.src_x as usize)
            .collect();

        let json = format!(
            "[{}]",
            assignments
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        std::fs::write(&out_path, json).unwrap_or_else(|e| panic!("write {out_path:?}: {e}"));
        println!("done in {:.1?}", t0.elapsed());
    }

    println!("All presets regenerated. Recompile to embed them.");
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    if std::env::args().any(|a| a == "--regen-presets") {
        regen_presets();
        return Ok(());
    }

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 1024.0])
            .with_min_inner_size([400.0, 400.0])
            .with_icon(
                // NOTE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon128.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "williamify",
        native_options,
        Box::new(|cc| Ok(Box::new(williamify::WilliamifyApp::new(cc)))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn start_app() {
    use eframe::wasm_bindgen::JsCast as _;
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    //web_sys::console::log_1(&"Starting williamify...".into());

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Warn).ok();

    let web_options = eframe::WebOptions {
        wgpu_options: egui_wgpu::WgpuConfiguration {
            // Force WebGL backend for maximum compatibility
            wgpu_setup: egui_wgpu::WgpuSetup::CreateNew(egui_wgpu::WgpuSetupCreateNew {
                instance_descriptor: egui_wgpu::wgpu::InstanceDescriptor {
                    backends: egui_wgpu::wgpu::Backends::GL,
                    ..Default::default()
                },
                power_preference: egui_wgpu::wgpu::PowerPreference::HighPerformance,
                device_descriptor: std::sync::Arc::new(|_adapter| {
                    let mut limits = egui_wgpu::wgpu::Limits::downlevel_webgl2_defaults();
                    // Clamp texture size to 2048 for WebGL compatibility
                    limits.max_texture_dimension_2d = 4096;
                    egui_wgpu::wgpu::DeviceDescriptor {
                        label: Some("egui_device"),
                        required_features: egui_wgpu::wgpu::Features::default(),
                        required_limits: limits,
                        memory_hints: egui_wgpu::wgpu::MemoryHints::default(),
                        trace: Default::default(),
                    }
                }),
                native_adapter_selector: None,
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(williamify::WilliamifyApp::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    use web_sys::js_sys::JsString;

                    loading_text.set_inner_html(&format!(
                        "<div> Please enable hardware acceleration in your browser :) </div> <div class=\"error\"> Error: {} </div>",
                        std::convert::Into::<JsString>::into(e.clone())
                    ));
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}

#[cfg(target_arch = "wasm32")]
pub fn main() {
    use wasm_bindgen::JsCast as _;
    console_error_panic_hook::set_once();

    // If we have a Window, we’re on the page → run the app.
    if web_sys::window().is_some() {
        start_app();
        return;
    }

    // Otherwise, if we have a DedicatedWorkerGlobalScope, we’re in a worker → install worker.
    if web_sys::js_sys::global()
        .dyn_ref::<web_sys::DedicatedWorkerGlobalScope>()
        .is_some()
    {
        williamify::worker_entry(); // <- your existing function that sets onmessage, etc.
        return;
    }

    // Fallback: unknown environment
    web_sys::console::warn_1(
        &"Unknown global (not Window / not DedicatedWorkerGlobalScope)".into(),
    );
}
