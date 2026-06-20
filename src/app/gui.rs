#[cfg(not(target_arch = "wasm32"))]
use super::DRAWING_ALPHA;

use super::GuiMode;
use super::WilliamifyApp;

// ── Design system ─────────────────────────────────────────────────────────────
const C_BG: Color32 = Color32::from_rgb(0, 0, 0);
const C_SURFACE: Color32 = Color32::from_rgb(10, 10, 10);
fn c_border() -> Color32 { Color32::from_rgba_unmultiplied(255, 255, 255, 20) }
fn c_border_hover() -> Color32 { Color32::from_rgba_unmultiplied(255, 255, 255, 38) }
const C_TEXT: Color32 = Color32::from_rgb(237, 237, 237);
const C_TEXT_MUTED: Color32 = Color32::from_rgb(102, 102, 102);
const C_TEXT_DIM: Color32 = Color32::from_rgb(51, 51, 51);
const C_GREEN: Color32 = Color32::from_rgb(74, 222, 128);
const C_RED: Color32 = Color32::from_rgb(248, 113, 113);

fn nav_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(C_BG)
        .inner_margin(egui::Margin { left: 16, right: 16, top: 0, bottom: 0 })
        .stroke(egui::Stroke::new(1.0, c_border()))
}

fn ctrl_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(C_SURFACE)
        .inner_margin(egui::Margin { left: 16, right: 16, top: 0, bottom: 0 })
        .stroke(egui::Stroke::new(1.0, c_border()))
}

fn tab_btn(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
    let color = if active { C_TEXT } else { C_TEXT_MUTED };
    let resp = ui.add(
        egui::Button::new(egui::RichText::new(label).size(13.0).color(color)).frame(false),
    );
    if active {
        let r = resp.rect;
        ui.painter()
            .hline(r.x_range(), r.bottom() + 0.5, egui::Stroke::new(1.0, Color32::WHITE));
    }
    resp
}

fn ctrl_sep(ui: &mut egui::Ui) {
    ui.add_space(4.0);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(1.0, 16.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, c_border_hover());
    ui.add_space(4.0);
}

fn ctrl_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .size(11.0)
            .color(C_TEXT_MUTED),
    );
}

fn play_btn(ui: &mut egui::Ui) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new("▶  play").size(12.0).color(C_GREEN).strong())
            .fill(Color32::from_rgba_unmultiplied(74, 222, 128, 20))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(74, 222, 128, 89))),
    )
}

