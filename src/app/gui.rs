#[cfg(not(target_arch = "wasm32"))]
use super::DRAWING_ALPHA;

use super::GuiMode;
use super::WilliamifyApp;
use crate::app::DEFAULT_RESOLUTION;
use crate::app::calculate;
use crate::app::calculate::ProgressMsg;
use crate::app::calculate::util::CropScale;
use crate::app::calculate::util::GenerationSettings;
use crate::app::calculate::util::SourceImg;
use crate::app::gif_recorder::GIF_FRAMERATE;
use crate::app::gif_recorder::GifStatus;
use crate::app::preset::Preset;
use crate::app::preset::UnprocessedPreset;
use eframe::App;
use eframe::Frame;
use egui::Color32;
use egui::Modal;
use egui::TextureHandle;
use egui::Window;
use image::buffer::ConvertBuffer;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use uuid::Uuid;

// #[cfg(not(target_arch = "wasm32"))]
// use std::thread as wasm_thread;

#[derive(Default)]
struct GuiImageCache {
    source_preview: Option<egui::TextureHandle>,
}

pub(crate) struct GuiState {
    #[cfg(not(target_arch = "wasm32"))]
    pub last_mouse_pos: Option<(f32, f32)>,
    #[cfg(not(target_arch = "wasm32"))]
    pub drawing_color: [f32; 4],
    mode: GuiMode,
    pub animate: bool,
    //pub fps_text: String,
    show_progress_modal: Option<Uuid>,
    last_progress: f32,
    process_cancelled: Arc<AtomicBool>,
    //pub currently_processing: Option<Preset>,
    pub presets: Vec<Preset>,
    //pub current_settings: GenerationSettings,
    configuring_generation: Option<(SourceImg, GenerationSettings, GuiImageCache)>,
    saved_config: Option<(SourceImg, GenerationSettings)>,
    pub current_preset: usize,
    error_message: Option<String>,

    has_williamified_once: bool,
}

impl GuiState {
    pub fn default(
        presets: Vec<Preset>,
        current_preset: usize,
        has_williamified_once: bool,
    ) -> GuiState {
        GuiState {
            animate: true,
            //fps_text: String::new(),
            presets,
            mode: GuiMode::Transform,
            show_progress_modal: None,
            last_progress: 0.0,
            process_cancelled: Arc::new(AtomicBool::new(false)),
            #[cfg(not(target_arch = "wasm32"))]
            last_mouse_pos: None,
            #[cfg(not(target_arch = "wasm32"))]
            drawing_color: [0.0, 0.0, 0.0, DRAWING_ALPHA],
            //currently_processing: None,
            //current_settings: GenerationSettings::default(),
            configuring_generation: None,
            saved_config: None,
            current_preset,
            error_message: None,
            has_williamified_once,
        }
    }

    fn show_progress_modal(&mut self, id: Uuid) {
        self.show_progress_modal = Some(id);
        #[cfg(target_arch = "wasm32")]
        hide_icons();
    }

    fn hide_progress_modal(&mut self) {
        self.show_progress_modal = None;
        #[cfg(target_arch = "wasm32")]
        show_icons();
    }

    fn show_error(&mut self, msg: String) {
        self.error_message = Some(msg);
    }

    fn hide_error(&mut self) {
        self.error_message = None;
    }
}

