// SMAA 1x pass 3: neighborhood blending. See modules/smaa_common.hlsl for the
// full copyright notice of the ported reference implementation.

[[vk::binding(0, 1)]] Texture2D color_tex;
[[vk::binding(1, 1)]] Texture2D blend_tex;
[[vk::binding(2, 1)]] SamplerState nearest_sampler;
[[vk::binding(3, 1)]] SamplerState linear_sampler;

#include "../../modules/smaa_common.hlsl"

[[shader("vertex")]]
SmaaVertex vs_main(uint vertex_index : SV_VulkanVertexID) {
    return smaa_vertex(vertex_index);
}

[[shader("pixel")]]
float4 fs_main(SmaaVertex input) : SV_Target {
    float2 dimensions;
    blend_tex.GetDimensions(dimensions.x, dimensions.y);
    g_smaa_rt_metrics = float4(1.0 / dimensions, dimensions);

    float4 offset = mad(SMAA_RT_METRICS.xyxy, float4(1.0, 0.0, 0.0, 1.0), input.uv.xyxy);

    return SMAANeighborhoodBlendingPS(input.uv, offset, color_tex, blend_tex);
}
