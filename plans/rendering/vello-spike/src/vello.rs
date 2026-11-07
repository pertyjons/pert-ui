//! The workloads encoded for vello_cpu and vello_gpu, which share one
//! drawing vocabulary through vello_common, and both renderers' frame loops.
use crate::workloads::{self, Item, Rgba};
use std::time::{Duration, Instant};
use vello_cpu::color::{AlphaColor, Srgb};
use vello_cpu::kurbo::{Affine, BezPath, Cap, Join, Point, Rect, RoundedRect, Shape, Stroke};
use vello_cpu::peniko::Gradient;
use vello_cpu::PaintType;

pub fn color(c: Rgba) -> AlphaColor<Srgb> {
    AlphaColor::from_rgba8(c[0], c[1], c[2], c[3])
}

fn rect(r: workloads::Rect) -> Rect {
    Rect::new(f64::from(r.x), f64::from(r.y), f64::from(r.x + r.w), f64::from(r.y + r.h))
}

/// The drawing calls both vello renderers offer with the same signatures.
pub trait Canvas {
    fn set_transform(&mut self, transform: Affine);
    fn set_paint(&mut self, paint: PaintType);
    fn set_stroke(&mut self, stroke: Stroke);
    fn fill_rect(&mut self, rect: &Rect);
    fn fill_path(&mut self, path: &BezPath);
    fn stroke_path(&mut self, path: &BezPath);
}

macro_rules! canvas {
    ($ty:ty) => {
        impl Canvas for $ty {
            fn set_transform(&mut self, transform: Affine) {
                <$ty>::set_transform(self, transform)
            }
            fn set_paint(&mut self, paint: PaintType) {
                <$ty>::set_paint(self, paint)
            }
            fn set_stroke(&mut self, stroke: Stroke) {
                <$ty>::set_stroke(self, stroke)
            }
            fn fill_rect(&mut self, rect: &Rect) {
                <$ty>::fill_rect(self, rect)
            }
            fn fill_path(&mut self, path: &BezPath) {
                <$ty>::fill_path(self, path)
            }
            fn stroke_path(&mut self, path: &BezPath) {
                <$ty>::stroke_path(self, path)
            }
        }
    };
}
canvas!(vello_cpu::RenderContext);
canvas!(vello_gpu::Scene);

