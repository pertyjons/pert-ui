//! Retained GPU textures for a scene's resource tables, shared device-wide.
//!
//! Each kind of resource, coverage masks and color images, has its own pipeline,
//! texture format and samplers. Uploads are keyed by the resource's public
//! content key and confirmed by equality, so equal content from different
//! allocations, scenes or windows on this device shares one texture, whatever
//! the windows' surface formats: the bind group layout, samplers and upload
//! pool of a kind are device-wide, and only its pipeline is per surface format.
use crate::{LimitError, RenderError, ResourceKind};
use pert_ui_runtime::{MaskImage, RasterImage};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

// What the cache needs from a scene resource.
pub(super) trait Pixels: Clone + PartialEq {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn pixels(&self) -> &[u8];
    fn content_key(&self) -> u64;
}
impl Pixels for MaskImage {
    fn width(&self) -> u32 {
        MaskImage::width(self)
    }
    fn height(&self) -> u32 {
        MaskImage::height(self)
    }
    fn pixels(&self) -> &[u8] {
        MaskImage::pixels(self)
    }
    fn content_key(&self) -> u64 {
        MaskImage::content_key(self)
    }
}
impl Pixels for RasterImage {
    fn width(&self) -> u32 {
        RasterImage::width(self)
    }
    fn height(&self) -> u32 {
        RasterImage::height(self)
    }
    fn pixels(&self) -> &[u8] {
        RasterImage::pixels(self)
    }
    fn content_key(&self) -> u64 {
        RasterImage::content_key(self)
    }
}

// The fixed description of one resource kind.
pub(super) struct Kind {
    pub(super) resource: ResourceKind,
    pub(super) label: &'static str,
    pub(super) shader: &'static str,
    pub(super) blend: wgpu::BlendState,
    pub(super) format: wgpu::TextureFormat,
    pub(super) bytes_per_pixel: u32,
    // One bind group per filter, in this order; a batch selects one by index.
    pub(super) filters: &'static [wgpu::FilterMode],
}

// 8-bit coverage, tinted by a straight-alpha color.
pub(super) const MASKS: Kind = Kind {
    resource: ResourceKind::Mask,
    label: "coverage",
    shader: include_str!("masks.wgsl"),
    blend: wgpu::BlendState::ALPHA_BLENDING,
    format: wgpu::TextureFormat::R8Unorm,
    bytes_per_pixel: 1,
    filters: &[wgpu::FilterMode::Linear],
};
// Premultiplied sRGB-encoded RGBA8, decoded to linear light by the sampler.
pub(super) const IMAGES: Kind = Kind {
    resource: ResourceKind::Image,
    label: "image",
    shader: include_str!("images.wgsl"),
    blend: wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
    format: wgpu::TextureFormat::Rgba8UnormSrgb,
    bytes_per_pixel: 4,
    filters: &[wgpu::FilterMode::Linear, wgpu::FilterMode::Nearest],
};

// Rejects a scene table whose resources exceed the texture dimension or whose
// unique bytes exceed the budget. The table is already deduplicated, so the
// sum counts unique content.
fn check_table<T: Pixels>(
    kind: ResourceKind,
    max: u32,
    budget: usize,
    table: &[T],
) -> Result<(), LimitError> {
    let mut bytes = 0usize;
    for image in table {
        let (width, height) = (image.width(), image.height());
        if width > max || height > max {
            return Err(LimitError::ResourceExtent {
                kind,
                width,
                height,
                max,
            });
        }
        bytes = bytes.saturating_add(image.pixels().len());
        if bytes > budget {
            return Err(LimitError::SceneBudget { kind });
        }
    }
    Ok(())
}

// One live upload: the content it holds and its GPU payload.
pub(super) struct Entry<T, G> {
    image: T,
    gpu: G,
}

