use std::num::{NonZeroU32, NonZeroU64};
use std::sync::Arc;

use bumpalo::Bump;
use bytemuck::{Pod, Zeroable};
use hashbrown::HashMap;
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites,
    CommandEncoder, CompareFunction, DepthBiasState, DepthStencilState, Device, Face, FragmentState, FrontFace, MultisampleState,
    PipelineCompilationOptions, PipelineLayout, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, ShaderStages, StencilState, TextureFormat, TextureSampleType, TextureView,
    TextureViewDimension, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, ForwardRenderPassContext, RenderPassContext,
};
use crate::graphics::shader_compiler::ShaderCompiler;
use crate::graphics::{BindlessSupport, Buffer, Capabilities, EntityInstruction, GlobalContext, Prepare, RenderInstruction, Texture};

const DRAWER_NAME: &str = "forward entity";
const INITIAL_INSTRUCTION_SIZE: usize = 256;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct InstanceData {
    world: [[f32; 4]; 4],
    frame_part_transform: [[f32; 4]; 4],
    texture_position: [f32; 2],
    texture_size: [f32; 2],
    color: [f32; 4],
    frame_size: [f32; 2],
    extra_depth_offset: f32,
    depth_offset: f32,
    curvature: f32,
    mirror: u32,
    texture_index: i32,
    flags: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum EntityPassMode {
    /// Draws all opaque parts of an entity.
    Opaque = 0,
    /// Draws all transparent parts of an entity.
    Transparent = 1,
}

pub(crate) struct ForwardEntityDrawData {
    pub(crate) pass_mode: EntityPassMode,
}

pub(crate) struct ForwardEntityDrawer {
    bindless_support: bool,
    solid_pixel_texture: Arc<Texture>,
    instance_data_buffer: Buffer<InstanceData>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    opaque_pipeline: RenderPipeline,
    transparent_pipeline: RenderPipeline,
    /// Additive variant of the transparent pipeline, used for the status
    /// effect body/weapon glow (official blend mode 5).
    additive_pipeline: RenderPipeline,
    draw_count: usize,
    /// Number of leading instances that use the regular transparent blend; the
    /// remaining instances are additive. `prepare` partitions them.
    additive_start: usize,
    instance_data: Vec<InstanceData>,
    /// Instance order built by `prepare` (non-additive first, additive last),
    /// shared by the bindless draw ranges and the texture switching of the
    /// non-bindless path.
    order: Vec<u32>,
    /// Texture of every instance in `order`, for the non-bindless path.
    ordered_textures: Vec<Arc<Texture>>,
    bump: Bump,
    lookup: HashMap<u64, i32>,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::Three }, { DepthAttachmentCount::One }> for ForwardEntityDrawer {
    type Context = ForwardRenderPassContext;
    type DrawData<'data> = ForwardEntityDrawData;

    fn new(
        capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        shader_compiler: &ShaderCompiler,
        global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = if capabilities.bindless_support() == BindlessSupport::Full {
            shader_compiler.create_shader_module("forward", "entity_bindless")
        } else {
            shader_compiler.create_shader_module("forward", "entity")
        };

        let instance_data_buffer = Buffer::with_capacity(
            device,
            format!("{DRAWER_NAME} instance data"),
            BufferUsages::COPY_DST | BufferUsages::STORAGE,
            (size_of::<InstanceData>() * INITIAL_INSTRUCTION_SIZE) as _,
        );

        let bind_group_layout = if capabilities.bindless_support() == BindlessSupport::Full {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(DRAWER_NAME),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(size_of::<InstanceData>() as _),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: NonZeroU32::new(capabilities.get_max_texture_binding_array_count()),
                    },
                ],
            })
        } else {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(DRAWER_NAME),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(size_of::<InstanceData>() as _),
                    },
                    count: None,
                }],
            })
        };

        let bind_group = if capabilities.bindless_support() == BindlessSupport::Full {
            Self::create_bind_group_bindless(device, &bind_group_layout, &instance_data_buffer, &[global_context
                .solid_pixel_texture
                .get_texture_view()])
        } else {
            Self::create_bind_group(device, &bind_group_layout, &instance_data_buffer)
        };

        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);

        let bind_group_layouts: &[Option<&BindGroupLayout>] = if capabilities.bindless_support() == BindlessSupport::Full {
            &[
                Some(pass_bind_group_layouts[0]),
                Some(pass_bind_group_layouts[1]),
                Some(&bind_group_layout),
            ]
        } else {
            &[
                Some(pass_bind_group_layouts[0]),
                Some(pass_bind_group_layouts[1]),
                Some(&bind_group_layout),
                Some(Texture::bind_group_layout(device)),
            ]
        };

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts,
            immediate_size: 0,
        });

        let color_attachment_formats = render_pass_context.color_attachment_formats();

        let opaque_pipeline = Self::create_pipeline(
            device,
            global_context,
            render_pass_context,
            &shader_module,
            &pipeline_layout,
            &color_attachment_formats,
            EntityPassMode::Opaque,
            false,
        );

        let wboit_pipeline = Self::create_pipeline(
            device,
            global_context,
            render_pass_context,
            &shader_module,
            &pipeline_layout,
            &color_attachment_formats,
            EntityPassMode::Transparent,
            false,
        );

        let additive_pipeline = Self::create_pipeline(
            device,
            global_context,
            render_pass_context,
            &shader_module,
            &pipeline_layout,
            &color_attachment_formats,
            EntityPassMode::Transparent,
            true,
        );

        Self {
            bindless_support: capabilities.bindless_support() == BindlessSupport::Full,
            solid_pixel_texture: global_context.solid_pixel_texture.clone(),
            instance_data_buffer,
            bind_group_layout,
            bind_group,
            opaque_pipeline,
            transparent_pipeline: wboit_pipeline,
            additive_pipeline,
            draw_count: 0,
            additive_start: 0,
            instance_data: Vec::default(),
            order: Vec::default(),
            ordered_textures: Vec::default(),
            bump: Bump::default(),
            lookup: HashMap::default(),
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        if self.draw_count == 0 {
            return;
        }

        pass.set_bind_group(2, &self.bind_group, &[]);

        match draw_data.pass_mode {
            EntityPassMode::Opaque => {
                // The opaque pass is unchanged: additive statuses keep their
                // regular lit body and the glow is added in the transparent
                // pass, so every instance uses the opaque pipeline here.
                pass.set_pipeline(&self.opaque_pipeline);
                self.draw_instances(pass, 0..self.draw_count);
            }
            EntityPassMode::Transparent => {
                // `prepare` put the non-additive instances first and the
                // additive ones last, so two instanced draws cover both blend
                // modes in one pass.
                if self.additive_start > 0 {
                    pass.set_pipeline(&self.transparent_pipeline);
                    self.draw_instances(pass, 0..self.additive_start);
                }
                if self.additive_start < self.draw_count {
                    pass.set_pipeline(&self.additive_pipeline);
                    self.draw_instances(pass, self.additive_start..self.draw_count);
                }
            }
        }
    }
}

