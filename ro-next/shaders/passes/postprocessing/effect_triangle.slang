#language slang 2026

// Additive triangle-list path for the official client's slot/record effect
// primitives (rings, cylinders, columns, fans). The client submits those as
// `D3DPT_TRIANGLELIST` triangles whose three corners are projected
// individually, so unlike `effect.slang` (which expands one quad instance)
// every vertex arrives pre-projected: NDC position plus its own reverse-z
// depth. One instance is one triangle and `vertex_index` selects its corner,
// which keeps the pipeline free of vertex buffers and lets it share the
// effect pass state of the quad drawer.

import postprocessing;

public struct EffectTriangleVertexOutput {
    public float4 position : SV_Position;
    [[vk::location(0)]] public var texture_coordinates: float2;
    [[vk::location(1)]] public var color: float4;
}

[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(0, 1)]] var triangle_instance_data: StructuredBuffer<EffectTriangleInstanceData>;
[[vk::binding(0, 2)]] var texture: Texture2D;

[[shader("vertex")]]
func vs_main(input: PostprocessingVertexInput) -> EffectTriangleVertexOutput {
    let instance = triangle_instance_data[input.instance_index];

    var position: float2;
    var depth: float;
    var texture_coordinate: float2;

    switch (input.vertex_index) {
        case 0:
            position = instance.position0;
            depth = instance.depth0.x;
            texture_coordinate = instance.texture0;
            break;
        case 1:
            position = instance.position1;
            depth = instance.depth0.y;
            texture_coordinate = instance.texture1;
            break;
        default:
            position = instance.position2;
            depth = instance.depth1.x;
            texture_coordinate = instance.texture2;
            break;
    }

    var output: EffectTriangleVertexOutput;
    // The xy are already NDC and the z is the NDC depth (reverse-z) of the
    // world position the official `Project` computed for this corner.
    output.position = float4(position, depth, 1.0);
    output.texture_coordinates = texture_coordinate;
    output.color = float4(instance.color0, instance.color1);
    return output;
}

[[shader("pixel")]]
func fs_main(input: EffectTriangleVertexOutput) -> float4 {
    let color = texture.Sample(linear_sampler, input.texture_coordinates) * input.color;

    // Same premultiplied-alpha rule as `effect.slang`: a fully transparent
    // texel is discarded so opaque / destination-alpha blend modes do not
    // write the keyed-out background as black.
    if (color.a <= 0.0001) {
        discard;
    }

    return color;
}
