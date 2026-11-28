#language slang 2026

import globals;
import screen_space;

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;

static const var PI: float = 3.14159265;
static const var TAU: float = 6.2831853;

func hash21(value: float2) -> float {
    var p = frac(float3(value.x, value.y, value.x) * float3(0.1031, 0.1030, 0.0973));
    p += dot(p, float3(p.y, p.z, p.x) + 33.33);
    return frac((p.x + p.y) * p.z);
}

func rotate2(value: float2, angle: float) -> float2 {
    let s = sin(angle);
    let c = cos(angle);
    return float2(value.x * c - value.y * s, value.x * s + value.y * c);
}

func sdEllipse(value: float2, radius: float2) -> float {
    let k = length(value / radius);
    return (k - 1.0) * min(radius.x, radius.y);
}

func sakuraShape(value: float2) -> float {
    // A single cherry-blossom petal with the characteristic notch at the tip.
    let petal = sdEllipse(value, float2(0.085, 0.038));
    let notch = sdEllipse(value - float2(0.0, 0.060), float2(0.025, 0.032));
    return max(petal, -notch);
}

func mapleShape(value: float2) -> float {
    let angle = atan2(value.y, value.x);
    let radius = 0.055 + 0.045 * abs(cos(angle * 2.5)) + 0.018 * abs(cos(angle * 5.0));
    return length(value) - radius;
}

func snowShape(value: float2) -> float {
    var distance = length(value) - 0.085;
    for (var i = 0; i < 3; i++) {
        let angle = float(i) * PI / 3.0;
        let direction = float2(cos(angle), sin(angle));
        let along = dot(value, direction);
        let across = abs(dot(value, float2(-direction.y, direction.x)));
        distance = min(distance, max(abs(along) - 0.13, across - 0.018));
    }
    return distance;
}

func noise2(value: float2) -> float {
    let i = floor(value);
    let f = frac(value);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + float2(1.0, 0.0));
    let c = hash21(i + float2(0.0, 1.0));
    let d = hash21(i + float2(1.0, 1.0));
    return lerp(lerp(a, b, u.x), lerp(c, d, u.x), u.y);
}

func fbm(value: float2) -> float {
    var total = 0.0;
    var amplitude = 0.5;
    var p = value;
    for (var i = 0; i < 5; i++) {
        total += noise2(p) * amplitude;
        p = p * 2.03 + float2(17.0, 9.0);
        amplitude *= 0.5;
    }
    return total;
}

func windStrength(time: float) -> float {
    // A slow breeze plus occasional sharp gusts. The gust frequency and
    // phase are intentionally incommensurate so the pattern doesn't repeat
    // visibly during a play session.
    let breeze = 0.5 + 0.5 * sin(time * 0.13 + 0.4);
    let gust = pow(max(0.0, sin(time * 0.37 + 1.7)), 8.0);
    return 0.10 + 0.16 * breeze + 0.55 * gust;
}

func particleLayer(uv: float2, aspect: float, density: float, speed: float, seed: float, shape: uint, tint: float3, time: float) -> float4 {
    let wind = windStrength(time);
    var p = float2(uv.x * aspect, uv.y);
    p.x += time * 0.022 + wind * 0.28;
    p.y += time * speed;
    let cell = floor(p * density);
    let local_cell = frac(p * density);
    let random = hash21(cell + float2(seed, seed * 1.7));
    let center = float2(0.16 + 0.68 * hash21(cell + float2(1.0, seed)), 0.16 + 0.68 * hash21(cell + float2(seed, 2.0)));
    var local = local_cell - center;
    // Horizontal sway grows with the current gust strength.
    local.x += sin(time * (1.2 + random * 0.9) + random * TAU) * wind * 0.075;
    let rotation = random * TAU + time * (0.35 + random * 0.55);
    local = rotate2(local, rotation);
    let scale = 0.75 + 0.35 * hash21(cell + float2(3.0, seed));
    var distance = 0.0;
    if (shape == 0u) {
        distance = sakuraShape(local / scale);
    } else if (shape == 1u) {
        distance = mapleShape(local / scale);
    } else {
        distance = snowShape(local / scale);
    }
    let alpha = smoothstep(0.012, -0.012, distance) * (0.45 + 0.35 * random);
    return float4(tint, alpha);
}

func weatherParticles(uv: float2, kind: uint, time: float, intensity: float) -> float4 {
    let aspect = max(1.0, float(global_uniforms.forward_size.x) / max(1.0, float(global_uniforms.forward_size.y)));
    var result = float4(0.0);

    if (kind == 1u) {
        result += particleLayer(uv, aspect, 5.5, 0.075, 1.0, 2u, float3(0.92, 0.96, 1.0), time);
        result += particleLayer(uv + float2(0.31, 0.17), aspect, 4.2, 0.055, 4.0, 2u, float3(0.82, 0.90, 1.0), time) * 0.55;
    } else if (kind == 2u) {
        result += particleLayer(uv, aspect, 5.5, 0.045, 2.0, 0u, float3(1.0, 0.58, 0.74), time) * 0.75;
        result += particleLayer(uv + float2(0.43, 0.21), aspect, 4.2, 0.035, 7.0, 0u, float3(1.0, 0.76, 0.86), time) * 0.45;
    } else if (kind == 3u) {
        result += particleLayer(uv, aspect, 4.0, 0.030, 3.0, 1u, float3(1.0, 0.56, 0.20), time) * 0.8;
        result += particleLayer(uv + float2(0.37, 0.41), aspect, 3.2, 0.024, 8.0, 1u, float3(0.90, 0.38, 0.12), time) * 0.5;
    }

    result.a = min(1.0, result.a) * intensity;
    return result;
}

func weatherSheets(uv: float2, kind: uint, time: float, intensity: float) -> float4 {
    let aspect = max(1.0, float(global_uniforms.forward_size.x) / max(1.0, float(global_uniforms.forward_size.y)));
    let p = float2(uv.x * aspect, uv.y);
    // Anchor the sheets to the world, not the camera: the same patch of fog
    // stays in front of the player as they walk through it. The factor is in
    // noise cells per world unit, so it is independent of resolution.
    let camera_offset = global_uniforms.camera_position.xz;

    if (kind == 4u) {
        let n = fbm(p * float2(3.0, 2.2) + camera_offset * 0.012 + float2(time * 0.006, 0.0));
        let cloud = smoothstep(0.48, 0.78, n);
        return float4(float3(0.78, 0.84, 0.95), cloud * 0.16 * intensity);
    }
    if (kind == 5u) {
        let n = fbm(p * float2(2.0, 1.7) + camera_offset * 0.035 + float2(time * 0.008, time * 0.003));
        let mist = smoothstep(0.30, 0.78, n);
        return float4(float3(0.62, 0.70, 0.82), mist * 0.20 * intensity);
    }
    return float4(0.0);
}

[[shader("vertex")]]
func vs_main(uint vertex_index : SV_VulkanVertexID) -> FullscreenVertex {
    return FullscreenVertex::new(vertex_index);
}

[[shader("pixel")]]
func fs_main(input: FullscreenVertex) -> float4 {
    let kind = uint(global_uniforms.weather.x + 0.5);
    if (kind == 0u) {
        return float4(0.0);
    }
    let time = global_uniforms.weather.y;
    let intensity = global_uniforms.weather.z;
    let particles = weatherParticles(input.uv, kind, time, intensity);
    let sheets = weatherSheets(input.uv, kind, time, intensity);
    let color = particles.rgb * particles.a + sheets.rgb * sheets.a;
    let alpha = particles.a + sheets.a * (1.0 - particles.a);
    return float4(color, alpha);
}
