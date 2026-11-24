//! Resources for the SMAA 1x anti-aliasing implementation.
//!
//! SMAA needs two screen sized intermediate textures (edge detection result and
//! blending weights), the two reference lookup textures and two clamp-to-edge
//! samplers. The area and search lookup textures are the unmodified reference
//! textures from the MIT licensed SMAA implementation by Jorge Jimenez et al.
//! (https://github.com/iryoku/smaa), stored as raw RG8/R8 binary data.

use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, Device, Extent3d, FilterMode, Origin3d, Queue, Sampler, SamplerDescriptor, ShaderStages, TexelCopyBufferLayout,
    TexelCopyTextureInfo, TextureAspect, TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension,
};

use crate::graphics::{AttachmentTexture, AttachmentTextureFactory, AttachmentTextureType, RENDER_TO_TEXTURE_FORMAT, ScreenSize};

/// The format of the texture that stores the result of the edge detection pass.
pub(crate) const SMAA_EDGE_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rg8Unorm;
/// The format of the texture that stores the blending weights.
pub(crate) const SMAA_BLEND_WEIGHTS_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

/// Area lookup texture: 160x560, RG8, extracted from `Textures/AreaTex.h` of
/// the SMAA reference implementation.
static SMAA_AREA_TEXTURE_DATA: &[u8] = include_bytes!("../../archive/data/anti_aliasing/smaa_area_tex.bin");
/// Search lookup texture: 64x16, R8, extracted from `Textures/SearchTex.h` of
/// the SMAA reference implementation.
static SMAA_SEARCH_TEXTURE_DATA: &[u8] = include_bytes!("../../archive/data/anti_aliasing/smaa_search_tex.bin");

const SMAA_AREA_TEXTURE_WIDTH: u32 = 160;
const SMAA_AREA_TEXTURE_HEIGHT: u32 = 560;
const SMAA_SEARCH_TEXTURE_WIDTH: u32 = 64;
const SMAA_SEARCH_TEXTURE_HEIGHT: u32 = 16;

/// Contains everything the SMAA passes need besides the color texture that is
/// rendered into.
pub(crate) struct SmaaResources {
    pub(crate) edge_texture: AttachmentTexture,
    pub(crate) blend_weights_texture: AttachmentTexture,
    /// The result of the neighborhood blending pass. The final pass cannot
    /// sample from and write to the color texture at the same time, so the
    /// result is written here and then blitted into the color texture.
    pub(crate) result_texture: AttachmentTexture,

    edge_bind_group: BindGroup,
    blend_weights_bind_group: BindGroup,
    blend_bind_group: BindGroup,
}

impl SmaaResources {
    pub(crate) fn new(device: &Device, queue: &Queue, screen_size: ScreenSize, color_texture: &AttachmentTexture) -> Self {
        let factory = AttachmentTextureFactory::new(device, screen_size, 1, None);

        let edge_texture = factory.new_attachment("smaa edges", SMAA_EDGE_TEXTURE_FORMAT, AttachmentTextureType::ColorAttachment);
        let blend_weights_texture = factory.new_attachment(
            "smaa blend weights",
            SMAA_BLEND_WEIGHTS_TEXTURE_FORMAT,
            AttachmentTextureType::ColorAttachment,
        );
        let result_texture = factory.new_attachment("smaa result", RENDER_TO_TEXTURE_FORMAT, AttachmentTextureType::ColorAttachment);

        let nearest_sampler = create_clamp_sampler(device, FilterMode::Nearest);
        let linear_sampler = create_clamp_sampler(device, FilterMode::Linear);

        let area_texture = create_lookup_texture(
            device,
            queue,
            "smaa area",
            SMAA_AREA_TEXTURE_WIDTH,
            SMAA_AREA_TEXTURE_HEIGHT,
            TextureFormat::Rg8Unorm,
            SMAA_AREA_TEXTURE_DATA,
        );
        let search_texture = create_lookup_texture(
            device,
            queue,
            "smaa search",
            SMAA_SEARCH_TEXTURE_WIDTH,
            SMAA_SEARCH_TEXTURE_HEIGHT,
            TextureFormat::R8Unorm,
            SMAA_SEARCH_TEXTURE_DATA,
        );

        let area_view = area_texture.create_view(&TextureViewDescriptor::default());
        let search_view = search_texture.create_view(&TextureViewDescriptor::default());

        let edge_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("smaa edge"),
            layout: smaa_edge_bind_group_layout(device),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(color_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&nearest_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&linear_sampler),
                },
            ],
        });

        let blend_weights_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("smaa blend weights"),
            layout: smaa_weights_bind_group_layout(device),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(edge_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&area_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&search_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&nearest_sampler),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&linear_sampler),
                },
            ],
        });

        let blend_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("smaa blend"),
            layout: smaa_blend_bind_group_layout(device),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(color_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(blend_weights_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&nearest_sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&linear_sampler),
                },
            ],
        });

        Self {
            edge_texture,
            blend_weights_texture,
            result_texture,
            edge_bind_group,
            blend_weights_bind_group,
            blend_bind_group,
        }
    }

    pub(crate) fn edge_bind_group(&self) -> &BindGroup {
        &self.edge_bind_group
    }

    pub(crate) fn blend_weights_bind_group(&self) -> &BindGroup {
        &self.blend_weights_bind_group
    }

    pub(crate) fn blend_bind_group(&self) -> &BindGroup {
        &self.blend_bind_group
    }
}