#[cfg(target_arch = "wasm32")]
fn show_icons() {
    use wasm_bindgen::JsCast;
    // show .bottom-left-icons class after processing
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        if let Some(icons) = document.query_selector(".bottom-left-icons").ok().flatten() {
            let _ = icons
                .dyn_ref::<web_sys::HtmlElement>()
                .map(|e| e.style().set_property("display", "flex"));
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn hide_icons() {
    use wasm_bindgen::JsCast;
    // hide .bottom-left-icons class while processing
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        if let Some(icons) = document.query_selector(".bottom-left-icons").ok().flatten() {
            let _ = icons
                .dyn_ref::<web_sys::HtmlElement>()
                .map(|e| e.style().set_property("display", "none"));
        }
    }
}

impl App for WilliamifyApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "presets", &self.gui.presets);
        eframe::set_value(
            storage,
            "has_williamified_once",
            &self.gui.has_williamified_once,
        );
    }
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        let Some(rs) = frame.wgpu_render_state() else {
            return;
        };

        let device = &rs.device;
        // Resize handling (match the egui "central panel" size)
        //let available = ctx.available_rect();
        // let target_size = (
        //     available.width().max(1.0) as u32,
        //     available.height().max(1.0) as u32,
        // );
        // if target_size != self.size {
        //     self.resize(rs, target_size);
        // }

        // Ensure texture is registered exactly once per allocation
        self.ensure_registered_texture(
            rs,
            if self.size.0 < 512 {
                wgpu::FilterMode::Nearest
            } else {
                wgpu::FilterMode::Linear
            },
        );

        #[cfg(target_arch = "wasm32")]
        self.ensure_worker(ctx);

        // Run GPU pipeline
        if let Some(img) = &self.preview_image {
            // show image
            let img = if img.width() != self.size.0 || img.height() != self.size.1 {
                &image::imageops::resize(
                    img,
                    self.size.0,
                    self.size.1,
                    image::imageops::FilterType::Nearest,
                )
            } else {
                img
            };
            let rgba: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> = img.convert();
            let rgba = rgba.into_raw();
            rs.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.color_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * self.size.0),
                    rows_per_image: Some(self.size.1),
                },
                wgpu::Extent3d {
                    width: self.size.0,
                    height: self.size.1,
                    depth_or_array_layers: 1,
                },
            );
        } else {
            self.run_gpu(rs);

            if self.gui.animate {
                if self.gif_recorder.is_recording() {
                    if self.gif_recorder.no_inflight() {
                        if let Err(e) = self.get_color_image_data(device, &rs.queue) {
                            self.gif_recorder.status = GifStatus::Error(e.to_string());
                        }
                    }
                    match self.gif_recorder.try_write_frame() {
                        Err(e) => {
                            self.gif_recorder.status = GifStatus::Error(e.to_string());
                            self.gui.animate = false;
                        }
                        Ok(true) => {
                            for _ in 0..(60 / GIF_FRAMERATE) {
                                self.sim.update(&mut self.seeds, self.size.0);
                            }

                            self.gif_recorder.frame_count += 1;

                            if self.gif_recorder.should_stop() {
                                // finish recording
                                if !self.gif_recorder.finish(
                                    self.gif_recorder.get_name(self.sim.name(), self.reverse),
                                ) {
                                    // cancelled
                                    self.stop_recording_gif(device, &rs.queue);
                                }

                                self.gui.animate = false;
                            } else {
                                // queue next frame
                                if let Err(e) = self.get_color_image_data(device, &rs.queue) {
                                    self.gif_recorder.status = GifStatus::Error(e.to_string());
                                }
                            }
                        }

                        Ok(false) => { /* not ready yet */ }
                    }
                } else {
                    self.sim.update(&mut self.seeds, self.size.0);
                }
                rs.queue
                    .write_buffer(&self.seed_buf, 0, bytemuck::cast_slice(&self.seeds));
                // Update seed texture for WebGL compatibility
                self.update_seed_texture_data(&rs.queue, &self.seeds);
            }
        }

        // let dt = self.prev_frame_time.elapsed();
        // self.prev_frame_time = std::time::Instant::now();
        // self.gui.fps_text = format!(
        //     "{:5.2} ms/frame (~{:06.0} FPS)",
        //     dt.as_secs_f64() * 1000.0,
        //     1.0 / dt.as_secs_f64()
        // );

        let screen_width = ctx.available_rect().width();
        let baseline_zoom = if screen_width > ctx.available_rect().height() {
            1.4_f32
        } else {
            1.0_f32
        };

        let btn_size = egui::vec2(160.0, 48.0);

        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(180.0)
            .show(ctx, |ui| {
                ui.ctx().set_zoom_factor(baseline_zoom);
                ui.add_space(16.0);
                ui.with_layout(
                    egui::Layout::top_down_justified(egui::Align::Center),
                    |ui| {
                        // Upload button (glows until first use)
                        let upload_btn = if !self.gui.has_williamified_once {
                            let time = ui.input(|i| i.time);
                            let pulse = ((time * 2.0).sin() * 0.5 + 0.5) as f32;
                            let glow = egui::Color32::from_rgb(
                                (30.0 + pulse * 100.0) as u8,
                                (120.0 + pulse * 135.0) as u8,
                                (200.0 + pulse * 55.0) as u8,
                            );
                            ui.add_sized(
                                btn_size,
                                egui::Button::new("upload your\nown image")
                                    .stroke(egui::Stroke::new(1.5, glow)),
                            )
                        } else {
                            ui.add_sized(btn_size, egui::Button::new("upload your\nown image"))
                        };

                        if upload_btn.clicked() {
                            if let Some((ref img, ref settings)) = self.gui.saved_config {
                                self.gui.configuring_generation = Some((
                                    img.clone(),
                                    settings.clone_with_new_id(),
                                    GuiImageCache::default(),
                                ));
                                #[cfg(target_arch = "wasm32")]
                                hide_icons();
                            } else {
                                prompt_image(
                                    "choose image to williamify",
                                    self,
                                    |name: String, mut img: SourceImg, app: &mut WilliamifyApp| {
                                        img = ensure_reasonable_size(img);
                                        app.gui.configuring_generation = Some((
                                            img,
                                            GenerationSettings::default(Uuid::new_v4(), name),
                                            GuiImageCache::default(),
                                        ));
                                        #[cfg(target_arch = "wasm32")]
                                        hide_icons();
                                    },
                                );
                            }
                        }

                        ui.add_space(8.0);

                        if ui
                            .add_sized(btn_size, egui::Button::new("▶  play"))
                            .clicked()
                        {
                            self.gui.animate = true;
                            self.sim.prepare_play(&mut self.seeds, self.reverse);
                        }

                        ui.add_space(8.0);

                        // Preset picker
                        ui.label("choose preset:");
                        egui::ComboBox::from_id_salt("preset_picker")
                            .width(btn_size.x)
                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                            .selected_text({
                                let name = self.sim.name();
                                if name.chars().count() > 13 {
                                    let truncated: String = name.chars().take(10).collect();
                                    format!("{truncated}…")
                                } else {
                                    name.clone()
                                }
                            })
                            .show_ui(ui, |ui| {
                                let mut to_remove: Option<usize> = None;
                                let mut close_menu = false;

                                for (i, preset) in self.gui.presets.clone().into_iter().enumerate()
                                {
                                    ui.horizontal(|ui| {
                                        let remove_enabled = self.gui.presets.len() > 4;

                                        let del_width = if remove_enabled {
                                            let txt = egui::WidgetText::from("x");
                                            let galley = txt.into_galley(
                                                ui,
                                                None,
                                                f32::INFINITY,
                                                egui::TextStyle::Button,
                                            );
                                            galley.size().x + ui.spacing().button_padding.x * 2.0
                                        } else {
                                            0.0
                                        };
                                        let spacing = if remove_enabled {
                                            ui.spacing().item_spacing.x
                                        } else {
                                            0.0
                                        };
                                        let preset_width =
                                            (ui.available_width() - del_width - spacing).max(0.0);

                                        let selected = i == self.gui.current_preset;
                                        let preset_resp = ui.add_sized(
                                            [preset_width, ui.spacing().interact_size.y],
                                            egui::Button::selectable(selected, &preset.inner.name),
                                        );

                                        if remove_enabled
                                            && ui
                                                .small_button("x")
                                                .on_hover_text("delete preset")
                                                .clicked()
                                        {
                                            to_remove = Some(i);
                                        } else if preset_resp.clicked() {
                                            self.change_sim(device, &rs.queue, preset.clone(), i);
                                            self.gui.animate = true;
                                            self.gui.current_preset = i;
                                            close_menu = true;
                                        }
                                    });
                                }

                                if let Some(idx) = to_remove {
                                    let removed_current = idx == self.gui.current_preset;
                                    self.gui.presets.remove(idx);
                                    if removed_current {
                                        let new_index = idx.min(self.gui.presets.len() - 1);
                                        self.change_sim(
                                            device,
                                            &rs.queue,
                                            self.gui.presets[new_index].clone(),
                                            new_index,
                                        );
                                        self.gui.current_preset = new_index;
                                    } else if idx < self.gui.current_preset {
                                        self.gui.current_preset -= 1;
                                    }
                                }

                                if close_menu {
                                    ui.close();
                                }
                            });

                        // export preset (native dev tool, pushed to bottom)
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                                ui.add_space(8.0);
                                if ui
                                    .add_sized(btn_size, egui::Button::new("export preset"))
                                    .on_hover_text("save to presets/<name>/ for hardcoding")
                                    .clicked()
                                {
                                    let preset = &self.gui.presets[self.gui.current_preset];
                                    match export_preset(preset) {
                                        Ok(dir) => {
                                            opener::open(&dir).ok();
                                        }
                                        Err(e) => self.gui.show_error(e.to_string()),
                                    }
                                }
                            });
                        }
                    },
                );
            });
        if self.gui.configuring_generation.is_some() {
            Window::new("williamification settings")
                .max_width(screen_width.min(400.0) * 0.8)
                //.max_height(500.0)
                .resizable(false)
                .collapsible(false)
                .movable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    //ctx.set_zoom_factor((screen_width / 400.0).max(1.0) * baseline_zoom);
                    // ui.set_width((screen_width * 0.9).min(400.0));
                    // ui.set_max_height(500.0);
                    let max_w = ui.available_width();
                    ui.allocate_ui_with_layout(
                        egui::vec2(max_w, 0.0),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.set_max_width(max_w);
                            // ui.add(egui::Label::new(
                            //     egui::RichText::new("williamification settings")
                            //         .heading()
                            //         .strong(),
                            // ));
                            // ui.separator();
                            ui.allocate_ui_with_layout(
                                egui::vec2(max_w, 0.0),
                                egui::Layout::left_to_right(egui::Align::Center)
                                    .with_main_wrap(true),
                                |ui| {
                                    ui.label("name:");
                                    if let Some((_, settings, _)) =
                                        self.gui.configuring_generation.as_mut()
                                    {
                                        ui.text_edit_singleline(&mut settings.name);
                                    }
                                },
                            );

                            ui.separator();

                            let mut change_source = false;

                            if let Some((source_img, settings, cache)) =
                                self.gui.configuring_generation.as_mut()
                            {
                                let william = {
                                    let raw = settings.get_raw_target();
                                    settings.target_crop_scale.apply(&raw, 128)
                                };
                                change_source = image_crop_gui(
                                    "source",
                                    ui,
                                    source_img,
                                    &mut settings.source_crop_scale,
                                    &mut cache.source_preview,
                                    Some(&william),
                                );
                            }

                            if change_source {
                                prompt_image(
                                    "choose image to williamify",
                                    self,
                                    |_, mut img: SourceImg, app: &mut WilliamifyApp| {
                                        img = ensure_reasonable_size(img);
                                        if let Some((src, _, cache)) =
                                            &mut app.gui.configuring_generation
                                        {
                                            *src = img;
                                            cache.source_preview = None;
                                        }
                                    },
                                );
                            }

                            ui.separator();

                            if let Some((_img, settings, _)) =
                                self.gui.configuring_generation.as_mut()
                            {
                                egui::CollapsingHeader::new("advanced settings")
                                    .default_open(false)
                                    .show(ui, |ui| {
                                        ui.allocate_ui_with_layout(
                                            egui::vec2(max_w, 0.0),
                                            egui::Layout::top_down(egui::Align::Min),
                                            |ui| {
                                                let slider_w = ui.available_width().min(260.0);
                                                ui.add_sized(
                                                    [slider_w, 20.0],
                                                    egui::Slider::new(
                                                        &mut settings.sidelen,
                                                        64..=256,
                                                    )
                                                    .text("resolution"),
                                                );

                                                let slider_w = ui.available_width().min(260.0);
                                                ui.add_sized(
                                                    [slider_w, 20.0],
                                                    egui::Slider::new(
                                                        &mut settings.proximity_importance,
                                                        0..=50,
                                                    )
                                                    .text("proximity importance"),
                                                );

                                                let mut algorithm = match settings.algorithm {
                                                    calculate::util::Algorithm::Optimal => {
                                                        "optimal algorithm"
                                                    }
                                                    calculate::util::Algorithm::Genetic => {
                                                        "fast algorithm"
                                                    }
                                                };

                                                egui::ComboBox::from_id_salt("algorithm_select")
                                                    .selected_text(algorithm)
                                                    .show_ui(ui, |ui| {
                                                        if ui.button("optimal algorithm").clicked()
                                                        {
                                                            algorithm = "optimal algorithm";
                                                            settings.algorithm =
                                                                calculate::util::Algorithm::Optimal;
                                                        }
                                                        if ui.button("fast algorithm").clicked() {
                                                            algorithm = "fast algorithm";
                                                            settings.algorithm =
                                                                calculate::util::Algorithm::Genetic;
                                                        }
                                                    });
                                            },
                                        );
                                    });
                            }
                            ui.separator();
                            ui.horizontal_wrapped(|ui| {
                                if ui
                                    .add(egui::Button::new(egui::RichText::new("start!").strong()))
                                    .clicked()
                                {
                                    if let Some((img, mut settings, _)) =
                                        self.gui.configuring_generation.take()
                                    {
                                        self.gui.show_progress_modal(settings.id);
                                        self.gui.saved_config =
                                            Some((img.clone(), settings.clone()));
                                        //self.gui.currently_processing = Some(path.clone());
                                        //self.change_sim(device, path.clone(), false);

                                        // adjust for consistency across resolutions
                                        settings.proximity_importance =
                                            (settings.proximity_importance as f32
                                                / (settings.sidelen as f32 / 128.0))
                                                as i64;

                                        self.gui
                                            .process_cancelled
                                            .store(false, std::sync::atomic::Ordering::Relaxed);

                                        let unprocessed = UnprocessedPreset {
                                            name: settings.name.clone(),
                                            width: img.width(),
                                            height: img.height(),
                                            source_img: img.into_raw(),
                                        };

                                        self.resize_textures(
                                            device,
                                            (settings.sidelen, settings.sidelen),
                                            false,
                                        );

                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            self.start_job(unprocessed, settings);
                                        }

                                        #[cfg(not(target_arch = "wasm32"))]
                                        {
                                            std::thread::spawn({
                                                let tx = self.progress_tx.clone();
                                                let cancelled = self.gui.process_cancelled.clone();
                                                move || {
                                                    let result = calculate::process(
                                                        unprocessed,
                                                        settings,
                                                        &mut tx.clone(),
                                                        cancelled,
                                                    );
                                                    if let Err(err) = result {
                                                        tx.send(ProgressMsg::Error(
                                                            err.to_string(),
                                                        ))
                                                        .ok();
                                                    }
                                                }
                                            });
                                        }
                                    }
                                }
                                if ui.button("cancel").clicked() {
                                    self.gui.configuring_generation = None;
                                    #[cfg(target_arch = "wasm32")]
                                    show_icons();
                                }
                            });
                        },
                    );
                });
        }

        if let Some(progress_id) = self.gui.show_progress_modal {
            Window::new(progress_id.to_string())
                .title_bar(false)
                .collapsible(false)
                .movable(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_BOTTOM, (0.0, 0.0))
                .show(ctx, |ui| {
                    let processing_label_message = "processing...";
                    ui.vertical(|ui| {
                        ui.set_min_width(ui.available_width().min(400.0));
                        while let Some(msg) = self.get_latest_msg() {
                            match msg {
                                ProgressMsg::Done(new_preset) => {
                                    self.preview_image = None;
                                    self.resize_textures(
                                        device,
                                        (DEFAULT_RESOLUTION, DEFAULT_RESOLUTION),
                                        false,
                                    );
                                    //self.gui.presets = get_presets();
                                    self.gui.presets.push(new_preset.clone());
                                    self.change_sim(
                                        device,
                                        &rs.queue,
                                        new_preset,
                                        self.gui.presets.len() - 1,
                                    );
                                    self.gui.animate = true;
                                    self.gui.has_williamified_once = true;
                                    self.gui.hide_progress_modal();
                                    ui.close();
                                    break;
                                }
                                ProgressMsg::Progress(p) => {
                                    self.gui.last_progress = p;
                                }
                                ProgressMsg::Error(err) => {
                                    ui.label(format!("error: {}", err));
                                    if ui.button("close").clicked() {
                                        ui.close();
                                    }
                                }
                                ProgressMsg::UpdatePreview {
                                    width,
                                    height,
                                    data,
                                } => {
                                    let image = image::ImageBuffer::from_vec(width, height, data);
                                    self.preview_image = image;
                                }
                                ProgressMsg::Cancelled => {
                                    self.preview_image = None;
                                    self.resize_textures(
                                        device,
                                        (DEFAULT_RESOLUTION, DEFAULT_RESOLUTION),
                                        false,
                                    );
                                    self.gui.hide_progress_modal();
                                    ui.close();
                                }
                                ProgressMsg::UpdateAssignments(assignments) => {
                                    self.sim.set_assignments(assignments, self.size.0)
                                }
                            }
                        }

                        if self.gui.process_cancelled.load(Ordering::Relaxed) {
                            ui.label("cancelling...");
                        } else if self.gui.last_progress == 0.0 {
                            ui.label("preparing...");
                        } else {
                            ui.label(processing_label_message);
                        }
                        ui.add(egui::ProgressBar::new(self.gui.last_progress).show_percentage());

                        ui.horizontal(|ui| {
                            if ui.button("cancel").clicked() {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    if let Some(w) = &self.worker {
                                        w.terminate();
                                    }
                                    self.worker = None;
                                    self.preview_image = None;
                                    self.resize_textures(
                                        device,
                                        (DEFAULT_RESOLUTION, DEFAULT_RESOLUTION),
                                        false,
                                    );
                                    self.gui.hide_progress_modal();
                                    ui.close();
                                }
                                self.gui.process_cancelled.store(true, Ordering::Relaxed);
                                self.gui.last_progress = 0.0;
                            }
                        })
                    });
                });
        } else if !self.gif_recorder.not_recording() {
            Modal::new(format!("recording_progress_{}", self.gif_recorder.id).into()).show(
                ctx,
                |ui| {
                    match self.gif_recorder.status.clone() {
                        GifStatus::Recording => {
                            ui.label("recording gif...");
                            if ui.button("cancel").clicked() {
                                self.stop_recording_gif(device, &rs.queue);
                                self.gui.animate = false;
                            }
                        }

                        GifStatus::Error(err) => {
                            ui.label(format!("Error: {}", err));
                            ui.horizontal(|ui| {
                                if ui.button("close").clicked() {
                                    self.stop_recording_gif(device, &rs.queue);
                                }
                            });
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        GifStatus::Complete(path) => {
                            ui.label("gif saved!");
                            ui.horizontal(|ui| {
                                if ui.button("open file").clicked() {
                                    opener::reveal(path).ok();
                                }
                                if ui.button("close").clicked() {
                                    self.stop_recording_gif(device, &rs.queue);
                                }
                            });
                        }
                        #[cfg(target_arch = "wasm32")]
                        GifStatus::Complete => {
                            // save opens dialog automatically
                            self.stop_recording_gif(device, &rs.queue);
                        }
                        GifStatus::None => unreachable!(),
                    }
                },
            );
        }
        if let Some(err) = &self.gui.error_message {
            let mut close = false;
            Window::new("error")
                .collapsible(false)
                .movable(true)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(err);
                    if ui.button("close").clicked() {
                        close = true;
                    }
                });
            if close {
                self.gui.hide_error();
            }
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ctx, |ui| {
                ui.with_layout(
                    egui::Layout::centered_and_justified(egui::Direction::TopDown),
                    |ui| {
                        if let Some(id) = self.egui_tex_id {
                            let full = ui.available_size();
                            let aspect = self.size.0 as f32 / self.size.1 as f32;
                            let desired = full.x.min(full.y) * egui::vec2(1.0, aspect);
                            ui.add(egui::Image::new((id, desired)).maintain_aspect_ratio(true));
                        } else {
                            ui.colored_label(Color32::LIGHT_RED, "Texture not ready");
                        }
                    },
                );
            });
        // continuous repaint for animation
        ctx.request_repaint();
        self.frame_count += 1;
    }
}

