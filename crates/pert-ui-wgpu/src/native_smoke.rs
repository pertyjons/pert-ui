//! Native GPU smoke checks: pixel read-back from a real hardware surface.
//! Test evidence only, not a screenshot API; run through `pert-ui-native-checks`.
use super::*;
use pert_ui_runtime::{
    Component, DrawError, Painter, Rect, Runtime, RuntimeError, UpdateContext, Viewport,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

fn srgb_to_linear(byte: u8) -> f32 {
    let encoded = f32::from(byte) / 255.0;
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

// A smoke frame that was not presented did not prepare its scene either.
fn presented(frame: Frame) -> Result<(), RenderError> {
    match frame {
        Frame::Presented => Ok(()),
        other => Err(RenderError::Surface(format!(
            "smoke: {other:?} instead of a presented frame"
        ))),
    }
}

impl Renderer {
    fn verify_vertex_reuse(&mut self) -> Result<(), RenderError> {
        struct Rectangles(usize, Color);
        impl pert_ui_runtime::Component for Rectangles {
            fn update(
                &mut self,
                _: &mut pert_ui_runtime::UpdateContext,
            ) -> Result<(), pert_ui_runtime::RuntimeError> {
                Ok(())
            }
            fn paint(
                &self,
                painter: &mut pert_ui_runtime::Painter,
            ) -> Result<(), pert_ui_runtime::DrawError> {
                for _ in 0..self.0 {
                    painter.fill_rect(pert_ui_runtime::Rect::new(0.0, 0.0, 64.0, 64.0)?, self.1);
                }
                Ok(())
            }
        }
        for (count, color, grows) in [
            (1, Color::TEXT, false),
            (100, Color::SURFACE, true),
            (1, Color::ACCENT, false),
            (0, Color::BACKGROUND, false),
            (1, Color::TEXT, false),
        ] {
            let before = self.vertices.clone();
            let mut ui = pert_ui_runtime::Runtime::new();
            ui.set_viewport(
                pert_ui_runtime::Viewport::new(64.0, 64.0, 1.0)
                    .map_err(|e| RenderError::Graphics(e.to_string()))?,
            )
            .map_err(|e| RenderError::Graphics(e.to_string()))?;
            ui.mount("rectangles", Rectangles(count, color))
                .map_err(|e| RenderError::Graphics(e.to_string()))?;
            ui.flush(std::time::Duration::ZERO)
                .map_err(|e| RenderError::Graphics(e.to_string()))?;
            self.prepare_scene(
                ui.paint()
                    .map_err(|e| RenderError::Graphics(e.to_string()))?,
            )?;
            if (before != self.vertices) != grows {
                return Err(RenderError::Graphics(
                    "smoke: unexpected vertex buffer allocation".into(),
                ));
            }
            self.verify_center_pixel(color)?;
        }
        Ok(())
    }

    // Independent source-over expectations in linear light, followed by sRGB
    // encoding. Read back the actual offscreen destination, including its alpha.
    fn verify_translucent_capture(&mut self) -> Result<(), RenderError> {
        use pert_ui_runtime::{ColorRole, ImageSampling, MaskImage, RasterImage, ThemeMode};
        struct Layers {
            mask: MaskImage,
            image: RasterImage,
        }
        impl Component for Layers {
            fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
                Ok(())
            }
            fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
                painter.fill_rect(
                    Rect::new(16.0, 0.0, 48.0, 64.0)?,
                    Color::rgba(1.0, 0.0, 0.0, 0.5)?,
                );
                painter.draw_mask(
                    &self.mask,
                    Rect::new(32.0, 0.0, 32.0, 64.0)?,
                    Color::rgba(0.0, 1.0, 0.0, 0.5)?,
                );
                painter.draw_image(
                    &self.image,
                    Rect::new(48.0, 0.0, 16.0, 64.0)?,
                    ImageSampling::Nearest,
                );
                Ok(())
            }
        }
        fn source_over(dst: [f32; 4], rgb: [f32; 3], a: f32) -> [f32; 4] {
            [
                rgb[0] * a + dst[0] * (1.0 - a),
                rgb[1] * a + dst[1] * (1.0 - a),
                rgb[2] * a + dst[2] * (1.0 - a),
                a + dst[3] * (1.0 - a),
            ]
        }
        fn check(
            frame: &pert_ui_runtime::FrameImage,
            x: usize,
            expected: [f32; 4],
        ) -> Result<(), Box<dyn std::error::Error>> {
            let offset = (4 * frame.width() as usize + x) * 4;
            let actual = &frame.pixels()[offset..offset + 4];
            for (index, (&actual, &linear)) in actual.iter().zip(&expected).enumerate() {
                let value = if index == 3 {
                    linear
                } else if linear <= 0.0031308 {
                    12.92 * linear
                } else {
                    1.055 * linear.powf(1.0 / 2.4) - 0.055
                };
                if (f32::from(actual) - value * 255.0).abs() > 2.0 {
                    return Err(format!("translucent capture x={x}: {actual:?} channel {index}, linear expectation {expected:?}").into());
                }
            }
            Ok(())
        }
        let old = self.extent.ok_or(RenderError::Unconfigured)?;
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            self.resize(64, 64)?;
            for alpha in [0.0, 0.5, 1.0] {
                let mut ui = Runtime::new();
                ui.set_viewport(Viewport::new(64.0, 64.0, 1.0)?)?;
                ui.theme().set_color(
                    ThemeMode::Dark,
                    ColorRole::Background,
                    Color::rgba(1.0, 1.0, 1.0, alpha)?,
                )?;
                let background = [alpha, alpha, alpha, alpha];
                let empty = self.render_offscreen(ui.paint()?)?;
                check(&empty, 4, background)?;
                ui.mount(
                    "layers",
                    Layers {
                        mask: MaskImage::new(1, 1, vec![255])?,
                        image: RasterImage::from_srgba8(1, 1, vec![0, 0, 255, 128])?,
                    },
                )?;
                ui.flush(Duration::ZERO)?;
                let frame = self.render_offscreen(ui.paint()?)?;
                check(&frame, 4, background)?;
                let fill = source_over(background, [1.0, 0.0, 0.0], 0.5);
                check(&frame, 20, fill)?;
                let mask = source_over(fill, [0.0, 1.0, 0.0], 0.5);
                check(&frame, 36, mask)?;
                check(
                    &frame,
                    52,
                    source_over(mask, [0.0, 0.0, 1.0], 128.0 / 255.0),
                )?;
            }
            Ok(())
        })();
        self.resize(old.width(), old.height())?;
        result.map_err(|error| RenderError::Graphics(error.to_string()))
    }

    fn verify_mixed_scene(&mut self) -> Result<(), RenderError> {
        use pert_ui_runtime::{
            Component, DrawError, Painter, ReadSignal, Runtime, RuntimeError, UpdateContext,
        };
        use std::{
            cell::Cell,
            rc::Rc,
            time::{Duration, Instant},
        };
        struct Probe {
            live: Option<ReadSignal<bool>>,
            color: Color,
            paints: Rc<Cell<usize>>,
            updates: Rc<Cell<usize>>,
        }
        impl Component for Probe {
            fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
                self.updates.set(self.updates.get() + 1);
                if let Some(signal) = &self.live {
                    self.color = if cx.read_paint(signal)? {
                        Color::TEXT
                    } else {
                        Color::ACCENT
                    };
                }
                Ok(())
            }
            fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
                self.paints.set(self.paints.get() + 1);
                painter.fill_rect(
                    pert_ui_runtime::Rect::new(0.0, 0.0, 64.0, 64.0)?,
                    self.color,
                );
                Ok(())
            }
        }
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let mut ui = Runtime::new();
            ui.set_viewport(pert_ui_runtime::Viewport::new(64.0, 64.0, 1.0)?)?;
            let paints = Rc::new(Cell::new(0));
            let updates = Rc::new(Cell::new(0));
            for i in 0..1000 {
                ui.mount(
                    format!("static-{i}"),
                    Probe {
                        live: None,
                        color: Color::SURFACE,
                        paints: paints.clone(),
                        updates: updates.clone(),
                    },
                )?;
            }
            let value = ui.signal(false);
            ui.mount(
                "changing",
                Probe {
                    live: Some(value.read_only()),
                    color: Color::ACCENT,
                    paints: Rc::default(),
                    updates: Rc::default(),
                },
            )?;
            ui.flush(Duration::ZERO)?;
            self.prepare_scene(ui.paint()?)?;
            let buffer = self.vertices.clone();
            let mut update_time = Duration::ZERO;
            let mut compose_time = Duration::ZERO;
            let mut upload_time = Duration::ZERO;
            for tick in 1..=60 {
                value.set(tick % 2 == 1)?;
                let start = Instant::now();
                ui.flush(Duration::from_millis(tick * 16))?;
                update_time += start.elapsed();
                let start = Instant::now();
                let scene = ui.paint()?;
                compose_time += start.elapsed();
                let start = Instant::now();
                self.prepare_scene(scene)?;
                // Submit queued writes; timing does not wait for GPU completion.
                self.queue.submit([]);
                upload_time += start.elapsed();
                assert_eq!(self.vertices, buffer);
            }
            assert_eq!(paints.get(), 1000);
            assert_eq!(updates.get(), 1000);
            self.verify_center_pixel(Color::ACCENT)?;
            println!(
                "Mixed scene, 1000 static rectangles + 1 changing, 60 changes: update {update_time:?}, compose {compose_time:?}, encode/upload submission {upload_time:?}; no new vertex buffers or static callbacks. CPU timings only, not frame latency."
            );
            Ok(())
        })();
        result.map_err(|error| RenderError::Graphics(error.to_string()))
    }

    // Diagnostic only: a persistent attachment and an opaque scene background
    // make loading and repainting a scissored area equivalent to a full clear.
    fn measure_damage_area(&mut self, window: &Window) -> Result<(), RenderError> {
        use pert_ui_runtime::ReadSignal;
        use std::{
            cell::Cell,
            rc::Rc,
            time::{Duration, Instant},
        };

        const WIDTH: u32 = 960;
        const HEIGHT: u32 = 640;
        const DAMAGE: [u32; 4] = [400, 280, 32, 32];
        const WARMUP: u64 = 12;
        const FRAMES: u64 = 300;

        if self.extent != Some(PhysicalExtent::new(WIDTH, HEIGHT)) {
            return Err(RenderError::Graphics(format!(
                "damage probe needs a {WIDTH}x{HEIGHT} physical surface, got {:?}",
                self.extent
            )));
        }
        if !self
            .surface
            .get_capabilities(&self.adapter)
            .usages
            .contains(wgpu::TextureUsages::COPY_DST)
        {
            return Err(RenderError::UnsupportedGraphics(
                "surface does not support COPY_DST for damage probe".into(),
            ));
        }
        self.config.usage |= wgpu::TextureUsages::COPY_DST;
        self.surface.configure(&self.device, &self.config);

        struct Grid(Rc<Cell<usize>>);
        impl Component for Grid {
            fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
                Ok(())
            }
            fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
                self.0.set(self.0.get() + 1);
                painter.fill_rect(
                    Rect::new(0.0, 0.0, 960.0, 640.0)?,
                    painter.color(pert_ui_runtime::ColorRole::Background),
                );
                for index in 0..1000 {
                    painter.fill_rect(
                        Rect::new(
                            (index % 40) as f32 * 24.0,
                            (index / 40) as f32 * 24.0,
                            16.0,
                            16.0,
                        )?,
                        Color::SURFACE,
                    );
                }
                Ok(())
            }
        }
        struct Trace {
            signal: ReadSignal<u8>,
            state: u8,
            paints: Rc<Cell<usize>>,
        }
        impl Component for Trace {
            fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
                self.state = cx.read_paint(&self.signal)?;
                Ok(())
            }
            fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
                self.paints.set(self.paints.get() + 1);
                let (x, width) = match self.state {
                    2 => (450.0, 32.0),
                    3 => (450.0, 8.0),
                    4 => return Ok(()),
                    _ => (400.0, 32.0),
                };
                painter.fill_rect(
                    Rect::new(x, 280.0, width, 32.0)?,
                    if self.state == 1 {
                        Color::rgba(0.08, 0.32, 0.85, 0.5)?
                    } else {
                        Color::rgba(1.0, 1.0, 1.0, 0.5)?
                    },
                );
                Ok(())
            }
        }

        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(WIDTH as f32, HEIGHT as f32, 1.0)?)?;
            let signal = ui.signal(0_u8);
            let grid_paints = Rc::new(Cell::new(0));
            let trace_paints = Rc::new(Cell::new(0));
            let grid = ui.mount("grid", Grid(grid_paints.clone()))?;
            ui.flush(Duration::ZERO)?;
            self.prepare_scene(ui.paint()?)?;

            let texture = |label| {
                self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: WIDTH,
                        height: HEIGHT,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: self.config.format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::COPY_SRC
                        | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                })
            };
            let full = texture("damage probe full frame");
            let partial = texture("damage probe scissored frame");
            let static_layer = texture("damage probe retained grid");
            let layered = texture("damage probe layered result");
            let surface_partial = texture("damage probe surface candidate");
            let full_view = full.create_view(&Default::default());
            let partial_view = partial.create_view(&Default::default());
            let static_view = static_layer.create_view(&Default::default());
            let layered_view = layered.create_view(&Default::default());
            let surface_partial_view = surface_partial.create_view(&Default::default());
            self.submit_probe_pass(&static_view, None)?;
            let trace = ui.mount(
                "trace",
                Trace {
                    signal: signal.read_only(),
                    state: 0,
                    paints: trace_paints.clone(),
                },
            )?;
            assert_ne!(grid, trace);
            ui.flush(Duration::ZERO)?;
            self.prepare_scene(ui.paint()?)?;
            // Initialize both attachments before the first load operation.
            self.submit_probe_pass(&full_view, None)?;
            self.submit_probe_pass(&partial_view, None)?;
            self.submit_probe_pass(&surface_partial_view, None)?;

            let mut full_time = Duration::ZERO;
            let mut partial_time = Duration::ZERO;
            let mut layered_time = Duration::ZERO;
            let mut compose_time = Duration::ZERO;
            let mut prepare_time = Duration::ZERO;
            let mut bright_pixels = None;
            let mut full_surface_samples = Vec::new();
            let mut damage_surface_samples = Vec::new();
            let mut full_acquisition_samples = Vec::new();
            let mut damage_acquisition_samples = Vec::new();
            let mut full_render_work = Vec::new();
            let mut damage_render_work = Vec::new();
            for frame in 0..WARMUP + FRAMES {
                signal.set(u8::from(frame % 2 == 0))?;
                ui.flush(Duration::from_millis(frame + 1))?;
                let start = Instant::now();
                let scene = ui.paint()?;
                let compose_elapsed = start.elapsed();
                if frame >= WARMUP {
                    compose_time += compose_elapsed;
                }
                let start = Instant::now();
                self.prepare_scene(scene)?;
                let prepare_elapsed = start.elapsed();
                if frame >= WARMUP {
                    prepare_time += prepare_elapsed;
                }
                // Rotate order to reduce queue-order and thermal bias.
                let order = match frame % 3 {
                    0 => [0, 1, 2],
                    1 => [1, 2, 0],
                    _ => [2, 0, 1],
                };
                for strategy in order {
                    let elapsed = match strategy {
                        0 => self.submit_probe_pass(&full_view, None)?,
                        1 => self.submit_probe_pass(&partial_view, Some(DAMAGE))?,
                        _ => self.submit_layer_pass(
                            &static_layer,
                            &layered_view,
                            &layered,
                            WIDTH,
                            HEIGHT,
                        )?,
                    };
                    if frame >= WARMUP {
                        match strategy {
                            0 => full_time += elapsed,
                            1 => partial_time += elapsed,
                            _ => layered_time += elapsed,
                        }
                    }
                }
                // Both paths use one submission and wait for completion.
                // The acquisition call and readback are outside these samples;
                // completion can still wait for surface synchronization.
                let (full_sample, damage_sample) = if (frame / 2) % 2 == 0 {
                    let full = self.probe_surface_pass(None, window)?;
                    let damage = self.probe_surface_pass(
                        Some((&surface_partial, &surface_partial_view, DAMAGE)),
                        window,
                    )?;
                    (full, damage)
                } else {
                    let damage = self.probe_surface_pass(
                        Some((&surface_partial, &surface_partial_view, DAMAGE)),
                        window,
                    )?;
                    let full = self.probe_surface_pass(None, window)?;
                    (full, damage)
                };
                let (full_acquisition, full_work) = full_sample;
                let (damage_acquisition, damage_work) = damage_sample;
                if frame >= WARMUP {
                    full_acquisition_samples.push(full_acquisition);
                    damage_acquisition_samples.push(damage_acquisition);
                    full_surface_samples.push(full_work);
                    damage_surface_samples.push(damage_work);
                    full_render_work.push(compose_elapsed + prepare_elapsed + full_work);
                    damage_render_work.push(compose_elapsed + prepare_elapsed + damage_work);
                }
                // Validate both colors, so a candidate that leaves the initial
                // trace untouched cannot pass by matching the last frame.
                if frame == WARMUP || frame == WARMUP + FRAMES - 1 {
                    let full_pixels = self.probe_pixels(&full, WIDTH, HEIGHT)?;
                    let partial_pixels = self.probe_pixels(&partial, WIDTH, HEIGHT)?;
                    let layered_pixels = self.probe_pixels(&layered, WIDTH, HEIGHT)?;
                    let surface_pixels = self.probe_pixels(&surface_partial, WIDTH, HEIGHT)?;
                    if full_pixels != partial_pixels
                        || full_pixels != layered_pixels
                        || full_pixels != surface_pixels
                    {
                        return Err("candidate pass differed from full pass pixels".into());
                    }
                    if frame == WARMUP {
                        bright_pixels = Some(full_pixels);
                    } else if bright_pixels.as_ref() == Some(&full_pixels) {
                        return Err(
                            "changing trace produced identical bright and dark frames".into()
                        );
                    }
                }
            }
            if grid_paints.get() != 1 {
                return Err("unchanged grid was repainted by the runtime".into());
            }
            if trace_paints.get() != (WARMUP + FRAMES + 1) as usize {
                return Err("changing trace did not repaint once per update".into());
            }
            // Old and new bounds are supplied explicitly by this fixture.
            for (index, (state, damage)) in [
                (2, [399, 279, 84, 34]),
                (3, [449, 279, 34, 34]),
                (4, [449, 279, 10, 34]),
                (0, [399, 279, 59, 34]),
            ]
            .into_iter()
            .enumerate()
            {
                signal.set(state)?;
                ui.flush(Duration::from_millis(WARMUP + FRAMES + index as u64 + 1))?;
                self.prepare_scene(ui.paint()?)?;
                self.submit_probe_pass(&full_view, None)?;
                self.submit_probe_pass(&partial_view, Some(damage))?;
                if self.probe_pixels(&full, WIDTH, HEIGHT)?
                    != self.probe_pixels(&partial, WIDTH, HEIGHT)?
                {
                    return Err(format!("damage pixels differed after trace state {state}").into());
                }
            }
            // A theme change invalidates the whole retained target. The next
            // trace-only update must still match the new full-frame theme.
            let before_theme = self.probe_pixels(&full, WIDTH, HEIGHT)?;
            ui.theme().set_mode(pert_ui_runtime::ThemeMode::Light)?;
            ui.flush(Duration::from_millis(WARMUP + FRAMES + 5))?;
            self.prepare_scene(ui.paint()?)?;
            self.submit_probe_pass(&full_view, None)?;
            self.submit_probe_pass(&partial_view, None)?;
            if self.probe_pixels(&full, WIDTH, HEIGHT)? == before_theme {
                return Err("theme change did not change fixture pixels".into());
            }
            signal.set(1)?;
            ui.flush(Duration::from_millis(WARMUP + FRAMES + 6))?;
            self.prepare_scene(ui.paint()?)?;
            self.submit_probe_pass(&full_view, None)?;
            self.submit_probe_pass(&partial_view, Some(DAMAGE))?;
            if self.probe_pixels(&full, WIDTH, HEIGHT)?
                != self.probe_pixels(&partial, WIDTH, HEIGHT)?
            {
                return Err("damage pixels differed after theme change".into());
            }
            // Without a background fill, a scissored load pass does not restore
            // the scene clear color. This is true for either background alpha.
            for background in [Color::BACKGROUND, Color::rgba(0.25, 0.35, 0.45, 0.5)?] {
                let mut bare = Runtime::new();
                bare.set_viewport(Viewport::new(WIDTH as f32, HEIGHT as f32, 1.0)?)?;
                let bare_signal = bare.signal(0_u8);
                bare.mount(
                    "trace",
                    Trace {
                        signal: bare_signal.read_only(),
                        state: 0,
                        paints: Rc::default(),
                    },
                )?;
                bare.theme().set_color(
                    pert_ui_runtime::ThemeMode::Dark,
                    pert_ui_runtime::ColorRole::Background,
                    background,
                )?;
                bare.flush(Duration::ZERO)?;
                self.prepare_scene(bare.paint()?)?;
                self.submit_probe_pass(&partial_view, None)?;
                bare_signal.set(1)?;
                bare.flush(Duration::from_millis(1))?;
                self.prepare_scene(bare.paint()?)?;
                self.submit_probe_pass(&full_view, None)?;
                self.submit_probe_pass(&partial_view, Some(DAMAGE))?;
                if self.probe_pixels(&full, WIDTH, HEIGHT)?
                    == self.probe_pixels(&partial, WIDTH, HEIGHT)?
                {
                    return Err(format!(
                        "scissored scene without background fill matched full at alpha {}",
                        background.alpha()
                    )
                    .into());
                }
            }
            for (name, samples) in [
                ("direct full surface", &mut full_surface_samples),
                ("damage + surface copy", &mut damage_surface_samples),
                ("full acquisition", &mut full_acquisition_samples),
                ("damage acquisition", &mut damage_acquisition_samples),
                ("full render work", &mut full_render_work),
                ("damage render work", &mut damage_render_work),
            ] {
                samples.sort_unstable();
                let median = samples[samples.len() / 2];
                let p95 = samples[(samples.len() * 95).div_ceil(100) - 1];
                println!("Damage-area probe {name}: median {median:?}, p95 {p95:?}");
            }
            println!(
                "Damage-area probe on {:?}: {FRAMES} frames, {WIDTH}x{HEIGHT}, surface mode {:?}, alpha mode {:?}, max frame latency {}, usage {:?}, scale {}; 1000 static rectangles, {DAMAGE:?} changed area; scene composition {compose_time:?}, renderer preparation {prepare_time:?}, full pass {full_time:?}, scissored pass {partial_time:?}, retained-grid copy + trace {layered_time:?}. Pass times include CPU encode + submit + GPU completion; all times exclude readback and presentation calls.",
                self.adapter.get_info(),
                self.config.present_mode,
                self.config.alpha_mode,
                self.config.desired_maximum_frame_latency,
                self.config.usage,
                window.scale_factor()
            );
            Ok(())
        })();
        result.map_err(|error| RenderError::Graphics(error.to_string()))
    }

    pub(super) fn submit_probe_pass(
        &self,
        view: &wgpu::TextureView,
        damage: Option<[u32; 4]>,
    ) -> Result<std::time::Duration, RenderError> {
        let start = std::time::Instant::now();
        let encoder = self.encode_probe_pass(view, damage)?;
        let submission = self.queue.submit([encoder.finish()]);
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(5)),
            })
            .map_err(|error| RenderError::Graphics(format!("damage probe GPU wait: {error}")))?;
        Ok(start.elapsed())
    }

    fn encode_probe_pass(
        &self,
        view: &wgpu::TextureView,
        damage: Option<[u32; 4]>,
    ) -> Result<wgpu::CommandEncoder, RenderError> {
        let encoder = if let Some([x, y, width, height]) = damage {
            let mut encoder = self.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("damage probe scissored scene"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
                pass.set_scissor_rect(x, y, width, height);
                if let Some(vertices) = &self.vertices {
                    pass.set_vertex_buffer(0, vertices.slice(..));
                    for batch in &self.batches {
                        match batch.source {
                            Source::Fill => pass.set_pipeline(&self.pipelines.fill),
                            Source::Mask(index) => {
                                pass.set_pipeline(&self.pipelines.masks.pipeline);
                                pass.set_bind_group(0, self.pipelines.masks.bind(index, 0)?, &[]);
                            }
                            Source::Image { index, filter } => {
                                pass.set_pipeline(&self.pipelines.images.pipeline);
                                pass.set_bind_group(
                                    0,
                                    self.pipelines.images.bind(index, filter)?,
                                    &[],
                                );
                            }
                        }
                        pass.draw(batch.range.clone(), 0..1);
                    }
                }
            }
            encoder
        } else {
            self.encode(view, Destination::Premultiplied)?
        };
        Ok(encoder)
    }

    fn probe_surface_pass(
        &self,
        candidate: Option<(&wgpu::Texture, &wgpu::TextureView, [u32; 4])>,
        window: &Window,
    ) -> Result<(std::time::Duration, std::time::Duration), RenderError> {
        let acquisition_started = std::time::Instant::now();
        let (frame, suboptimal) =
            acquired(self.surface.get_current_texture()).map_err(|outcome| {
                RenderError::Surface(format!("damage probe acquisition returned {outcome:?}"))
            })?;
        if suboptimal {
            return Err(RenderError::Surface(
                "damage probe acquired a suboptimal surface".into(),
            ));
        }
        let acquisition = acquisition_started.elapsed();
        let start = std::time::Instant::now();
        let mut encoder = if let Some((_, view, damage)) = candidate {
            self.encode_probe_pass(view, Some(damage))?
        } else {
            self.encode(
                &frame.texture.create_view(&Default::default()),
                self.presentation_destination(),
            )?
        };
        // This experimental copy-to-surface candidate is presentation-correct
        // for opaque backgrounds. A translucent premultiplied offscreen target
        // cannot be copied unchanged into an opaque compositor destination.
        if let Some((source, _, _)) = candidate {
            let extent = self.extent.ok_or(RenderError::Unconfigured)?;
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: source,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &frame.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: extent.width(),
                    height: extent.height(),
                    depth_or_array_layers: 1,
                },
            );
        }
        let submission = self.queue.submit([encoder.finish()]);
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(5)),
            })
            .map_err(|error| RenderError::Graphics(format!("surface probe GPU wait: {error}")))?;
        let work = start.elapsed();
        window.pre_present_notify();
        self.queue.present(frame);
        Ok((acquisition, work))
    }

    // The grid texture stands in for a layer owned by the grid WidgetId. The
    // current DisplayList has no ownership metadata, so the last six vertices
    // are selected explicitly as this fixture's trace command.
    fn submit_layer_pass(
        &self,
        static_layer: &wgpu::Texture,
        view: &wgpu::TextureView,
        output: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> Result<std::time::Duration, RenderError> {
        let start = std::time::Instant::now();
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let source = |texture| wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        };
        encoder.copy_texture_to_texture(
            source(static_layer),
            source(output),
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let vertices = self
            .vertices
            .as_ref()
            .ok_or_else(|| RenderError::Graphics("damage probe has no vertex buffer".into()))?;
        let batch = self
            .batches
            .last()
            .ok_or_else(|| RenderError::Graphics("damage probe has no trace batch".into()))?;
        if batch.source != Source::Fill || batch.range.end < 6 {
            return Err(RenderError::Graphics(
                "damage probe trace is not a fill".into(),
            ));
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("damage probe retained grid and trace"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.set_pipeline(&self.pipelines.fill);
            pass.draw(batch.range.end - 6..batch.range.end, 0..1);
        }
        let submission = self.queue.submit([encoder.finish()]);
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(5)),
            })
            .map_err(|error| RenderError::Graphics(format!("layer probe GPU wait: {error}")))?;
        Ok(start.elapsed())
    }

    pub(super) fn probe_pixels(
        &self,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, RenderError> {
        let padded = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("damage probe readback"),
            size: u64::from(padded) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = send.send(result);
            });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(5)),
            })
            .map_err(|error| RenderError::Graphics(error.to_string()))?;
        receive
            .recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|error| RenderError::Graphics(error.to_string()))?
            .map_err(|error| RenderError::Graphics(error.to_string()))?;
        let mapped = buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|error| RenderError::Graphics(error.to_string()))?;
        let pixels = mapped.to_vec();
        drop(mapped);
        buffer.unmap();
        Ok(pixels)
    }

    fn verify_split_grip_symmetry(&mut self) -> Result<(), RenderError> {
        use pert_ui_runtime::{
            Container, Element, Palette, Runtime, SplitAxis, SplitPane, Viewport,
        };
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            for axis in [SplitAxis::LeftRight, SplitAxis::TopBottom] {
                let mut ui = Runtime::new();
                let vertical = axis == SplitAxis::LeftRight;
                ui.set_viewport(Viewport::new(
                    if vertical { 8.0 } else { 64.0 },
                    if vertical { 64.0 } else { 8.0 },
                    1.0,
                )?)?;
                ui.mount_tree(Element::new(
                    "split",
                    SplitPane::new(
                        axis,
                        Element::new("a", Container::default()),
                        Element::new("b", Container::default()),
                    ),
                ))?;
                ui.flush(std::time::Duration::ZERO)?;
                self.prepare_scene(ui.paint()?)?;
                let color = Palette::dark().color(pert_ui_runtime::ColorRole::ResizeHandle);
                // Identify the rasterized line first: the top-left fill rule
                // can select either neighboring pixel at a half-pixel boundary.
                let mut line = Vec::new();
                let mut grip = Vec::new();
                let foreground = color.red();
                let background = Color::BACKGROUND.red();
                let threshold = ((foreground + background) / 2.0).powf(1.0 / 2.4) * 255.0;
                for across in 0..8 {
                    let outside = self.read_pixel(
                        if vertical { across } else { 0 },
                        if vertical { 0 } else { across },
                    )?;
                    let middle = self.read_pixel(
                        if vertical { across } else { 4 },
                        if vertical { 4 } else { across },
                    )?;
                    if f32::from(outside[0]) > threshold {
                        line.push(across);
                    }
                    if f32::from(middle[0]) > threshold {
                        grip.push(across);
                    }
                }
                assert_eq!(line.len(), 1);
                assert_eq!(
                    grip,
                    vec![line[0] - 1, line[0], line[0] + 1],
                    "grip must protrude equally around the actual rasterized line"
                );
            }
            Ok(())
        })();
        result.map_err(|error| RenderError::Graphics(error.to_string()))
    }

    fn verify_segmented_control(&mut self) -> Result<(), RenderError> {
        use pert_ui_runtime::{
            Choice, ColorRole, Element, Runtime, SegmentedControl, ThemeMode, Viewport,
        };
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            for mode in [ThemeMode::Dark, ThemeMode::Light] {
                let mut ui = Runtime::new();
                ui.set_viewport(Viewport::new(8.0, 8.0, 1.0)?)?;
                ui.theme().set_mode(mode)?;
                let selected = ui.signal(0);
                ui.mount_tree(Element::new(
                    "segments",
                    SegmentedControl::new(
                        selected.clone(),
                        [Choice::new(0, ""), Choice::new(1, "")],
                    )?,
                ))?;
                for index in [0, 1] {
                    selected.set(index)?;
                    ui.flush(std::time::Duration::ZERO)?;
                    self.prepare_scene(ui.paint()?)?;
                    let palette = ui.theme().palette(mode)?;
                    self.verify_pixel(0, 0, palette.color(ColorRole::Background))?;
                    self.verify_pixel(
                        if index == 0 { 2 } else { 6 },
                        3,
                        palette.color(ColorRole::TabSelected),
                    )?;
                    self.verify_pixel(
                        if index == 0 { 6 } else { 2 },
                        3,
                        palette.color(ColorRole::Tab),
                    )?;
                    // Joined segments draw no underline: each fill reaches the
                    // bottom rows, sampled beside the internal edge so the
                    // exterior rounded corners do not blend into the check.
                    self.verify_pixel(
                        if index == 0 { 3 } else { 5 },
                        6,
                        palette.color(ColorRole::TabSelected),
                    )?;
                    self.verify_pixel(
                        if index == 0 { 5 } else { 3 },
                        6,
                        palette.color(ColorRole::Tab),
                    )?;
                    self.verify_pixel(4, 2, palette.color(ColorRole::Outline))?;
                }
            }
            Ok(())
        })();
        result.map_err(|error| RenderError::Graphics(error.to_string()))
    }

    // Premultiplied sRGB pixels must reach the surface unchanged when opaque,
    // blend in linear light when translucent, and upload once per content.
    fn verify_images(&mut self) -> Result<(), RenderError> {
        use pert_ui_runtime::{
            Component, DrawError, ImageSampling, Painter, RasterImage, Rect, Runtime, RuntimeError,
            UpdateContext, Viewport,
        };
        struct Images(Vec<(RasterImage, [f32; 4], Color)>);
        impl Component for Images {
            fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
                Ok(())
            }
            fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
                for (image, [x, y, w, h], tint) in &self.0 {
                    painter.draw_image_tinted(
                        image,
                        Rect::new(*x, *y, *w, *h)?,
                        ImageSampling::Nearest,
                        *tint,
                    );
                }
                Ok(())
            }
        }
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            // Left half opaque orange, right half half-transparent white; the
            // second copy is equal content from another allocation.
            let pixels = [230, 120, 20, 255, 255, 255, 255, 128].to_vec();
            let image = RasterImage::from_srgba8(2, 1, pixels.clone())?;
            let copy = RasterImage::from_srgba8(2, 1, pixels)?;
            let faded = Color::rgba(1.0, 1.0, 1.0, 0.5)?;
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(8.0, 8.0, 1.0)?)?;
            ui.mount(
                "images",
                Images(vec![
                    (image, [0.0, 0.0, 8.0, 4.0], Color::WHITE),
                    (copy, [0.0, 4.0, 8.0, 4.0], faded),
                ]),
            )?;
            ui.flush(std::time::Duration::ZERO)?;
            let scene = ui.paint()?;
            assert_eq!(
                scene.images().len(),
                1,
                "equal content shares one scene entry"
            );
            let background = scene.background();
            self.prepare_scene(scene)?;
            assert_eq!(self.pipelines.images.live_uploads(), 1);
            let orange = Color::rgba(
                srgb_to_linear(230),
                srgb_to_linear(120),
                srgb_to_linear(20),
                1.0,
            )?;
            self.verify_pixel(1, 1, orange)?;
            // Half-transparent white over the background, in linear light.
            let over = |a: f32, background: Color| {
                let [r, g, b, _] = background.channels().map(|c| a + c * (1.0 - a));
                Color::rgba(r, g, b, 1.0)
            };
            self.verify_pixel(6, 1, over(128.0 / 255.0, background)?)?;
            // A tint alpha of one half halves both.
            let half_orange = Color::rgba(
                orange.red() * 0.5 + background.red() * 0.5,
                orange.green() * 0.5 + background.green() * 0.5,
                orange.blue() * 0.5 + background.blue() * 0.5,
                1.0,
            )?;
            self.verify_pixel(1, 6, half_orange)?;
            self.verify_pixel(6, 6, over(64.0 / 255.0, background)?)?;
            Ok(())
        })();
        result.map_err(|error| RenderError::Graphics(error.to_string()))
    }

    // Vector paths reach the adapter as coverage masks; a filled path and a stroked
    // polyline must cover their pixels fully and leave the outside untouched.
    fn verify_geometry(&mut self) -> Result<(), RenderError> {
        use pert_ui_runtime::{
            Component, DrawError, FillRule, Length, Painter, Path, Point, Runtime, RuntimeError,
            StrokeStyle, UpdateContext, Viewport,
        };
        struct Geometry(Color, Color);
        impl Component for Geometry {
            fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
                Ok(())
            }
            fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
                let point = |x, y| Point::new(x, y);
                let triangle =
                    Path::polygon([point(0.0, 0.0)?, point(8.0, 0.0)?, point(0.0, 8.0)?])?;
                painter.fill_path(&triangle, FillRule::NonZero, self.0)?;
                painter.stroke_polyline(
                    &[point(0.0, 6.5)?, point(8.0, 6.5)?],
                    StrokeStyle::new(Length::new(2.0)?),
                    self.1,
                )
            }
        }
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let fill = Color::rgba(0.9, 0.3, 0.1, 1.0)?;
            let stroke = Color::rgba(0.1, 0.7, 0.3, 1.0)?;
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(8.0, 8.0, 1.0)?)?;
            ui.mount("geometry", Geometry(fill, stroke))?;
            ui.flush(std::time::Duration::ZERO)?;
            let scene = ui.paint()?;
            let background = scene.background();
            self.prepare_scene(scene)?;
            self.verify_pixel(1, 1, fill)?;
            self.verify_pixel(3, 2, fill)?;
            self.verify_pixel(6, 6, stroke)?;
            self.verify_pixel(7, 3, background)?;
            Ok(())
        })();
        result.map_err(|error| RenderError::Graphics(error.to_string()))
    }

    // A quarter turn samples the upright coverage through permuted source
    // coordinates: the covered half of a two-texel mask must land at the top when
    // turned right and at the bottom when turned left.
    fn verify_quarter_turns(&mut self) -> Result<(), RenderError> {
        use pert_ui_runtime::{
            Component, DrawError, Painter, QuarterTurn, Rect, Runtime, RuntimeError, UpdateContext,
            Viewport,
        };
        struct Turned(Color);
        impl Component for Turned {
            fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
                Ok(())
            }
            fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
                // Covered on its left, empty on its right.
                let mask = MaskImage::new(2, 1, vec![255, 0])?;
                painter.draw_mask_turned(
                    &mask,
                    Rect::new(0.0, 0.0, 4.0, 8.0)?,
                    self.0,
                    QuarterTurn::Right,
                );
                painter.draw_mask_turned(
                    &mask,
                    Rect::new(4.0, 0.0, 4.0, 8.0)?,
                    self.0,
                    QuarterTurn::Left,
                );
                Ok(())
            }
        }
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let ink = Color::rgba(0.9, 0.3, 0.1, 1.0)?;
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(8.0, 8.0, 1.0)?)?;
            ui.mount("turned", Turned(ink))?;
            ui.flush(std::time::Duration::ZERO)?;
            let scene = ui.paint()?;
            assert_eq!(scene.masks().len(), 1, "both turns share one coverage");
            let background = scene.background();
            self.prepare_scene(scene)?;
            self.verify_pixel(1, 1, ink)?;
            self.verify_pixel(1, 6, background)?;
            self.verify_pixel(6, 6, ink)?;
            self.verify_pixel(6, 1, background)?;
            Ok(())
        })();
        result.map_err(|error| RenderError::Graphics(error.to_string()))
    }

    // A view transform combined with a fractional native scale: a fill and a
    // vector recorded in content space must land on the physical pixels that
    // `offset + scale * p`, times the viewport scale, says they cover.
    fn verify_view_transform(&mut self) -> Result<(), RenderError> {
        use pert_ui_runtime::{
            Component, DrawError, Element, FillRule, Painter, Path, Point, Rect, Runtime,
            RuntimeError, UpdateContext, ViewScale, ViewTransform, Viewport,
        };
        struct Magnified(ViewTransform, Vec<Element>);
        impl Component for Magnified {
            fn take_children(&mut self) -> Vec<Element> {
                std::mem::take(&mut self.1)
            }
            fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
                Ok(())
            }
            fn child_transform(&self) -> ViewTransform {
                self.0
            }
        }
        struct Content(Color, Color);
        impl Component for Content {
            fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
                Ok(())
            }
            fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
                painter.fill_rect(Rect::new(0.0, 0.0, 1.6, 1.6)?, self.0);
                let point = |x, y| Point::new(x, y);
                let square = Path::polygon([
                    point(1.6, 1.6)?,
                    point(2.4, 1.6)?,
                    point(2.4, 2.4)?,
                    point(1.6, 2.4)?,
                ])?;
                painter.fill_path(&square, FillRule::NonZero, self.1)
            }
        }
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let fill = Color::rgba(0.9, 0.3, 0.1, 1.0)?;
            let vector = Color::rgba(0.1, 0.7, 0.3, 1.0)?;
            let mut ui = Runtime::new();
            // Eight physical pixels at a native scale of 1.25.
            ui.set_viewport(Viewport::new(6.4, 6.4, 1.25)?)?;
            // Content at 200% from a logical offset of 0.8, one physical pixel:
            // the fill covers physical 1..5 and the vector 5..7 on both axes.
            let view = ViewTransform::new(ViewScale::new(2.0)?, Point::new(0.8, 0.8)?);
            ui.mount_tree(Element::new(
                "magnified",
                Magnified(view, vec![Element::new("content", Content(fill, vector))]),
            ))?;
            ui.flush(std::time::Duration::ZERO)?;
            let scene = ui.paint()?;
            let background = scene.background();
            self.prepare_scene(scene)?;
            self.verify_pixel(0, 0, background)?;
            self.verify_pixel(1, 1, fill)?;
            self.verify_pixel(4, 4, fill)?;
            self.verify_pixel(5, 5, vector)?;
            self.verify_pixel(6, 6, vector)?;
            self.verify_pixel(7, 7, background)?;
            self.verify_pixel(6, 2, background)?;
            Ok(())
        })();
        result.map_err(|error| RenderError::Graphics(error.to_string()))
    }

    // Native smoke only: render the retained scene into a tiny GPU attachment and
    // read its center pixel. This is test evidence, not a screenshot API.
    pub(crate) fn verify_center_pixel(&self, expected_color: Color) -> Result<(), RenderError> {
        self.verify_pixel(4, 4, expected_color)
    }
    fn verify_pixel(&self, x: u32, y: u32, expected_color: Color) -> Result<(), RenderError> {
        let rgba = self.read_pixel(x, y)?;
        let expected = expected_color.channels().map(|v| {
            if v <= 0.0031308 {
                12.92 * v
            } else {
                1.055 * v.powf(1.0 / 2.4) - 0.055
            }
        });
        if rgba[..3]
            .iter()
            .zip(expected)
            .any(|(actual, expected)| (f32::from(*actual) - expected * 255.0).abs() > 2.0)
            || rgba[3] != 255
        {
            return Err(RenderError::Graphics(format!(
                "smoke: incorrect pixel at ({x}, {y}): {rgba:?}"
            )));
        }
        // `pert-ui-native-checks` finds the adapter by this `Hardware: ` prefix
        // and the `;` after it; its tests pin the text, so keep them in step.
        println!(
            "Hardware: {:?}; pixel ({x}, {y}) RGBA {rgba:?}",
            self.adapter.get_info()
        );
        Ok(())
    }
    fn read_pixel(&self, x: u32, y: u32) -> Result<[u8; 4], RenderError> {
        use std::time::Duration;
        let image = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("smoke attachment"),
            size: wgpu::Extent3d {
                width: 8,
                height: 8,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("smoke pixel"),
            size: 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let view = image.create_view(&Default::default());
        let mut encoder = self.encode(&view, Destination::Premultiplied)?;
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &image,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = send.send(result);
            });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(5)),
            })
            .map_err(|e| RenderError::Graphics(e.to_string()))?;
        receive
            .recv_timeout(Duration::from_secs(5))
            .map_err(|e| RenderError::Graphics(e.to_string()))?
            .map_err(|e| RenderError::Graphics(e.to_string()))?;
        let bytes = buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|e| RenderError::Graphics(e.to_string()))?;
        let rgb = match self.config.format {
            wgpu::TextureFormat::Bgra8UnormSrgb => [bytes[2], bytes[1], bytes[0]],
            wgpu::TextureFormat::Rgba8UnormSrgb => [bytes[0], bytes[1], bytes[2]],
            _ => {
                return Err(RenderError::Graphics(
                    "smoke: expected 8-bit sRGB surface".into(),
                ));
            }
        };
        let rgba = [rgb[0], rgb[1], rgb[2], bytes[3]];
        drop(bytes);
        buffer.unmap();
        Ok(rgba)
    }
}