impl Prepare for ForwardEntityDrawer {
    fn prepare(&mut self, device: &Device, instructions: &RenderInstruction) {
        self.draw_count = instructions.entities.len();
        self.instance_data.clear();
        self.ordered_textures.clear();
        self.order.clear();

        if self.draw_count == 0 {
            self.additive_start = 0;
            return;
        }

        // Stable partition: non-additive instances first, additive last. The
        // split index is where the transparent pass switches pipelines.
        self.additive_start = partition_additive_last(&mut self.order, instructions.entities);

        if self.bindless_support {
            self.bump.reset();
            self.lookup.clear();
            let mut texture_views = Vec::with_capacity_in(self.draw_count, &self.bump);

            for position in 0..self.order.len() {
                let instruction = &instructions.entities[self.order[position] as usize];
                let mut texture_index = texture_views.len() as i32;
                let id = instruction.texture.get_id();
                let potential_index = self.lookup.get(&id);

                if let Some(potential_index) = potential_index {
                    texture_index = *potential_index;
                } else {
                    self.lookup.insert(id, texture_index);
                    texture_views.push(instruction.texture.get_texture_view());
                }

                self.instance_data.push(InstanceData {
                    world: instruction.world.into(),
                    frame_part_transform: instruction.frame_part_transform.into(),
                    texture_position: instruction.texture_position.into(),
                    texture_size: instruction.texture_size.into(),
                    color: instruction.color.components_linear(),
                    frame_size: instruction.frame_size.into(),
                    extra_depth_offset: instruction.extra_depth_offset,
                    depth_offset: instruction.depth_offset,
                    curvature: instruction.curvature,
                    mirror: instruction.mirror as u32,
                    texture_index,
                    // The unused padding carries the additive flag to the GPU;
                    // the flag is currently consumed on the CPU by the two
                    // draw ranges, but it stays available for a future shader.
                    flags: instruction.additive as u32 | (instruction.flat as u32) << 1,
                });
            }

            if texture_views.is_empty() {
                texture_views.push(self.solid_pixel_texture.get_texture_view());
            }

            self.instance_data_buffer.reserve(device, self.instance_data.len());
            self.bind_group = Self::create_bind_group_bindless(device, &self.bind_group_layout, &self.instance_data_buffer, &texture_views)
        } else {
            self.ordered_textures.reserve(self.draw_count);

            for position in 0..self.order.len() {
                let instruction = &instructions.entities[self.order[position] as usize];

                // The non-bindless shader samples one texture through bind
                // group 3, so keep the per-instance texture in the same order
                // as `instance_data`.
                self.ordered_textures.push(instruction.texture.clone());

                self.instance_data.push(InstanceData {
                    world: instruction.world.into(),
                    frame_part_transform: instruction.frame_part_transform.into(),
                    texture_position: instruction.texture_position.into(),
                    texture_size: instruction.texture_size.into(),
                    color: instruction.color.components_linear(),
                    frame_size: instruction.frame_size.into(),
                    extra_depth_offset: instruction.extra_depth_offset,
                    depth_offset: instruction.depth_offset,
                    curvature: instruction.curvature,
                    mirror: instruction.mirror as u32,
                    texture_index: 0,
                    flags: instruction.additive as u32 | (instruction.flat as u32) << 1,
                });
            }

            self.instance_data_buffer.reserve(device, self.instance_data.len());
            self.bind_group = Self::create_bind_group(device, &self.bind_group_layout, &self.instance_data_buffer)
        }
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        self.instance_data_buffer
            .write(device, staging_belt, command_encoder, &self.instance_data);
    }
}