/// What a frame draws, in logical pixels; `encode` turns it into calls.
pub enum Frame<'a> {
    Panels(&'a [Item]),
    Polyline(&'a [(f32, f32)]),
    Bars(&'a [(workloads::Rect, workloads::Rect)]),
}

pub fn encode(canvas: &mut impl Canvas, frame: &Frame<'_>, scale: f64) {
    canvas.set_transform(Affine::scale(scale));
    let (w, h) = workloads::LOGICAL;
    canvas.set_paint(color(workloads::BACKGROUND).into());
    canvas.fill_rect(&Rect::new(0.0, 0.0, f64::from(w), f64::from(h)));
    match frame {
        Frame::Panels(items) => {
            // The border is stroked centered on the outline inset by half its
            // width, so it lies inside the rectangle with concentric corners,
            // as pert-ui's inward `stroke_shape` draws it.
            canvas.set_stroke(Stroke::new(1.0));
            for item in *items {
                match *item {
                    Item::Panel(p) => {
                        let r = rect(p.rect);
                        let radius = f64::from(p.radius);
                        canvas.set_paint(color(p.fill).into());
                        canvas.fill_path(&RoundedRect::from_rect(r, radius).to_path(0.1));
                        canvas.set_paint(color(p.border).into());
                        canvas.stroke_path(
                            &RoundedRect::from_rect(r.inset(-0.5), radius - 0.5).to_path(0.1),
                        );
                    }
                    Item::Plain(r, c) => {
                        canvas.set_paint(color(c).into());
                        canvas.fill_rect(&rect(r));
                    }
                }
            }
        }
        Frame::Polyline(points) => {
            let mut path = BezPath::new();
            for (i, &(x, y)) in points.iter().enumerate() {
                let p = Point::new(f64::from(x), f64::from(y));
                if i == 0 { path.move_to(p) } else { path.line_to(p) }
            }
            // pert-ui's StrokeStyle::new has butt caps; kurbo's default is round.
            canvas.set_stroke(
                Stroke::new(f64::from(workloads::LINE_WIDTH))
                    .with_join(Join::Round)
                    .with_caps(Cap::Butt),
            );
            canvas.set_paint(color(workloads::LINE).into());
            canvas.stroke_path(&path);
        }
        Frame::Bars(bars) => {
            let m = workloads::METER;
            let bottom = f64::from(m.y + m.h);
            let top = f64::from(m.y);
            let stops = workloads::STOPS.map(|(t, c)| (t, color(c)));
            for &(full, lit) in *bars {
                canvas.set_paint(color(workloads::TRACK).into());
                canvas.fill_rect(&rect(full));
                if lit.h > 0.0 {
                    let x = f64::from(full.x);
                    let gradient = Gradient::new_linear((x, bottom), (x, top)).with_stops(stops);
                    canvas.set_paint(gradient.into());
                    canvas.fill_rect(&rect(lit));
                }
            }
        }
    }
}

/// vello_cpu: one context per thread setting, reused across frames.
pub struct Cpu {
    context: vello_cpu::RenderContext,
    resources: vello_cpu::Resources,
    pixmap: vello_cpu::Pixmap,
}

impl Cpu {
    pub fn new(width: u16, height: u16, threads: u16) -> Self {
        let settings = vello_cpu::RenderSettings { num_threads: threads, ..Default::default() };
        Self {
            context: vello_cpu::RenderContext::new_with(width, height, settings),
            resources: vello_cpu::Resources::new(),
            pixmap: vello_cpu::Pixmap::new(width, height),
        }
    }

    /// Encodes (including `flush`, which waits for the worker threads'
    /// path processing in multithreaded mode) and renders into the pixmap.
    pub fn frame(&mut self, frame: &Frame<'_>, scale: f64) -> (Duration, Duration) {
        let started = Instant::now();
        self.context.reset();
        encode(&mut self.context, frame, scale);
        self.context.flush();
        let encode = started.elapsed();
        let started = Instant::now();
        self.context.render(&mut self.pixmap, &mut self.resources);
        (encode, started.elapsed())
    }

    /// Renders the polyline alone, white on transparent, so the pixmap's
    /// alpha is vello's coverage without any blending against a background.
    pub fn coverage(&mut self, points: &[(f32, f32)], scale: f64) -> Vec<u8> {
        self.context.reset();
        self.context.set_transform(Affine::scale(scale));
        let mut path = BezPath::new();
        for (i, &(x, y)) in points.iter().enumerate() {
            let p = Point::new(f64::from(x), f64::from(y));
            if i == 0 { path.move_to(p) } else { path.line_to(p) }
        }
        self.context.set_stroke(
            Stroke::new(f64::from(workloads::LINE_WIDTH)).with_join(Join::Round).with_caps(Cap::Butt),
        );
        self.context.set_paint(color([255, 255, 255, 255]));
        self.context.stroke_path(&path);
        self.context.flush();
        self.context.render(&mut self.pixmap, &mut self.resources);
        self.pixmap.data_as_u8_slice().chunks(4).map(|p| p[3]).collect()
    }

    pub fn pixels(&self) -> Vec<u8> {
        self.pixmap.data_as_u8_slice().to_vec()
    }
}

/// vello_gpu on its own wgpu device, rendering into an Rgba8Unorm texture.
pub struct Gpu {
    pub info: wgpu::AdapterInfo,
    device: wgpu::Device,
    queue: wgpu::Queue,
    target: Option<Target>,
}

struct Target {
    width: u16,
    height: u16,
    scene: vello_gpu::Scene,
    renderer: vello_gpu::Renderer,
    resources: vello_gpu::Resources,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    depth: wgpu::TextureView,
    readback: wgpu::Buffer,
    padded: u32,
}

/// One vello_gpu frame's phases.
#[derive(Clone, Copy, Debug, Default)]
pub struct GpuTimes {
    /// Scene building: `Scene` processes paths into strips as they are added.
    pub encode: Duration,
    /// `Renderer::render`: scheduling, buffer/texture uploads, pass recording.
    pub prepare: Duration,
    /// `queue.submit` of the render and a blocking poll until it completes.
    pub gpu: Duration,
    /// Copy to a buffer, submit, map, wait and copy the rows out as RGBA8.
    pub readback: Duration,
}

impl Gpu {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            ..Default::default()
        }))?;
        let info = adapter.get_info();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("vello_gpu spike"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
            ..Default::default()
        }))?;
        Ok(Self { info, device, queue, target: None })
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        if self.target.as_ref().is_some_and(|t| t.width == width && t.height == height) {
            return;
        }
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vello_gpu target"),
            size: wgpu::Extent3d { width: width.into(), height: height.into(), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let (renderer, resources) = vello_gpu::Renderer::new(
            &self.device,
            &vello_gpu::RenderTargetConfig { format, width, height },
        );
        let depth = vello_gpu::Renderer::create_depth_texture_view(
            &self.device,
            &vello_gpu::RenderSize { width, height },
        );
        let padded = (u32::from(width) * 4).next_multiple_of(256);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vello_gpu readback"),
            size: u64::from(padded) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        self.target = Some(Target {
            width,
            height,
            scene: vello_gpu::Scene::new(width, height),
            renderer,
            resources,
            texture,
            view,
            depth,
            readback,
            padded,
        });
    }

    /// Renders `frame` (or an empty scene for `None`) and reads it back.
    pub fn frame(
        &mut self,
        frame: Option<&Frame<'_>>,
        scale: f64,
    ) -> Result<(GpuTimes, Vec<u8>), Box<dyn std::error::Error>> {
        let t = self.target.as_mut().ok_or("no target")?;
        let mut times = GpuTimes::default();
        let started = Instant::now();
        t.scene.reset();
        if let Some(frame) = frame {
            encode(&mut t.scene, frame, scale);
        }
        times.encode = started.elapsed();

        let started = Instant::now();
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let size = vello_gpu::RenderSize { width: t.width, height: t.height };
        t.renderer
            .render(
                &t.scene,
                &mut t.resources,
                &self.device,
                &self.queue,
                &mut encoder,
                &size,
                &t.view,
                Some(&t.depth),
                &vello_gpu::TextureBindings::new(),
                vello_gpu::TargetInit::Clear(vello_gpu::ClearSettings::default()),
            )
            .map_err(|e| format!("vello_gpu render: {e:?}"))?;
        let commands = encoder.finish();
        times.prepare = started.elapsed();

        let started = Instant::now();
        self.queue.submit([commands]);
        self.device.poll(wgpu::PollType::wait_indefinitely())?;
        times.gpu = started.elapsed();

        let started = Instant::now();
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &t.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &t.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(t.padded),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d { width: t.width.into(), height: t.height.into(), depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);
        let slice = t.readback.slice(..);
        let (send, receive) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = send.send(result);
        });
        self.device.poll(wgpu::PollType::wait_indefinitely())?;
        receive.recv()??;
        let row = usize::from(t.width) * 4;
        let mut rgba = Vec::with_capacity(row * usize::from(t.height));
        {
            let mapped = slice.get_mapped_range()?;
            for chunk in mapped.chunks(t.padded as usize) {
                rgba.extend_from_slice(&chunk[..row]);
            }
        }
        t.readback.unmap();
        times.readback = started.elapsed();
        Ok((times, rgba))
    }
}