struct Rectangle;
impl Component for Rectangle {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        painter.draw_mask(
            &pert_ui_runtime::MaskImage::new(2, 2, vec![255u8; 4])?,
            Rect::new(16.0, 16.0, 32.0, 32.0)?,
            Color::ACCENT,
        );
        Ok(())
    }
}
struct Harness {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    scene: DisplayList,
    error: Option<RenderError>,
    attempts: u8,
    damage_probe: bool,
    proxy: EventLoopProxy<Notice>,
}
impl ApplicationHandler<Notice> for Harness {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let result = (|| {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("pert-ui GPU verification")
                            .with_inner_size(if self.damage_probe {
                                winit::dpi::Size::Physical(winit::dpi::PhysicalSize::new(960, 640))
                            } else {
                                winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(64.0, 64.0))
                            }),
                    )
                    .map_err(|e| RenderError::Graphics(e.to_string()))?,
            );
            let size = window.inner_size();
            let proxy = self.proxy.clone();
            self.renderer = Some(Renderer::new(
                window.clone(),
                size.width,
                size.height,
                move |event| {
                    let _ = proxy.send_event(event);
                },
            )?);
            window.request_redraw();
            self.window = Some(window);
            Ok::<_, RenderError>(())
        })();
        if let Err(error) = result {
            self.error = Some(error);
            event_loop.exit();
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if matches!(event, WindowEvent::RedrawRequested)
            && let (Some(renderer), Some(window)) = (&mut self.renderer, &self.window)
        {
            if self.damage_probe {
                self.error = renderer.measure_damage_area(window).err();
                event_loop.exit();
                return;
            }
            match renderer.render(self.scene.clone(), || window.pre_present_notify()) {
                Ok(Frame::Presented) => {
                    self.error = (|| {
                        renderer.verify_center_pixel(Color::ACCENT)?;
                        renderer.verify_translucent_capture()?;
                        renderer.verify_vertex_reuse()?;
                        renderer.verify_mixed_scene()?;
                        renderer.verify_split_grip_symmetry()?;
                        renderer.verify_segmented_control()?;
                        renderer.verify_images()?;
                        renderer.verify_geometry()?;
                        renderer.verify_quarter_turns()?;
                        renderer.verify_view_transform()?;
                        // Verify the clear color without geometry covering the sample pixel.
                        let mut background = Runtime::new();
                        background
                            .theme()
                            .set_mode(pert_ui_runtime::ThemeMode::Light)
                            .map_err(|e| RenderError::Graphics(e.to_string()))?;
                        let light = background
                            .paint()
                            .map_err(|e| RenderError::Graphics(e.to_string()))?;
                        let expected = light.background();
                        renderer.scene = Some(light);
                        renderer.vertices = None;
                        renderer.batches.clear();
                        renderer.verify_center_pixel(expected)?;
                        struct CutTab;
                        impl Component for CutTab {
                            fn update(
                                &mut self,
                                _: &mut UpdateContext,
                            ) -> Result<(), RuntimeError> {
                                Ok(())
                            }
                            fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
                                painter.fill_shape(
                                    Rect::new(0.0, 0.0, 64.0, 64.0)?,
                                    pert_ui_runtime::Shape::CutTopRight(32.into()),
                                    Color::ACCENT,
                                )
                            }
                        }
                        let mut shapes = Runtime::new();
                        shapes
                            .set_viewport(
                                Viewport::new(64.0, 64.0, 1.25)
                                    .map_err(|e| RenderError::Graphics(e.to_string()))?,
                            )
                            .map_err(|e| RenderError::Graphics(e.to_string()))?;
                        shapes
                            .mount("cut-tab", CutTab)
                            .map_err(|e| RenderError::Graphics(e.to_string()))?;
                        shapes
                            .flush(std::time::Duration::ZERO)
                            .map_err(|e| RenderError::Graphics(e.to_string()))?;
                        presented(
                            renderer.render(
                                shapes
                                    .paint()
                                    .map_err(|e| RenderError::Graphics(e.to_string()))?,
                                || window.pre_present_notify(),
                            )?,
                        )?;
                        renderer.verify_pixel(7, 0, Color::BACKGROUND)?;
                        renderer.verify_pixel(0, 0, Color::ACCENT)?;
                        renderer.verify_pixel(7, 7, Color::ACCENT)?;
                        struct Oversized(u32);
                        impl Component for Oversized {
                            fn update(
                                &mut self,
                                _: &mut UpdateContext,
                            ) -> Result<(), RuntimeError> {
                                Ok(())
                            }
                            fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
                                painter.draw_mask(
                                    &pert_ui_runtime::MaskImage::new(
                                        self.0,
                                        1,
                                        vec![255; self.0 as usize],
                                    )?,
                                    Rect::new(0.0, 0.0, 64.0, 64.0)?,
                                    Color::ACCENT,
                                );
                                Ok(())
                            }
                        }
                        let mut ui = Runtime::new();
                        ui.set_viewport(
                            Viewport::new(64.0, 64.0, 1.0)
                                .map_err(|e| RenderError::Graphics(e.to_string()))?,
                        )
                        .map_err(|e| RenderError::Graphics(e.to_string()))?;
                        ui.mount(
                            "bad",
                            Oversized(renderer.device.limits().max_texture_dimension_2d + 1),
                        )
                        .map_err(|e| RenderError::Graphics(e.to_string()))?;
                        ui.flush(std::time::Duration::ZERO)
                            .map_err(|e| RenderError::Graphics(e.to_string()))?;
                        let bad = ui
                            .paint()
                            .map_err(|e| RenderError::Graphics(e.to_string()))?;
                        if !matches!(
                            renderer.render(bad, || {}),
                            Err(RenderError::Limit(LimitError::ResourceExtent {
                                kind: ResourceKind::Mask,
                                ..
                            }))
                        ) {
                            return Err(RenderError::Graphics(
                                "oversized resource was not reported as a mask limit".into(),
                            ));
                        }
                        presented(
                            renderer.render(self.scene.clone(), || window.pre_present_notify())?,
                        )?;
                        renderer.verify_center_pixel(Color::ACCENT)
                    })()
                    .err();
                    event_loop.exit();
                }
                Ok(_) if self.attempts < 3 => {
                    self.attempts += 1;
                    window.request_redraw();
                }
                Ok(_) => {
                    self.error = Some(RenderError::Surface("smoke acquisition failed".into()));
                    event_loop.exit();
                }
                Err(error) => {
                    self.error = Some(error);
                    event_loop.exit();
                }
            }
        }
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: Notice) {
        let (Notice::Error(message) | Notice::Lost(message)) = event;
        self.error = Some(RenderError::Graphics(message));
        event_loop.exit();
    }
}
// libtest runs each test on a worker thread. Linux Wayland and Windows accept
// that explicitly; AppKit only accepts the process main thread, so macOS
// reports an error instead of letting winit panic.
pub(crate) fn native_test_event_loop() -> Result<EventLoop<Notice>, Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    let event_loop = {
        use winit::platform::wayland::EventLoopBuilderExtWayland;
        EventLoop::with_user_event()
            .with_wayland()
            .with_any_thread(true)
            .build()
    };
    #[cfg(target_os = "windows")]
    let event_loop = {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        EventLoop::with_user_event().with_any_thread(true).build()
    };
    #[cfg(target_os = "macos")]
    let event_loop = {
        if std::thread::current().name() != Some("main") {
            // `pert-ui-native-checks` reports this text as "unsupported" rather
            // than failed; its tests pin the text, so keep them in step.
            return Err(
                "macOS native tests need the process main thread, which libtest does not provide"
                    .into(),
            );
        }
        EventLoop::with_user_event().build()
    };
    Ok(event_loop?)
}
/// Presents a scene on a real surface and verifies read-back pixels.
pub(super) fn pixel_readback() -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(64.0, 64.0, 1.0)?)?;
    runtime.mount("rectangle", Rectangle)?;
    runtime.flush(std::time::Duration::ZERO)?;
    let event_loop = native_test_event_loop()?;
    let mut harness = Harness {
        window: None,
        renderer: None,
        scene: runtime.paint()?,
        error: None,
        attempts: 0,
        damage_probe: false,
        proxy: event_loop.create_proxy(),
    };
    event_loop.run_app(&mut harness)?;
    if let Some(error) = harness.error {
        return Err(error.into());
    }
    Ok(())
}

