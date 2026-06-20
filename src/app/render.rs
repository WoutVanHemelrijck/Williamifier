//! GPU rendering pipeline: texture / bind-group setup and the per-frame
//! jump-flood (JFA) + shade pass. Methods on `WilliamifyApp` split out of
//! `app.rs` to keep the core app/state file readable.

use super::*;

impl WilliamifyApp {
    pub(super) fn make_ids_texture(
        device: &wgpu::Device,
        size: (u32, u32),
        label: Option<&str>,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor {
            label: Some("ids_view"),
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(1),
            ..Default::default()
        });
        (tex, view)
    }

    pub(super) fn make_color_texture(
        device: &wgpu::Device,
        size: (u32, u32),
        label: Option<&str>,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        (tex, view)
    }

    pub(super) fn make_seed_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        seeds: &[SeedPos],
        max_seeds: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        // Pack seeds into a 2D texture to respect WebGL texture size limits (typically 2048-4096)
        // Use a square-ish layout: width = 1024, height = ceil(max_seeds / 1024)
        const TEX_WIDTH: u32 = 1024;
        let tex_height = max_seeds.div_ceil(TEX_WIDTH);

        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("seed_positions"),
            size: wgpu::Extent3d {
                width: TEX_WIDTH,
                height: tex_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg32Float, // Store x,y as 2 floats
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload seed data to texture (packed in 2D)
        let mut data = vec![0.0f32; (TEX_WIDTH * tex_height * 2) as usize];
        for (i, seed) in seeds.iter().enumerate() {
            data[i * 2] = seed.xy[0];
            data[i * 2 + 1] = seed.xy[1];
        }

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(TEX_WIDTH * 8), // 2 floats * 4 bytes per pixel
                rows_per_image: Some(tex_height),
            },
            wgpu::Extent3d {
                width: TEX_WIDTH,
                height: tex_height,
                depth_or_array_layers: 1,
            },
        );

        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        (tex, view)
    }

    pub(super) fn update_seed_texture_data(&self, queue: &wgpu::Queue, seeds: &[SeedPos]) {
        // Update seed texture data without recreating the texture
        const TEX_WIDTH: u32 = 1024;
        let tex_height = self.seed_count.div_ceil(TEX_WIDTH);

        let mut data = vec![0.0f32; (TEX_WIDTH * tex_height * 2) as usize];
        for (i, seed) in seeds.iter().enumerate() {
            data[i * 2] = seed.xy[0];
            data[i * 2 + 1] = seed.xy[1];
        }

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.seed_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(TEX_WIDTH * 8), // 2 floats * 4 bytes per pixel
                rows_per_image: Some(tex_height),
            },
            wgpu::Extent3d {
                width: TEX_WIDTH,
                height: tex_height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub(super) fn make_color_lookup_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        colors: &[SeedColor],
        max_seeds: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        // Pack colors into a 2D texture to respect WebGL texture size limits
        const TEX_WIDTH: u32 = 1024;
        let tex_height = max_seeds.div_ceil(TEX_WIDTH);

        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("color_lookup"),
            size: wgpu::Extent3d {
                width: TEX_WIDTH,
                height: tex_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float, // Store RGBA as 4 floats
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload color data to texture (packed in 2D)
        let mut data = vec![0.0f32; (TEX_WIDTH * tex_height * 4) as usize];
        for (i, color) in colors.iter().enumerate() {
            data[i * 4] = color.rgba[0];
            data[i * 4 + 1] = color.rgba[1];
            data[i * 4 + 2] = color.rgba[2];
            data[i * 4 + 3] = color.rgba[3];
        }

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(TEX_WIDTH * 16), // 4 floats * 4 bytes per pixel
                rows_per_image: Some(tex_height),
            },
            wgpu::Extent3d {
                width: TEX_WIDTH,
                height: tex_height,
                depth_or_array_layers: 1,
            },
        );

        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        (tex, view)
    }

    pub(super) fn ensure_registered_texture(
        &mut self,
        rs: &egui_wgpu::RenderState,
        filter_mode: wgpu::FilterMode,
    ) {
        if self.egui_tex_id.is_none() || self.current_filter_mode != filter_mode {
            let id = rs.renderer.write().register_native_texture(
                &rs.device,
                &self.color_view,
                filter_mode,
            );
            self.egui_tex_id = Some(id);
            self.current_filter_mode = filter_mode;
        }
    }

    pub(super) fn rebuild_bind_groups(&mut self, device: &wgpu::Device) {
        // Rebuild any BGs that reference texture views
        self.clear_bg_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg_clear_a"),
            layout: &self.clear_bgl,
            entries: &[],
        });
        self.clear_bg_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg_clear_b"),
            layout: &self.clear_bgl,
            entries: &[],
        });
        self.seed_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg_seed_splat"),
            layout: &self.seed_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.seed_tex_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.params_common_buf.as_entire_binding(),
                },
            ],
        });
        self.jfa_bg_a_to_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg_jfa_a_to_b"),
            layout: &self.jfa_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.seed_tex_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.ids_a_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.nearest_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.params_jfa_buf.as_entire_binding(),
                },
            ],
        });
        self.jfa_bg_b_to_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg_jfa_b_to_a"),
            layout: &self.jfa_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.seed_tex_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.ids_b_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.nearest_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.params_jfa_buf.as_entire_binding(),
                },
            ],
        });
        self.shade_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg_shade"),
            layout: &self.shade_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.ids_a_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.nearest_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.seed_tex_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.color_lookup_tex_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.params_common_buf.as_entire_binding(),
                },
            ],
        });
    }

    pub(super) fn resize_textures(&mut self, device: &wgpu::Device, new_size: (u32, u32), rebuild_bg: bool) {
        self.size = new_size;
        // Recreate textures
        let (ids_a, ids_a_view) = Self::make_ids_texture(device, self.size, Some("ids_a"));
        let (ids_b, ids_b_view) = Self::make_ids_texture(device, self.size, Some("ids_b"));
        let (color_tex, color_view) = Self::make_color_texture(device, self.size, Some("color"));
        self.ids_a = ids_a;
        self.ids_a_view = ids_a_view;
        self.ids_b = ids_b;
        self.ids_b_view = ids_b_view;
        self.color_tex = color_tex;
        self.color_view = color_view;

        // Update params_common
        let params_common = ParamsCommon {
            width: self.size.0,
            height: self.size.1,
            n_seeds: self.seed_count,
            _pad: 0,
        };
        self.params_common_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("params_common"),
            contents: bytemuck::bytes_of(&params_common),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let params_jfa = ParamsJfa {
            width: self.size.0,
            height: self.size.1,
            step: 1,
            _pad: 0,
        };

        self.params_jfa_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("params_jfa"),
            contents: bytemuck::bytes_of(&params_jfa),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        if rebuild_bg {
            self.rebuild_bind_groups(device);
        }

        // Force re-registering the egui texture
        self.egui_tex_id = None;
    }

    pub(super) fn run_gpu(&mut self, rs: &egui_wgpu::RenderState) {
        let device = &rs.device;

        // Prepare commands
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("voronoi_jfa_encoder"),
        });

        // 1) Clear ID texture A (where we'll splat seeds)
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear_ids_a"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.ids_a_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.clear_pipeline);
            rpass.set_bind_group(0, &self.clear_bg_a, &[]);
            rpass.draw(0..4, 0..1);
        }

        // 2) Seed splat into A
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("seed_splat"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.ids_a_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.seed_splat_pipeline);
            rpass.set_bind_group(0, &self.seed_bg, &[]);
            rpass.draw(0..self.seed_count, 0..1);
        }

        // 3) JFA passes, ping-pong A<->B

        let max_dim = self.size.0.max(self.size.1);
        let mut step = 1u32;
        while step < max_dim {
            step <<= 1;
        }
        step >>= 1;

        let mut flip = false;
        let mut is_first_jfa_pass = true;
        while step >= 1 {
            let pj = ParamsJfa {
                width: self.size.0,
                height: self.size.1,
                step,
                _pad: 0,
            };
            let staging = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("params_jfa_staging"),
                contents: bytemuck::bytes_of(&pj),
                usage: wgpu::BufferUsages::COPY_SRC,
            });
            encoder.copy_buffer_to_buffer(
                &staging,
                0,
                &self.params_jfa_buf,
                0,
                std::mem::size_of::<ParamsJfa>() as u64,
            );
            {
                // On first pass writing to B, clear it. After that, always load previous content.
                let load_op = if is_first_jfa_pass && !flip {
                    wgpu::LoadOp::Clear(wgpu::Color::WHITE)
                } else {
                    wgpu::LoadOp::Load
                };

                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("jfa_step"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: if !flip {
                            &self.ids_b_view
                        } else {
                            &self.ids_a_view
                        },
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: load_op,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                rpass.set_pipeline(&self.jfa_pipeline);
                rpass.set_bind_group(
                    0,
                    if !flip {
                        &self.jfa_bg_a_to_b
                    } else {
                        &self.jfa_bg_b_to_a
                    },
                    &[],
                );
                rpass.draw(0..4, 0..1);
            }
            is_first_jfa_pass = false;
            flip = !flip;
            step >>= 1;
        }

        // if self.refined {
        //     for _ in 0..2 {
        //         let pj = ParamsJfa {
        //             width: self.size.0,
        //             height: self.size.1,
        //             step: 1,
        //             _pad: 0,
        //         };
        //         let staging = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //             label: Some("params_jfa_staging"),
        //             contents: bytemuck::bytes_of(&pj),
        //             usage: wgpu::BufferUsages::COPY_SRC,
        //         });
        //         encoder.copy_buffer_to_buffer(
        //             &staging,
        //             0,
        //             &self.params_jfa_buf,
        //             0,
        //             std::mem::size_of::<ParamsJfa>() as u64,
        //         );
        //         {
        //             let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        //                 label: Some("jfa_step"),
        //                 timestamp_writes: None,
        //             });
        //             cpass.set_pipeline(&self.jfa_pipeline);
        //             cpass.set_bind_group(
        //                 0,
        //                 if !flip {
        //                     &self.jfa_bg_a_to_b
        //                 } else {
        //                     &self.jfa_bg_b_to_a
        //                 },
        //                 &[],
        //             );
        //             cpass.dispatch_workgroups(groups_x, groups_y, 1);
        //         }
        //         flip = !flip;
        //     }
        // }

        // 4) Shade to color (the final IDs are in A if flip is true, else in B).
        // Our shade BG was built with ids_a_view at binding 0. If the last write ended in B,
        // we temporarily rebind with B for this dispatch.
        let shade_with_b = flip; // if true, IDs live in B
        if shade_with_b {
            let tmp_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bg_shade_tmp_b"),
                layout: &self.shade_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.ids_b_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.nearest_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.seed_tex_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&self.color_lookup_tex_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: self.params_common_buf.as_entire_binding(),
                    },
                ],
            });
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shade"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.shade_pipeline);
            rpass.set_bind_group(0, &tmp_bg, &[]);
            rpass.draw(0..4, 0..1);
        } else {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shade"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.shade_pipeline);
            rpass.set_bind_group(0, &self.shade_bg, &[]);
            rpass.draw(0..4, 0..1);
        }

        // Submit
        rs.queue.submit(std::iter::once(encoder.finish()));
    }
}