fn prompt_image(
    title: &'static str,
    app: &mut WilliamifyApp,
    callback: impl FnOnce(String, image::RgbImage, &mut WilliamifyApp) + 'static,
) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen_futures::spawn_local;
        let app_ptr: *mut WilliamifyApp = app;

        spawn_local(async move {
            if let Some(handle) = rfd::AsyncFileDialog::new()
                .set_title(title)
                .add_filter("image files", &["png", "jpg", "jpeg", "webp"])
                .pick_file()
                .await
            {
                let name = get_default_preset_name(handle.file_name());
                let data = handle.read().await;
                match image::load_from_memory(&data) {
                    Ok(img) => unsafe {
                        if let Some(app) = app_ptr.as_mut() {
                            callback(name, img.to_rgb8(), app);
                        }
                    },
                    Err(e) => unsafe {
                        if let Some(app) = app_ptr.as_mut() {
                            app.gui.show_error(format!("failed to load image: {}", e));
                        }
                    },
                }
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(file) = rfd::FileDialog::new()
            .set_title(title)
            .add_filter("image files", &["png", "jpg", "jpeg", "webp"])
            .pick_file()
        {
            let name =
                get_default_preset_name(file.file_name().unwrap().to_string_lossy().to_string());

            match image::open(file) {
                Ok(img) => callback(name, img.to_rgb8(), app),
                Err(e) => app.gui.show_error(format!("failed to load image: {}", e)),
            }
        }
    }
}

fn ensure_reasonable_size(img: SourceImg) -> SourceImg {
    let max_side = 512;
    let (w, h) = img.dimensions();
    if w <= max_side && h <= max_side {
        return img;
    }
    let scale = (max_side as f32 / w as f32).min(max_side as f32 / h as f32);
    let new_w = (w as f32 * scale).round() as u32;
    let new_h = (h as f32 * scale).round() as u32;

    image::imageops::resize(&img, new_w, new_h, image::imageops::FilterType::Lanczos3)
}

fn blend_images(base: &SourceImg, overlay: &SourceImg, overlay_alpha: f32) -> SourceImg {
    let (w, h) = base.dimensions();
    let a = overlay_alpha.clamp(0.0, 1.0);
    let ia = 1.0 - a;
    let mut out = SourceImg::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let s = base.get_pixel(x, y);
            let t = overlay.get_pixel(x, y);
            out.put_pixel(
                x,
                y,
                image::Rgb([
                    (s[0] as f32 * ia + t[0] as f32 * a).round() as u8,
                    (s[1] as f32 * ia + t[1] as f32 * a).round() as u8,
                    (s[2] as f32 * ia + t[2] as f32 * a).round() as u8,
                ]),
            );
        }
    }
    out
}