/// Measures a candidate dirty-area render pass against the current full pass.
pub(super) fn damage_area_probe() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = native_test_event_loop()?;
    let mut harness = Harness {
        window: None,
        renderer: None,
        scene: Runtime::new().paint()?,
        error: None,
        attempts: 0,
        damage_probe: true,
        proxy: event_loop.create_proxy(),
    };
    event_loop.run_app(&mut harness)?;
    if let Some(error) = harness.error {
        return Err(error.into());
    }
    Ok(())
}

// Creates a renderer without a size, then walks it through sizing and
// minimizing: while it has no nonzero extent, `render` must report
// `Frame::Occluded` without touching the surface and a capture must fail with
// `RenderError::Unconfigured`; once sized, a capture has the requested size.
struct ZeroSize {
    error: Option<Box<dyn std::error::Error>>,
}
impl ApplicationHandler<Notice> for ZeroSize {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("pert-ui zero-size verification")
                        .with_inner_size(winit::dpi::PhysicalSize::new(48, 32)),
                )?,
            );
            let mut renderer = Renderer::new(window.clone(), 0, 0, |_| {})?;
            let adapter = renderer.adapter.limits().max_texture_dimension_2d;
            let device = renderer.device.limits().max_texture_dimension_2d;
            println!("Texture dimension limit: adapter {adapter}, device {device}");
            if device != adapter {
                return Err(format!("device limit {device} is not the adapter's {adapter}").into());
            }
            let scene =
                |width: f32, height: f32| -> Result<DisplayList, Box<dyn std::error::Error>> {
                    let mut ui = Runtime::new();
                    ui.set_viewport(Viewport::new(width, height, 1.0)?)?;
                    Ok(ui.paint()?)
                };
            let unconfigured = |renderer: &mut Renderer| -> Result<(), Box<dyn std::error::Error>> {
                if renderer.extent().is_some() {
                    return Err("an unsized renderer reports an extent".into());
                }
                if !matches!(renderer.render(scene(48.0, 32.0)?, || {})?, Frame::Occluded) {
                    return Err("an unsized renderer did not report occlusion".into());
                }
                if !matches!(
                    renderer.render_offscreen(scene(48.0, 32.0)?),
                    Err(RenderError::Unconfigured)
                ) {
                    return Err("an unsized capture did not fail as unconfigured".into());
                }
                Ok(())
            };
            unconfigured(&mut renderer)?;
            renderer.resize(48, 32)?;
            if renderer.extent() != Some(PhysicalExtent::new(48, 32)) {
                return Err("resize did not record the extent".into());
            }
            let frame = renderer.render_offscreen(scene(48.0, 32.0)?)?;
            if (frame.width(), frame.height()) != (48, 32) {
                return Err("the capture does not have the configured size".into());
            }
            renderer.resize(0, 0)?;
            unconfigured(&mut renderer)?;
            Ok(())
        })();
        self.error = result.err();
        event_loop.exit();
    }
    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}

/// Checks the unconfigured-surface contract on a real surface.
pub(super) fn zero_size() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = native_test_event_loop()?;
    let mut check = ZeroSize { error: None };
    event_loop.run_app(&mut check)?;
    match check.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