fn pause_btn(ui: &mut egui::Ui) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new("⏸  pause").size(12.0).color(C_RED).strong())
            .fill(Color32::from_rgba_unmultiplied(248, 113, 113, 20))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(248, 113, 113, 89))),
    )
}
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
    william_preview: Option<egui::TextureHandle>,
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
    show_how_it_works: bool,
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
            show_how_it_works: false,
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
                                if !self.gif_recorder.finish(
                                    self.gif_recorder.get_name(self.sim.name(), self.reverse),
                                ) {
                                    self.stop_recording_gif(device, &rs.queue);
                                }
                                self.gui.animate = false;
                            } else {
                                if let Err(e) = self.get_color_image_data(device, &rs.queue) {
                                    self.gif_recorder.status = GifStatus::Error(e.to_string());
                                }
                            }
                        }
                        Ok(false) => {}
                    }
                } else {
                    self.sim.update(&mut self.seeds, self.size.0);
                }
                rs.queue
                    .write_buffer(&self.seed_buf, 0, bytemuck::cast_slice(&self.seeds));
                self.update_seed_texture_data(&rs.queue, &self.seeds);
            }
        }

        let screen_width = ctx.available_rect().width();
        let baseline_zoom = if screen_width > ctx.available_rect().height() {
            1.4_f32
        } else {
            1.0_f32
        };
        ctx.set_zoom_factor(baseline_zoom);

        // ── NAV BAR ───────────────────────────────────────────────────────────
        egui::TopBottomPanel::top("navbar")
            .frame(nav_frame())
            .exact_height(48.0)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new("williamifier")
                            .size(13.0)
                            .strong()
                            .color(C_TEXT),
                    );
                    ui.add_space(16.0);
                    if tab_btn(ui, "animation", !self.gui.show_how_it_works).clicked() {
                        self.gui.show_how_it_works = false;
                    }
                    if tab_btn(ui, "how it works", self.gui.show_how_it_works).clicked() {
                        self.gui.show_how_it_works = true;
                    }
                });
            });

        // ── HOW IT WORKS PAGE ─────────────────────────────────────────────────
        if self.gui.show_how_it_works {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(C_BG))
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            how_it_works_content(ui);
                        });
                });
            ctx.request_repaint();
            self.frame_count += 1;
            return;
        }

        // ── CONTROL BAR ───────────────────────────────────────────────────────
        egui::TopBottomPanel::top("controlbar")
            .frame(ctrl_frame())
            .exact_height(44.0)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    // Upload button (pulses white until first use)
                    let upload_resp = if !self.gui.has_williamified_once {
                        let time = ui.input(|i| i.time);
                        let pulse = ((time * 2.0).sin() * 0.5 + 0.5) as f32;
                        let a = (20.0 + pulse * 18.0) as u8;
                        ui.add(
                            egui::Button::new(
                                egui::RichText::new("upload image")
                                    .size(12.0)
                                    .color(C_TEXT),
                            )
                            .stroke(egui::Stroke::new(
                                1.0,
                                Color32::from_rgba_unmultiplied(255, 255, 255, a),
                            )),
                        )
                    } else {
                        ui.add(
                            egui::Button::new(
                                egui::RichText::new("upload image").size(12.0).color(C_TEXT_MUTED),
                            ),
                        )
                    };

                    if upload_resp.clicked() {
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

                    ctrl_sep(ui);

                    // Play / Pause
                    if self.gui.animate {
                        if pause_btn(ui).clicked() {
                            self.gui.animate = false;
                        }
                    } else {
                        if play_btn(ui).clicked() {
                            self.gui.animate = true;
                            self.sim.prepare_play(&mut self.seeds, self.reverse);
                        }
                    }

                    ctrl_sep(ui);

                    // Preset picker
                    ctrl_label(ui, "preset");
                    egui::ComboBox::from_id_salt("preset_picker")
                        .width(120.0)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .selected_text(
                            egui::RichText::new({
                                let name = self.sim.name();
                                if name.chars().count() > 13 {
                                    format!("{}…", name.chars().take(10).collect::<String>())
                                } else {
                                    name
                                }
                            })
                            .size(12.0)
                            .color(C_TEXT),
                        )
                        .show_ui(ui, |ui| {
                            let mut to_remove: Option<usize> = None;
                            let mut close_menu = false;
                            for (i, preset) in self.gui.presets.clone().into_iter().enumerate() {
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
                                        && ui.small_button("x").on_hover_text("delete").clicked()
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

                    // Export preset — native only, pushed to the right
                    #[cfg(not(target_arch = "wasm32"))]
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(
                                egui::RichText::new("export preset").size(12.0).color(C_TEXT_DIM),
                            ))
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
                });
            });

        // ── MODAL WINDOWS ─────────────────────────────────────────────────────
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
                                    &mut cache.william_preview,
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
                                            cache.william_preview = None;
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
        if self.gui.show_how_it_works {
            egui::TopBottomPanel::top("hiw_topbar")
                .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(16, 10)))
                .show(ctx, |ui| {
                    ui.ctx().set_zoom_factor(baseline_zoom);
                    ui.horizontal(|ui| {
                        if ui.button("← back").clicked() {
                            self.gui.show_how_it_works = false;
                        }
                    });
                });
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(egui::Color32::from_rgb(248, 248, 246)))
                .show(ctx, |ui| {
                    ui.ctx().set_zoom_factor(baseline_zoom);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            how_it_works_content(ui);
                        });
                });
            ctx.request_repaint();
            self.frame_count += 1;
            return;
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
    source_cache: &mut Option<TextureHandle>,
    william_cache: &mut Option<TextureHandle>,
    overlay: Option<&SourceImg>,
) -> bool {
    let mut open_file_dialog = false;
    ui.vertical(|ui| {
        // Build both preview textures when cache is stale
        let source_tex = match &source_cache {
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
                *source_cache = Some(p.clone());
                p
            }
            Some(t) => t.clone(),
        };
        let william_tex = overlay.map(|ov| match &william_cache {
            None => {
                let cropped = crop_scale.apply(img, 128);
                let preview = blend_images(ov, &cropped, 0.45);
                let p = ui.ctx().load_texture(
                    &format!("{name}_william"),
                    egui::ColorImage::from_rgb([128, 128], preview.as_raw()),
                    egui::TextureOptions::LINEAR,
                );
                *william_cache = Some(p.clone());
                p
            }
            Some(t) => t.clone(),
        });

        // Show previews side by side
        ui.horizontal(|ui| {
            ui.add(egui::Image::from_texture(&source_tex));
            if let Some(wt) = william_tex {
                ui.add(egui::Image::from_texture(&wt));
            }
        });
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
                *source_cache = None;
                *william_cache = None;
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


// ── "How it works" helpers ────────────────────────────────────────────────────

/// Draws one collapsible section: border-top, clickable header (title left, › right),
/// and body content when open. Matches the HTML <details class="tech-section"> structure.
fn tech_section(
    ui: &mut egui::Ui,
    title: &str,
    default_open: bool,
    is_last: bool,
    content: impl FnOnce(&mut egui::Ui),
) {
    let id = egui::Id::new(("hiw_section", title));
    let open = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(default_open));

    // border-top: 1px solid var(--border)
    let top_y = ui.cursor().top();
    let x_range = ui.clip_rect().x_range();
    ui.painter().hline(x_range, top_y, egui::Stroke::new(1.0, c_border()));

    // Clickable header row — padding: 16px 0
    let full_w = ui.available_width();
    let (header_rect, header_resp) = ui.allocate_exact_size(
        egui::vec2(full_w, 52.0),   // 16 + ~20 text + 16
        egui::Sense::click(),
    );

    if ui.is_rect_visible(header_rect) {
        let painter = ui.painter();
        let text_color = if header_resp.hovered() { Color32::WHITE } else { C_TEXT };

        // Section label — 12px, 600, uppercase, letter-spacing approximated
        painter.text(
            egui::pos2(header_rect.left(), header_rect.center().y),
            egui::Align2::LEFT_CENTER,
            title.to_uppercase(),
            egui::FontId::new(12.0, egui::FontFamily::Proportional),
            text_color,
        );

        let chevron_str = "›";
        let glyph_color = C_TEXT_MUTED;
        painter.text(
            egui::pos2(header_rect.right(), header_rect.center().y),
            egui::Align2::RIGHT_CENTER,
            chevron_str,
            egui::FontId::new(18.0, egui::FontFamily::Proportional),
            if open { C_TEXT } else { glyph_color },
        );
    }

    if header_resp.clicked() {
        ui.data_mut(|d| d.insert_temp(id, !open));
    }

    if open {
        content(ui);
        ui.add_space(32.0); // padding-bottom: 32px
    }

    // Last section gets border-bottom too
    if is_last {
        let bot_y = ui.cursor().top();
        let x_range = ui.clip_rect().x_range();
        ui.painter().hline(x_range, bot_y, egui::Stroke::new(1.0, c_border()));
        ui.add_space(1.0); // ensure the line is visible
    }
}

/// Body paragraph: 14px, line-height 1.8, --text color, max-width 600px.
fn tech_p(ui: &mut egui::Ui, text: &str) {
    ui.add_space(0.0);
    ui.set_max_width(600.0);
    ui.label(
        egui::RichText::new(text)
            .size(14.0)
            .color(C_TEXT)
            .line_height(Some(14.0 * 1.8)),
    );
    ui.add_space(16.0);
}

/// Monospace block: matches .tech-table (SF Mono / Fira Code style, surface bg, border, 8px radius).
fn tech_table(ui: &mut egui::Ui, content: &str) {
    ui.add_space(14.0);
    let full_w = ui.available_width().min(860.0);
    ui.allocate_ui_with_layout(
        egui::vec2(full_w, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            egui::Frame::new()
                .fill(C_SURFACE)
                .corner_radius(egui::CornerRadius::same(8))
                .stroke(egui::Stroke::new(1.0, c_border()))
                .inner_margin(egui::Margin { left: 18, right: 18, top: 14, bottom: 14 })
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(content)
                            .monospace()
                            .size(11.5)
                            .color(C_TEXT)
                            .line_height(Some(11.5 * 1.7)),
                    );
                });
        },
    );
    ui.add_space(14.0);
}