// Every live upload of one kind, keyed by content key and confirmed by
// equality. Entries are weak: the bound tables of the renderers that draw a
// resource keep it alive, and a resource no table binds is released. Generic
// over the payload so the bookkeeping is testable without a device.
pub(super) struct Pool<T, G> {
    entries: HashMap<u64, Vec<Weak<Entry<T, G>>>>,
}
impl<T, G> Default for Pool<T, G> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}
impl<T: Pixels, G> Pool<T, G> {
    // The entries for `table`, index-aligned, uploading only content that no
    // live entry holds yet.
    fn bind(&mut self, table: &[T], mut upload: impl FnMut(&T) -> G) -> Vec<Rc<Entry<T, G>>> {
        self.entries.retain(|_, bucket| {
            bucket.retain(|entry| entry.strong_count() > 0);
            !bucket.is_empty()
        });
        let mut bound = Vec::with_capacity(table.len());
        for image in table {
            let bucket = self.entries.entry(image.content_key()).or_default();
            let found = bucket
                .iter()
                .filter_map(Weak::upgrade)
                .find(|entry| entry.image == *image);
            let entry = found.unwrap_or_else(|| {
                let entry = Rc::new(Entry {
                    image: image.clone(),
                    gpu: upload(image),
                });
                bucket.push(Rc::downgrade(&entry));
                entry
            });
            bound.push(entry);
        }
        bound
    }
    // Uploads some renderer still binds.
    #[cfg(test)]
    fn live(&self) -> usize {
        self.entries
            .values()
            .flatten()
            .filter(|entry| entry.strong_count() > 0)
            .count()
    }
}

type BindGroups = Vec<wgpu::BindGroup>;

// The device-wide part of one resource kind: its bind group layout, samplers
// and upload pool, shared by the pipelines of every surface format.
pub(super) struct Shared<T> {
    kind: &'static Kind,
    layout: wgpu::BindGroupLayout,
    samplers: Vec<wgpu::Sampler>,
    pool: RefCell<Pool<T, BindGroups>>,
}
impl<T: Pixels> Shared<T> {
    pub(super) fn new(device: &wgpu::Device, kind: &'static Kind) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(kind.label),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        // Clamp-to-edge addressing is the default, which the image contract
        // requires: border pixels extend rather than wrapping around.
        let samplers = kind
            .filters
            .iter()
            .map(|&filter| {
                device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some(kind.label),
                    mag_filter: filter,
                    min_filter: filter,
                    ..Default::default()
                })
            })
            .collect();
        Self {
            kind,
            layout,
            samplers,
            pool: RefCell::default(),
        }
    }
    fn upload(&self, device: &wgpu::Device, queue: &wgpu::Queue, image: &T) -> BindGroups {
        let size = wgpu::Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(self.kind.label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.kind.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            image.pixels(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(image.width() * self.kind.bytes_per_pixel),
                rows_per_image: Some(image.height()),
            },
            size,
        );
        let view = texture.create_view(&Default::default());
        self.samplers
            .iter()
            .map(|sampler| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(self.kind.label),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(sampler),
                        },
                    ],
                })
            })
            .collect()
    }
}