fn create_clamp_sampler(device: &Device, filter_mode: FilterMode) -> Sampler {
    device.create_sampler(&SamplerDescriptor {
        label: Some("smaa sampler"),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: filter_mode,
        min_filter: filter_mode,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: 0.0,
        compare: None,
        anisotropy_clamp: 1,
        border_color: None,
    })
}

fn create_lookup_texture(
    device: &Device,
    queue: &Queue,
    label: &str,
    width: u32,
    height: u32,
    format: TextureFormat,
    data: &[u8],
) -> wgpu::Texture {
    let bytes_per_pixel: u32 = match format {
        TextureFormat::Rg8Unorm => 2,
        TextureFormat::R8Unorm => 1,
        _ => panic!("unsupported SMAA lookup texture format"),
    };

    assert_eq!(
        data.len(),
        (width * height * bytes_per_pixel) as usize,
        "invalid SMAA lookup texture data"
    );

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        data,
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * bytes_per_pixel),
            rows_per_image: Some(height),
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    texture
}

/// The bind group layouts are shared between the resource creation (here) and
/// the drawers (in `passes::postprocessing::smaa`), so they live next to the
/// resources.
///
/// The first bind group is always the global bind group of the render pass, the
/// second one contains the textures and samplers of the individual SMAA pass.
macro_rules! texture_entry {
    ($binding:expr) => {
        BindGroupLayoutEntry {
            binding: $binding,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }
    };
}

macro_rules! sampler_entry {
    ($binding:expr) => {
        BindGroupLayoutEntry {
            binding: $binding,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        }
    };
}

/// Bind group layout of the edge detection pass: color texture, nearest sampler
/// and linear sampler.
pub(crate) fn smaa_edge_bind_group_layout(device: &Device) -> &'static BindGroupLayout {
    static LAYOUT: std::sync::OnceLock<BindGroupLayout> = std::sync::OnceLock::new();

    LAYOUT.get_or_init(|| {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("smaa edge"),
            entries: &[texture_entry!(0), sampler_entry!(1), sampler_entry!(2)],
        })
    })
}

/// Bind group layout of the blending weight calculation pass: edge texture,
/// area lookup texture, search lookup texture, nearest sampler and linear
/// sampler.
pub(crate) fn smaa_weights_bind_group_layout(device: &Device) -> &'static BindGroupLayout {
    static LAYOUT: std::sync::OnceLock<BindGroupLayout> = std::sync::OnceLock::new();

    LAYOUT.get_or_init(|| {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("smaa weights"),
            entries: &[
                texture_entry!(0),
                texture_entry!(1),
                texture_entry!(2),
                sampler_entry!(3),
                sampler_entry!(4),
            ],
        })
    })
}

/// Bind group layout of the neighborhood blending pass: color texture, blend
/// weights texture, nearest sampler and linear sampler.
pub(crate) fn smaa_blend_bind_group_layout(device: &Device) -> &'static BindGroupLayout {
    static LAYOUT: std::sync::OnceLock<BindGroupLayout> = std::sync::OnceLock::new();

    LAYOUT.get_or_init(|| {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("smaa blend"),
            entries: &[texture_entry!(0), texture_entry!(1), sampler_entry!(2), sampler_entry!(3)],
        })
    })
}