fn how_it_works_content(ui: &mut egui::Ui) {
    // .tech-content: max-width 860px, margin: 0 auto, padding: 48px 40px 80px
    let outer_w = ui.available_width();
    let content_w = outer_w.min(860.0);
    let h_pad = ((outer_w - content_w) / 2.0).max(40.0);

    ui.add_space(48.0);

    ui.allocate_ui_with_layout(
        egui::vec2(outer_w, 0.0),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            ui.set_max_width(content_w - h_pad * 2.0);

            // h1
            ui.label(
                egui::RichText::new("How it works")
                    .size(22.0)
                    .strong()
                    .color(C_TEXT),
            );
            ui.add_space(10.0);

            // .tech-intro: 14px, 1.75, --text-muted, max-width 600px, margin-bottom 36px
            ui.set_max_width(600.0);
            ui.label(
                egui::RichText::new(
                    "A walkthrough of the math and algorithms behind the Williamifier. \
                     Each pixel in your image is matched to a position in William's face, \
                     then physically simulated flying to its destination, \
                     and rendered each frame as a Voronoi mosaic on the GPU.",
                )
                .size(14.0)
                .color(C_TEXT_MUTED)
                .line_height(Some(14.0 * 1.75)),
            );
            ui.add_space(36.0);

            // Restore full content width for sections
            ui.set_max_width(content_w - h_pad * 2.0);

            // ── Section 1 ────────────────────────────────────────────────────
            tech_section(ui, "Phase 1 — Pixel Assignment", true, false, |ui| {
                tech_p(ui,
                    "Before the animation can run, every pixel in your image must be matched \
                     to exactly one position in William's face, so a one-to-one bijection. \
                     Both images are resampled to an N × N grid (default N = 128). \
                     We then search for the assignment that minimises a total cost.");

                tech_p(ui,
                    "For source pixel s at position (xs, ys) with colour (rs, gs, bs) \
                     assigned to target pixel t at (xt, yt) with colour (rt, gt, bt), the cost is:");

                tech_table(ui,
"cost(s, t)  =  Dc²(s,t) · w(t)  +  ( Dp²(s,t) · λ )²

Dc²(s,t)  =  (rs − rt)² + (gs − gt)² + (bs − bt)²   ← colour distance²
Dp²(s,t)  =  (xs − xt)² + (ys − yt)²                ← spatial distance²

w(t)  — importance weight for target pixel t  (from William's face)
λ     — proximity slider  (0 = colour only,  large = stay nearby)");

                tech_p(ui,
                    "The weight map w(t) is higher near William's prominent features (eyes, \
                     nose) so those regions are matched more precisely. Increasing λ penalises \
                     long-distance moves, keeping matched pixels spatially close.");

                tech_p(ui, "Two algorithms are available:");

                tech_p(ui,
                    "Fast (genetic): Start with the identity assignment and repeatedly \
                     pick a random pair (a, b) within search radius r. Swap them if doing \
                     so lowers the total cost:");

                tech_table(ui,
"swap(a, b)  if  cost(pa, tb) + cost(pb, ta)  <  cost(pa, ta) + cost(pb, tb)

r  ←  max( r · 0.99,  2 )   after each generation");

                tech_p(ui,
                    "Optimal (Hungarian / Kuhn-Munkres): Builds the full N² × N² cost matrix \
                     and finds the globally optimal assignment. Maintains a labelling (lx, ly) satisfying:");

                tech_table(ui,
"lx(i) + ly(j)  ≥  cost(i, j)   for all (i, j)

Augments a matching along zero-slack edges until a perfect
matching is found — that matching is provably optimal.");
            });

            // ── Section 2 ────────────────────────────────────────────────────
            tech_section(ui, "Phase 2 — Physics Simulation", true, false, |ui| {
                tech_p(ui,
                    "Each pixel is a particle with position p, velocity v, and acceleration a. \
                     Four forces are summed each frame, then velocity and position are updated:");

                tech_table(ui,
"v  ←  ( v + a ) · 0.97       ← integrate & damp
p  ←  p + clamp(v, −6, 6)    ← move (max 6 px / frame)
age  ←  age + 1");

                tech_p(ui, "Force 1 — Destination pull. Each particle is attracted to its target \
                     position p_dst. The force ramps up cubically so motion starts slow and accelerates:");

                tech_table(ui,
"elapsed  =  age / 60                      ← time in seconds at 60 fps
factor   =  min( (elapsed · k)³,  1000 )
dist     =  ‖p_dst − p‖

a  +=  (p_dst − p) · dist · factor / L

k = 0.13  (preset animations),   L = canvas side length");

                tech_p(ui, "The dist factor makes distant particles accelerate faster, \
                    producing a snapping effect as they approach the target.");

                tech_p(ui, "Force 2 — Neighbour repulsion. Particles repel each other within \
                    personal space σ = 0.95 · pixel_size. For each neighbour j within σ of i:");

                tech_table(ui,
"d   =  ‖pj − pi‖
w   =  (σ − d) / (σ · d)         ← weight → ∞ as d → 0

ai  −=  (pj − pi) · w");

                tech_p(ui, "Force 3 — Wall repulsion. Particles are pushed away from the canvas boundary:");

                tech_table(ui,
"σ_wall = σ / 2

if  px < σ_wall :       ax  +=  (σ_wall − px) / σ_wall
if  px > L − σ_wall :   ax  −=  (px − (L − σ_wall)) / σ_wall
(and symmetrically for py / ay)");

                tech_p(ui, "Force 4 — Velocity alignment. Particles nudge their velocity toward \
                    the weighted average of their neighbours, producing flock-like coherent motion:");

                tech_table(ui,
"v̄  =  ( Σ vj · wj ) / ( Σ wj )      ← over all neighbours j

a  +=  ( v̄ − v ) · 0.8");
            });

            // ── Section 3 ────────────────────────────────────────────────────
            tech_section(ui, "Phase 3 — Voronoi Rendering (Jump Flood Algorithm)", true, true, |ui| {
                tech_p(ui,
                    "Every frame the GPU colours each pixel with the colour of its nearest \
                     particle. A naïve search costs O(N² · S) per frame. The Jump Flood \
                     Algorithm (JFA) approximates the Voronoi diagram in O(N² log N) using \
                     only ⌈log₂ N⌉ GPU render passes.");

                tech_p(ui, "Step 1 — Clear: every pixel initialised to sentinel ID = 0xFFFFFFFF.");
                tech_p(ui, "Step 2 — Seed splat: seed i at position (xi, yi) writes its index to the nearest integer pixel.");
                tech_p(ui, "Step 3 — JFA passes: k starts at 2^⌊log₂(max_dim)⌋ and halves each pass down to k = 1:");

                tech_table(ui,
"for each pass with step k:
  for each pixel p = (px, py):
    for each δ ∈ { (±k, 0), (0, ±k), (±k, ±k) }:
      q  =  p + δ
      if q has seed j at (xj, yj):
        d²  =  (px − xj)² + (py − yj)²
        if d² < d²_best:  best ← j,  d²_best ← d²
    write best to pixel p");

                tech_p(ui, "Step 4 — Shade: each pixel reads the colour stored for its \
                    assigned seed index, producing the final mosaic frame.");

                tech_p(ui, "For a 1024 × 1024 canvas this is 10 passes. For a 2048 × 2048 canvas, 11 passes.");
            });

            ui.add_space(80.0); // padding-bottom
        },
    );
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