fn image_crop_gui(
    name: &'static str,
    ui: &mut egui::Ui,
    img: &SourceImg,
    crop_scale: &mut CropScale,
    cache: &mut Option<TextureHandle>,
    overlay: Option<&SourceImg>,
) -> bool {
    let mut open_file_dialog = false;
    ui.vertical(|ui| {
        let tex = match &cache {
            None => {
                let cropped = crop_scale.apply(img, 128);
                let preview = if let Some(ov) = overlay {
                    blend_images(&cropped, ov, 0.45)
                } else {
                    cropped
                };
                let p = ui.ctx().load_texture(
                    name,
                    egui::ColorImage::from_rgb([128, 128], preview.as_raw()),
                    egui::TextureOptions::LINEAR,
                );
                *cache = Some(p.clone());
                p
            }
            Some(t) => t.clone(),
        };
        ui.add(egui::Image::from_texture(&tex));
        if ui.button("change image").clicked() {
            open_file_dialog = true;
        }
        // crop sliders
        ui.vertical(|ui| {
            let values = *crop_scale;
            let slider_w = ui.available_width().min(260.0);

            ui.add_sized(
                [slider_w, 20.0],
                egui::Slider::new(&mut crop_scale.scale, 0.2..=5.0)
                    .show_value(false)
                    .text("zoom"),
            );
            ui.add_sized(
                [slider_w, 20.0],
                egui::Slider::new(&mut crop_scale.x, -1.0..=1.0)
                    .show_value(false)
                    .text("x-off."),
            );
            ui.add_sized(
                [slider_w, 20.0],
                egui::Slider::new(&mut crop_scale.y, -1.0..=1.0)
                    .show_value(false)
                    .text("y-off."),
            );
            ui.add_sized(
                [slider_w, 20.0],
                egui::Slider::new(&mut crop_scale.rotation, -180.0..=180.0)
                    .show_value(false)
                    .text("rotate"),
            );

            if values != *crop_scale {
                *cache = None; // force reload
            }
        });
    });

    open_file_dialog
}

#[cfg(not(target_arch = "wasm32"))]
fn export_preset(preset: &Preset) -> Result<String, Box<dyn std::error::Error>> {
    let dir = format!("presets/{}", preset.inner.name);
    std::fs::create_dir_all(&dir)?;

    let img = image::RgbImage::from_raw(
        preset.inner.width,
        preset.inner.height,
        preset.inner.source_img.clone(),
    )
    .ok_or("invalid image dimensions")?;
    img.save(format!("{dir}/source.png"))?;

    let json = format!(
        "[{}]",
        preset
            .assignments
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    std::fs::write(format!("{dir}/assignments.json"), json)?;

    Ok(dir)
}

fn get_default_preset_name(mut n: String) -> String {
    let mut name = {
        if let Some(dot) = n.rfind('.') {
            if dot > 0 {
                n.truncate(dot);
            }
        }
        if n.is_empty() {
            "untitled".to_owned()
        } else {
            n
        }
    };
    if name.chars().count() > 20 {
        name = name.chars().take(20).collect();
    }
    name
}