// One surface format's pipeline for a resource kind and the resources the
// renderer's current scene binds.
pub(super) struct Textures<T> {
    pub(super) pipeline: wgpu::RenderPipeline,
    shared: Rc<Shared<T>>,
    // Index-aligned with the prepared scene's resource table; holding the
    // entries here is what keeps them alive while the scene uses them.
    bound: Vec<Rc<Entry<T, BindGroups>>>,
}
impl<T> Clone for Textures<T> {
    fn clone(&self) -> Self {
        Self {
            pipeline: self.pipeline.clone(),
            shared: self.shared.clone(),
            bound: Vec::new(),
        }
    }
}
impl<T: Pixels> Textures<T> {
    pub(super) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        shared: &Rc<Shared<T>>,
    ) -> Self {
        let kind = shared.kind;
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(kind.label),
            bind_group_layouts: &[Some(&shared.layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(kind.label),
            source: wgpu::ShaderSource::Wgsl(kind.shader.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(kind.label),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 32,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x2,1=>Float32x4,2=>Float32x2],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(kind.blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            shared: shared.clone(),
            bound: Vec::new(),
        }
    }
    // Binds every resource of a scene table, uploading only content that no
    // renderer on this device holds yet. A failure leaves the previous binding.
    pub(super) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        table: &[T],
    ) -> Result<(), RenderError> {
        let kind = self.shared.kind.resource;
        check_table(
            kind,
            device.limits().max_texture_dimension_2d,
            kind.scene_budget(),
            table,
        )?;
        let shared = &self.shared;
        self.bound = shared
            .pool
            .borrow_mut()
            .bind(table, |image| shared.upload(device, queue, image));
        Ok(())
    }
    // The bind group of a scene table index with the filter at `filter`.
    pub(super) fn bind(
        &self,
        index: usize,
        filter: usize,
    ) -> Result<&wgpu::BindGroup, RenderError> {
        self.bound
            .get(index)
            .and_then(|entry| entry.gpu.get(filter))
            .ok_or_else(|| {
                RenderError::Graphics(format!("missing {} resource", self.shared.kind.label))
            })
    }
    #[cfg(test)]
    pub(super) fn live_uploads(&self) -> usize {
        self.shared.pool.borrow().live()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A resource whose content key the test chooses, so collisions are cheap.
    #[derive(Clone, Debug, PartialEq)]
    struct Fake {
        key: u64,
        size: u32,
        pixels: Vec<u8>,
    }
    impl Fake {
        fn new(key: u64, size: u32, fill: u8) -> Self {
            Self {
                key,
                size,
                pixels: vec![fill; (size * size) as usize],
            }
        }
    }
    impl Pixels for Fake {
        fn width(&self) -> u32 {
            self.size
        }
        fn height(&self) -> u32 {
            self.size
        }
        fn pixels(&self) -> &[u8] {
            &self.pixels
        }
        fn content_key(&self) -> u64 {
            self.key
        }
    }

    #[test]
    fn equal_content_from_other_allocations_uploads_once_across_bindings() {
        let mut pool = Pool::<Fake, usize>::default();
        let mut uploads = 0;
        let mut upload = |_: &Fake| {
            uploads += 1;
            uploads
        };
        let first = pool.bind(&[Fake::new(1, 2, 7)], &mut upload);
        // Another renderer, scene or window binds equal content.
        let second = pool.bind(&[Fake::new(1, 2, 7), Fake::new(2, 2, 9)], &mut upload);
        assert_eq!(uploads, 2);
        assert!(Rc::ptr_eq(&first[0], &second[0]));
        assert_eq!(second[1].gpu, 2);
        assert_eq!(pool.live(), 2);
    }

    #[test]
    fn colliding_keys_are_confirmed_by_equality() {
        let mut pool = Pool::<Fake, ()>::default();
        let mut uploads = 0;
        let bound = pool.bind(&[Fake::new(5, 1, 1), Fake::new(5, 1, 2)], |_| uploads += 1);
        assert_eq!(uploads, 2, "different content under one key uploads twice");
        assert!(!Rc::ptr_eq(&bound[0], &bound[1]));
        assert_eq!(bound[1].image.pixels, [2]);
    }

    #[test]
    fn content_no_binding_holds_is_released_and_uploaded_again() {
        let mut pool = Pool::<Fake, ()>::default();
        let mut uploads = 0;
        let bound = pool.bind(&[Fake::new(1, 1, 1)], |_| uploads += 1);
        drop(bound);
        assert_eq!(pool.live(), 0);
        let empty = pool.bind(&[], |_| uploads += 1);
        assert!(empty.is_empty());
        assert!(pool.entries.is_empty(), "binding prunes dead entries");
        let _bound = pool.bind(&[Fake::new(1, 1, 1)], |_| uploads += 1);
        assert_eq!(uploads, 2);
    }

    #[test]
    fn tables_beyond_the_texture_dimension_or_budget_are_typed_limits() {
        let kind = ResourceKind::Image;
        let table = [Fake::new(1, 4, 0), Fake::new(2, 4, 0)];
        assert_eq!(check_table(kind, 4, 32, &table), Ok(()));
        assert_eq!(
            check_table(kind, 3, 32, &table),
            Err(LimitError::ResourceExtent {
                kind,
                width: 4,
                height: 4,
                max: 3
            })
        );
        assert_eq!(
            check_table(kind, 4, 31, &table),
            Err(LimitError::SceneBudget { kind })
        );
    }

    #[test]
    fn the_documented_budgets_hold() {
        assert_eq!(ResourceKind::Mask.scene_budget(), 64 << 20);
        assert_eq!(ResourceKind::Image.scene_budget(), 256 << 20);
        assert_eq!(MASKS.resource, ResourceKind::Mask);
        assert_eq!(IMAGES.resource, ResourceKind::Image);
    }
}