impl ForwardEntityDrawer {
    /// Draws one contiguous instance range from the reordered instance buffer.
    ///
    /// The bindless path needs a single instanced draw; the fallback path has
    /// one texture per instance and therefore switches the texture bind group
    /// while iterating. The instance index keeps matching the position in
    /// `instance_data`, which `prepare` filled in the same order.
    fn draw_instances(&self, pass: &mut RenderPass<'_>, range: std::ops::Range<usize>) {
        if range.is_empty() {
            return;
        }

        if self.bindless_support {
            pass.draw(0..6, range.start as u32..range.end as u32);
            return;
        }

        let mut current_texture_id = self.solid_pixel_texture.get_id();
        pass.set_bind_group(3, self.solid_pixel_texture.get_bind_group(), &[]);

        for index in range {
            let texture = &self.ordered_textures[index];
            if texture.get_id() != current_texture_id {
                current_texture_id = texture.get_id();
                pass.set_bind_group(3, texture.get_bind_group(), &[]);
            }
            let instance = index as u32;
            pass.draw(0..6, instance..instance + 1);
        }
    }

    fn create_bind_group_bindless(
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        instance_data_buffer: &Buffer<InstanceData>,
        texture_views: &[&TextureView],
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(DRAWER_NAME),
            layout: bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: instance_data_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureViewArray(texture_views),
                },
            ],
        })
    }

    fn create_bind_group(device: &Device, bind_group_layout: &BindGroupLayout, instance_data_buffer: &Buffer<InstanceData>) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(DRAWER_NAME),
            layout: bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: instance_data_buffer.as_entire_binding(),
            }],
        })
    }

    #[allow(clippy::too_many_arguments)] // pipeline descriptors are positional by nature
    fn create_pipeline(
        device: &Device,
        global_context: &GlobalContext,
        render_pass_context: &ForwardRenderPassContext,
        shader_module: &ShaderModule,
        pipeline_layout: &PipelineLayout,
        color_attachment_formats: &[TextureFormat; 3],
        pass_mode: EntityPassMode,
        additive: bool,
    ) -> RenderPipeline {
        // Additive blend state for RO's blend mode 5 (the official
        // `EF_REDBODY`/`EF_AURABLADE2` body/weapon glow). The transparent pass
        // draws additive instances into the forward colour target so the glow
        // is really added to the lit scene instead of going through the
        // order-independent WBOIT accumulation. `SrcAlpha / One` scales the
        // added colour by the sprite alpha, which keeps the soft cut-out edges
        // from gaining a hard rim; the destination alpha is left untouched.
        let empty_target = |format| ColorTargetState {
            format,
            blend: Some(BlendState {
                color: BlendComponent::default(),
                alpha: BlendComponent::default(),
            }),
            write_mask: ColorWrites::empty(),
        };

        let targets = match (pass_mode, additive) {
            (EntityPassMode::Opaque, _) => [
                Some(ColorTargetState {
                    format: color_attachment_formats[0],
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                }),
                Some(empty_target(color_attachment_formats[1])),
                Some(empty_target(color_attachment_formats[2])),
            ],
            (EntityPassMode::Transparent, false) => [
                Some(ColorTargetState {
                    format: color_attachment_formats[0],
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::empty(),
                }),
                Some(ColorTargetState {
                    format: color_attachment_formats[1],
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                }),
                Some(ColorTargetState {
                    format: color_attachment_formats[2],
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::OneMinusSrc,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent::default(),
                    }),
                    write_mask: ColorWrites::RED,
                }),
            ],
            (EntityPassMode::Transparent, true) => [
                Some(ColorTargetState {
                    format: color_attachment_formats[0],
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                }),
                Some(empty_target(color_attachment_formats[1])),
                Some(empty_target(color_attachment_formats[2])),
            ],
        };

        // The additive pipeline reuses `opaque_main` (`PASS_MODE == 0`): it
        // keeps the opaque pixels of the sprite, which is the body/weapon
        // surface that the official blend mode 5 turns into a glow. The
        // `GreaterEqual` depth test lets it re-cover the pixels the opaque pass
        // already wrote (same geometry, same depth) while still being occluded
        // by geometry in front; depth writes stay disabled.
        let pass_mode_constant = match additive {
            true => 0,
            false => pass_mode as u32,
        };

        let constants = &[
            // PASS_MODE
            ("0", f64::from(pass_mode_constant)),
        ];

        let entry_point = match (pass_mode, additive) {
            (EntityPassMode::Opaque, _) | (EntityPassMode::Transparent, true) => "opaque_main",
            (EntityPassMode::Transparent, false) => "transparent_main",
        };

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(&format!(
                "{DRAWER_NAME} {pass_mode:?}{}",
                if additive { " additive" } else { "" }
            )),
            layout: Some(pipeline_layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants,
                    zero_initialize_workgroup_memory: false,
                },
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: Some(entry_point),
                compilation_options: PipelineCompilationOptions {
                    constants,
                    zero_initialize_workgroup_memory: false,
                },
                targets: &targets,
            }),
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                front_face: FrontFace::Ccw,
                ..Default::default()
            },
            multisample: MultisampleState {
                count: global_context.msaa.sample_count(),
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: render_pass_context.depth_attachment_output_format()[0],
                depth_write_enabled: Some(pass_mode == EntityPassMode::Opaque),
                depth_compare: Some(match additive {
                    true => CompareFunction::GreaterEqual,
                    false => CompareFunction::Greater,
                }),
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            cache: None,
            multiview_mask: None,
        })
    }
}

