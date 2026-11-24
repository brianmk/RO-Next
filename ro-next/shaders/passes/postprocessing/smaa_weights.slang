// SMAA 1x pass 2: blending weight calculation. See modules/smaa_common.hlsl
// for the full copyright notice of the ported reference implementation.

// The full SMAA 1x shader (with diagonal pattern searches and corner rounding)
// crashes the NVIDIA Vulkan shader compiler on driver 620.70.06, so this pass
// is compiled with the "medium" preset (no diagonal detection, no corner
// rounding). See notes/ro-next-dlss-aa-report.md.
#define SMAA_DISABLE_DIAG_DETECTION
#define SMAA_DISABLE_CORNER_DETECTION

[[vk::binding(0, 1)]] Texture2D edges_tex;
[[vk::binding(1, 1)]] Texture2D area_tex;
[[vk::binding(2, 1)]] Texture2D search_tex;
[[vk::binding(3, 1)]] SamplerState nearest_sampler;
[[vk::binding(4, 1)]] SamplerState linear_sampler;

#include "../../modules/smaa_common.hlsl"

[[shader("vertex")]]
SmaaVertex vs_main(uint vertex_index : SV_VulkanVertexID) {
    return smaa_vertex(vertex_index);
}

[[shader("pixel")]]
float4 fs_main(SmaaVertex input) : SV_Target {
    float2 dimensions;
    edges_tex.GetDimensions(dimensions.x, dimensions.y);
    g_smaa_rt_metrics = float4(1.0 / dimensions, dimensions);

    float2 texcoord = input.uv;
    float2 pixcoord = texcoord * SMAA_RT_METRICS.zw;

    float4 offset[3];
    // We will use these offsets for the searches later on (see @PSEUDO_GATHER4):
    offset[0] = mad(SMAA_RT_METRICS.xyxy, float4(-0.25, -0.125, 1.25, -0.125), texcoord.xyxy);
    offset[1] = mad(SMAA_RT_METRICS.xyxy, float4(-0.125, -0.25, -0.125, 1.25), texcoord.xyxy);

    // And these for the searches, they indicate the ends of the loops:
    offset[2] = mad(
        SMAA_RT_METRICS.xxyy,
        float4(-2.0, 2.0, -2.0, 2.0) * float(SMAA_MAX_SEARCH_STEPS),
        float4(offset[0].xz, offset[1].yw)
    );

    float4 weights = SMAABlendingWeightCalculationPS(texcoord, pixcoord, offset, edges_tex, area_tex, search_tex, float4(0.0, 0.0, 0.0, 0.0));

    return weights;
}
