use wgpu::{
    BindGroup, ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, TextureFormat, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, PostProcessingRenderPassContext, RenderPassContext,
};
use crate::graphics::shader_compiler::ShaderCompiler;
use crate::graphics::{
    Capabilities, GlobalContext, SMAA_BLEND_WEIGHTS_TEXTURE_FORMAT, SMAA_EDGE_TEXTURE_FORMAT, smaa_blend_bind_group_layout,
    smaa_edge_bind_group_layout, smaa_weights_bind_group_layout,
};

const EDGE_DRAWER_NAME: &str = "post processing smaa edge";
const BLEND_WEIGHTS_DRAWER_NAME: &str = "post processing smaa blend weights";
const BLEND_DRAWER_NAME: &str = "post processing smaa blend";

fn create_smaa_pipeline(
    device: &Device,
    shader_compiler: &ShaderCompiler,
    shader_name: &str,
    label: &str,
    bind_group_layout: &wgpu::BindGroupLayout,
    pass_bind_group_layout: &wgpu::BindGroupLayout,
    color_format: TextureFormat,
) -> RenderPipeline {
    let shader_module = shader_compiler.create_shader_module("postprocessing", shader_name);

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[Some(pass_bind_group_layout), Some(bind_group_layout)],
        immediate_size: 0,
    });

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader_module,
            entry_point: Some("vs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            buffers: &[],
        },
        fragment: Some(FragmentState {
            module: &shader_module,
            entry_point: Some("fs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            targets: &[Some(ColorTargetState {
                format: color_format,
                blend: None,
                write_mask: ColorWrites::default(),
            })],
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        cache: None,
        multiview_mask: None,
    })
}

/// First SMAA pass: luma edge detection. Writes the edges into an RG8 texture.
pub(crate) struct PostProcessingSmaaEdgeDrawer {
    pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for PostProcessingSmaaEdgeDrawer {
    type Context = PostProcessingRenderPassContext;
    type DrawData<'data> = &'data BindGroup;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        shader_compiler: &ShaderCompiler,
        _global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);

        let pipeline = create_smaa_pipeline(
            device,
            shader_compiler,
            "smaa_edge",
            EDGE_DRAWER_NAME,
            smaa_edge_bind_group_layout(device),
            pass_bind_group_layouts[0],
            SMAA_EDGE_TEXTURE_FORMAT,
        );

        let _ = render_pass_context;

        Self { pipeline }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(1, draw_data, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// Second SMAA pass: blending weight calculation. Writes the weights into an
/// RGBA texture.
pub(crate) struct PostProcessingSmaaBlendWeightsDrawer {
    pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }>
    for PostProcessingSmaaBlendWeightsDrawer
{
    type Context = PostProcessingRenderPassContext;
    type DrawData<'data> = &'data BindGroup;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        shader_compiler: &ShaderCompiler,
        _global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);

        let pipeline = create_smaa_pipeline(
            device,
            shader_compiler,
            "smaa_weights",
            BLEND_WEIGHTS_DRAWER_NAME,
            smaa_weights_bind_group_layout(device),
            pass_bind_group_layouts[0],
            SMAA_BLEND_WEIGHTS_TEXTURE_FORMAT,
        );

        let _ = render_pass_context;

        Self { pipeline }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(1, draw_data, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// Third SMAA pass: neighborhood blending. Writes the antialiased color into
/// the color texture.
pub(crate) struct PostProcessingSmaaBlendDrawer {
    pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for PostProcessingSmaaBlendDrawer {
    type Context = PostProcessingRenderPassContext;
    type DrawData<'data> = &'data BindGroup;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        shader_compiler: &ShaderCompiler,
        _global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);

        let pipeline = create_smaa_pipeline(
            device,
            shader_compiler,
            "smaa_blend",
            BLEND_DRAWER_NAME,
            smaa_blend_bind_group_layout(device),
            pass_bind_group_layouts[0],
            render_pass_context.color_attachment_formats()[0],
        );

        Self { pipeline }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(1, draw_data, &[]);
        pass.draw(0..3, 0..1);
    }
}
