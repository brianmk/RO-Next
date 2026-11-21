#language slang 2026

import postprocessing;

public struct EffectVertexOutput {
    public float4 position : SV_Position;
    [[vk::location(0)]] public var texture_coordinates: float2;
    [[vk::location(1)]] public var color: float4;
    [[vk::location(2)]] public var texture_index: int;
}

[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(0, 1)]] var instance_data: StructuredBuffer<EffectInstanceData>;
[[vk::binding(1, 1)]] var textures: Texture2D[];

[[shader("vertex")]]
func vs_main(input: PostprocessingVertexInput) -> EffectVertexOutput {
    let instance = instance_data[input.instance_index];

    let data = effect_vertex_data(instance, input.vertex_index);

    var output: EffectVertexOutput;
    // The z value is the NDC depth (reverse-z) of the corresponding world position. The
    // effects are rendered in a pass with the scene depth buffer attached, which makes map
    // geometry and entities in front of them occlude them.
    output.position = float4(data.position, data.depth, 1.0);
    output.texture_coordinates = data.texture_coordinate;
    output.color = float4(instance.color0, instance.color1);
    output.texture_index = instance.texture_index;
    return output;
}

[[shader("pixel")]]
func fs_main(input: EffectVertexOutput) -> float4 {
    return textures[input.texture_index].Sample(linear_sampler, input.texture_coordinates) * input.color;
}