/// Stable partition of instance indices: non-additive first, additive last.
/// Returns the split index, i.e. the first additive instance. `order` is
/// rebuilt in place so the per-frame path does not allocate.
fn partition_additive_last<T>(order: &mut Vec<u32>, items: &[T]) -> usize
where
    T: HasAdditive,
{
    order.clear();
    order.extend((0..items.len() as u32).filter(|&index| !items[index as usize].additive()));
    let split = order.len();
    order.extend((0..items.len() as u32).filter(|&index| items[index as usize].additive()));
    split
}

/// Minimal view of an entity instruction needed by the partition helper, so
/// the ordering rule is unit-testable without constructing GPU resources.
trait HasAdditive {
    fn additive(&self) -> bool;
}

impl HasAdditive for EntityInstruction {
    fn additive(&self) -> bool {
        self.additive
    }
}

#[cfg(test)]
mod tests {
    use super::partition_additive_last;

    impl super::HasAdditive for bool {
        fn additive(&self) -> bool {
            *self
        }
    }

    #[test]
    fn additive_instances_are_partitioned_to_the_end() {
        // A frame mixes regular body parts with the glowing ones; the relative
        // order inside each group must be preserved (stable partition).
        let flags = [false, true, false, false, true, false];
        let mut order = Vec::new();
        let split = partition_additive_last(&mut order, &flags);

        assert_eq!(split, 4, "four non-additive instances come first");
        assert_eq!(order, vec![0, 2, 3, 5, 1, 4]);
        assert!(order[..split].iter().all(|&index| !flags[index as usize]));
        assert!(order[split..].iter().all(|&index| flags[index as usize]));
    }

    #[test]
    fn all_and_no_additive_instances_are_handled() {
        let mut order = Vec::new();

        let none = [false, false, false];
        assert_eq!(partition_additive_last(&mut order, &none), 3);
        assert_eq!(order, vec![0, 1, 2]);

        let all = [true, true];
        assert_eq!(partition_additive_last(&mut order, &all), 0);
        assert_eq!(order, vec![0, 1]);

        let empty: [bool; 0] = [];
        assert_eq!(partition_additive_last(&mut order, &empty), 0);
        assert!(order.is_empty());
    }
}
