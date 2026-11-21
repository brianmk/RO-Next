// SMAA 1x pass 1: luma edge detection. See modules/smaa_common.hlsl for the
// full copyright notice of the ported reference implementation.

[[vk::binding(0, 1)]] Texture2D color_tex;
[[vk::binding(1, 1)]] SamplerState nearest_sampler;
[[vk::binding(2, 1)]] SamplerState linear_sampler;

#include "../../modules/smaa_common.hlsl"

[[shader("vertex")]]
SmaaVertex vs_main(uint vertex_index : SV_VulkanVertexID) {
    return smaa_vertex(vertex_index);
}

[[shader("pixel")]]
float4 fs_main(SmaaVertex input) : SV_Target {
    float2 dimensions;
    color_tex.GetDimensions(dimensions.x, dimensions.y);
    g_smaa_rt_metrics = float4(1.0 / dimensions, dimensions);

    float4 offset[3];
    offset[0] = mad(SMAA_RT_METRICS.xyxy, float4(-1.0, 0.0, 0.0, -1.0), input.uv.xyxy);
    offset[1] = mad(SMAA_RT_METRICS.xyxy, float4(1.0, 0.0, 0.0, 1.0), input.uv.xyxy);
    offset[2] = mad(SMAA_RT_METRICS.xyxy, float4(-2.0, 0.0, 0.0, -2.0), input.uv.xyxy);

    float2 edges = SMAALumaEdgeDetectionPS(input.uv, offset, color_tex);

    // Write zero instead of discarding, the edge texture is not cleared.
    return float4(edges, 0.0, 1.0);
}
