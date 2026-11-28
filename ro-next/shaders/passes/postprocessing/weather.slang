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
    // A slow breeze plus occasional sharp gusts.
    let breeze = 0.5 + 0.5 * sin(time * 0.13 + 0.4);
    let gust = pow(max(0.0, sin(time * 0.37 + 1.7)), 8.0);
    return 0.10 + 0.16 * breeze + 0.55 * gust;
}

func gustAt(uv: float2, time: float) -> float2 {
    // A travelling gust cell. Only particles inside its footprint react, so
    // the whole screen doesn't lurch sideways at the same time.
    let center = float2(0.5 + 0.35 * sin(time * 0.13 + 0.7), 0.45 + 0.28 * cos(time * 0.10 + 1.9));
    let delta = (uv - center) * float2(1.0, 1.25);
    let influence = exp(-dot(delta, delta) * 12.0);
    let pulse = pow(max(0.0, sin(time * 0.41 + 0.3)), 6.0);
    let direction = normalize(float2(cos(time * 0.17), 0.22 + 0.22 * sin(time * 0.11)));
    return direction * influence * pulse * 0.10;
}

func particleLayer(
    uv: float2,
    aspect: float,
    density: float,
    speed: float,
    seed: float,
    shape: uint,
    tint: float3,
    time: float,
    depth: float,
    direction: float2
) -> float4 {
    let wind = windStrength(time);
    // Near layers drift faster than far layers, and camera movement shifts
    // them more, which gives the flat screen-space particles parallax depth.
    let parallax = global_uniforms.camera_position.xz * (-0.0018 * depth);
    var p = float2(uv.x * aspect, uv.y) + float2(parallax.x, parallax.y * 0.35);
    p += direction * time * (0.020 + speed * 0.55);
    p.y += time * speed;
    let cell = floor(p * density);
    let local_cell = frac(p * density);
    let random = hash21(cell + float2(seed, seed * 1.7));
    let center = float2(0.16 + 0.68 * hash21(cell + float2(1.0, seed)), 0.16 + 0.68 * hash21(cell + float2(seed, 2.0)));
    var local = local_cell - center;
    // Sway and local gust displacement. The gust uses the particle's screen
    // position, so it moves one patch of petals/flakes rather than the field.
    let gust = gustAt(uv + float2(seed * 0.07, seed * 0.04), time);
    local += gust * (0.35 + depth);
    local.x += sin(time * (1.2 + random * 0.9) + random * TAU) * wind * 0.055;

    // 3D tumble: a flat petal/flake is projected through roll (screen-plane
    // spin), yaw (around the vertical axis) and pitch (around the horizontal
    // axis). Dividing by the projected axis lengths makes the particle turn
    // edge-on and open up again, instead of only spinning like a sticker.
    let spin_roll = random * TAU + time * (0.5 + random * 1.1);
    let spin_yaw = random * TAU + time * (0.35 + random * 1.35);
    let spin_pitch = random * TAU + time * (0.28 + random * 1.55);
    local = rotate2(local, spin_roll);
    let yaw_cos = max(abs(cos(spin_yaw)), 0.18);
    let pitch_cos = max(abs(cos(spin_pitch)), 0.18);
    local = float2(local.x / yaw_cos, local.y / pitch_cos);

    let scale = (0.55 + 0.35 * hash21(cell + float2(3.0, seed))) * (0.65 + depth * 0.65);
    var distance = 0.0;
    if (shape == 0u) {
        distance = sakuraShape(local / scale);
    } else if (shape == 1u) {
        distance = mapleShape(local / scale);
    } else {
        distance = snowShape(local / scale);
    }

    // Depth shading: the near face is a little brighter than the edge.
    let face = smoothstep(0.0, -0.055, distance);
    let shaded = tint * (0.72 + 0.28 * face);
    let alpha = smoothstep(0.012, -0.012, distance) * (0.38 + 0.28 * random) * (0.55 + depth * 0.55);
    return float4(shaded, alpha);
}

func weatherParticles(uv: float2, kind: uint, time: float, intensity: float) -> float4 {
    let aspect = max(1.0, float(global_uniforms.forward_size.x) / max(1.0, float(global_uniforms.forward_size.y)));
    var result = float4(0.0);

    if (kind == 1u) {
        // Three parallax layers: far, middle, near.
        result += particleLayer(uv, aspect, 2.4, 0.055, 1.0, 2u, float3(0.82, 0.90, 1.0), time, 0.30, float2(-0.45, 1.0)) * 0.55;
        result += particleLayer(uv + float2(0.31, 0.17), aspect, 3.0, 0.085, 4.0, 2u, float3(0.92, 0.96, 1.0), time, 0.62, float2(0.30, 1.0)) * 0.78;
        result += particleLayer(uv + float2(0.64, 0.38), aspect, 2.2, 0.12, 9.0, 2u, float3(1.0, 1.0, 1.0), time, 1.0, float2(-0.12, 1.0)) * 0.95;
    } else if (kind == 2u) {
        result += particleLayer(uv, aspect, 2.2, 0.035, 2.0, 0u, float3(1.0, 0.72, 0.82), time, 0.30, float2(-0.55, 1.0)) * 0.50;
        result += particleLayer(uv + float2(0.43, 0.21), aspect, 3.0, 0.050, 7.0, 0u, float3(1.0, 0.58, 0.74), time, 0.62, float2(0.38, 1.0)) * 0.72;
        result += particleLayer(uv + float2(0.72, 0.44), aspect, 2.0, 0.070, 12.0, 0u, float3(1.0, 0.86, 0.90), time, 1.0, float2(-0.18, 1.0)) * 0.95;
    } else if (kind == 3u) {
        result += particleLayer(uv, aspect, 1.8, 0.022, 3.0, 1u, float3(0.78, 0.34, 0.12), time, 0.30, float2(-0.50, 1.0)) * 0.55;
        result += particleLayer(uv + float2(0.37, 0.41), aspect, 2.6, 0.032, 8.0, 1u, float3(0.92, 0.48, 0.16), time, 0.62, float2(0.32, 1.0)) * 0.78;
        result += particleLayer(uv + float2(0.66, 0.23), aspect, 1.9, 0.045, 13.0, 1u, float3(1.0, 0.62, 0.22), time, 1.0, float2(-0.15, 1.0)) * 0.95;
    }

    result.a = min(1.0, result.a) * intensity;
    return result;
}

// One layer of drifting noise. The low-frequency domain warp breaks up the
// regular FBM lattice and gives the layer a soft, rolling edge instead of a
// static grain.
func cloudLayer(p: float2, camera_offset: float2, time: float, scale: float, direction: float2, speed: float, parallax: float, color: float3, opacity: float) -> float4 {
    let base = p * scale + camera_offset * parallax + direction * time * speed;
    let warp = float2(
        fbm(base * 0.75 + float2(1.7, 9.2) + direction.yx * time * speed * 0.6),
        fbm(base * 0.75 + float2(8.3, 2.8) - direction * time * speed * 0.5)
    );
    let n = fbm(base + (warp - 0.5) * 1.35);
    let density = smoothstep(0.34, 0.78, n) * opacity;
    return float4(color, density);
}

func weatherSheets(uv: float2, kind: uint, time: float, intensity: float) -> float4 {
    let aspect = max(1.0, float(global_uniforms.forward_size.x) / max(1.0, float(global_uniforms.forward_size.y)));
    let p = float2(uv.x * aspect, uv.y);
    // World anchoring: use Z directly so walking north makes the fog appear
    // to slide south (the opposite screen direction), not follow the player.
    let camera_offset = float2(global_uniforms.camera_position.x, global_uniforms.camera_position.z);

    if (kind == 4u) {
        let far = cloudLayer(p, camera_offset, time, 1.7, float2(-0.4, 0.9), 0.45, 0.008, float3(0.72, 0.80, 0.94), 0.10 * intensity);
        let near = cloudLayer(p, camera_offset, time, 3.4, float2(0.7, 0.5), 0.80, 0.016, float3(0.84, 0.88, 1.0), 0.12 * intensity);
        let alpha = far.a + near.a * (1.0 - far.a);
        let color = (far.rgb * far.a + near.rgb * near.a) / max(alpha, 0.0001);
        return float4(color, alpha);
    }

    if (kind == 5u) {
        // Three independent fog layers: broad background, rolling middle and
        // fast low wisps. Each has a different direction and parallax factor.
        let far = cloudLayer(p, camera_offset, time, 1.3, float2(-0.35, 0.9), 0.28, 0.010, float3(0.52, 0.61, 0.74), 0.13 * intensity);
        let middle = cloudLayer(p, camera_offset, time, 2.4, float2(0.75, 0.45), 0.48, 0.024, float3(0.64, 0.72, 0.84), 0.12 * intensity);
        let near = cloudLayer(p, camera_offset, time, 4.3, float2(-0.8, 0.2), 0.78, 0.045, float3(0.76, 0.82, 0.92), 0.10 * intensity);

        let alpha = far.a + middle.a * (1.0 - far.a) + near.a * (1.0 - far.a) * (1.0 - middle.a);
        let color = (far.rgb * far.a + middle.rgb * middle.a + near.rgb * near.a) / max(alpha, 0.0001);
        return float4(color, alpha);
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
