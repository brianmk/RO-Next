// SMAA 1x: Subpixel Morphological Antialiasing
//
// Ported from the reference implementation:
//   https://github.com/iryoku/smaa
//   Copyright (C) 2013 Jorge Jimenez (jorge@iryoku.com)
//   Copyright (C) 2013 Jose I. Echevarria (joseignacioechevarria@gmail.com)
//   Copyright (C) 2013 Belen Masia (bmasia@unizar.es)
//   Copyright (C) 2013 Fernando Navarro (fernandn@microsoft.com)
//   Copyright (C) 2013 Diego Gutierrez (diegog@unizar.es)
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software. As clarification, there
// is no requirement that the copyright notice and permission be included in
// binary distributions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
//
// This header only contains SMAA 1x (no separate samples, temporal
// reprojection, predication or velocity buffer support). The three passes map
// to smaa_edge.slang, smaa_weights.slang and smaa_blend.slang. The area and
// search lookup textures are the unmodified reference textures (RG8 160x560
// and R8 64x16) as extracted from the SMAA repository.
//
// Each pass declares its own group-1 textures and two clamped samplers before
// including this header:
//
//   nearest_sampler  point sampling, clamp to edge
//   linear_sampler   linear sampling, clamp to edge
//
// and assigns `g_smaa_rt_metrics = float4(1.0 / screen_size, screen_size)`
// in its fragment entry point before using any of the functions below.

// SMAA quality preset "high": 16 search steps, diagonal detection and corner
// rounding enabled.
#define SMAA_THRESHOLD 0.1
#define SMAA_LOCAL_CONTRAST_ADAPTATION_FACTOR 2.0
#define SMAA_MAX_SEARCH_STEPS 16
#define SMAA_MAX_SEARCH_STEPS_DIAG 8
#define SMAA_CORNER_ROUNDING 25

#define SMAA_AREATEX_MAX_DISTANCE 16
#define SMAA_AREATEX_MAX_DISTANCE_DIAG 20
#define SMAA_AREATEX_PIXEL_SIZE (1.0 / float2(160.0, 560.0))
#define SMAA_AREATEX_SUBTEX_SIZE (1.0 / 7.0)
#define SMAA_SEARCHTEX_SIZE float2(66.0, 33.0)
#define SMAA_SEARCHTEX_PACKED_SIZE float2(64.0, 16.0)
#define SMAA_CORNER_ROUNDING_NORM (float(SMAA_CORNER_ROUNDING) / 100.0)

#define SMAA_FLATTEN
#define SMAA_BRANCH

// Shared screen space metrics, assigned in each fragment shader entry point.
static float4 g_smaa_rt_metrics;
#define SMAA_RT_METRICS (g_smaa_rt_metrics)

// Sampling helpers.
#define SMAASampleLevelZero(tex, coord) tex.SampleLevel(linear_sampler, coord, 0)
#define SMAASampleLevelZeroPoint(tex, coord) tex.SampleLevel(nearest_sampler, coord, 0)
#define SMAASampleLevelZeroOffset(tex, coord, offset) tex.SampleLevel(linear_sampler, coord, 0, offset)
#define SMAASample(tex, coord) tex.Sample(linear_sampler, coord)
#define SMAASamplePoint(tex, coord) tex.Sample(nearest_sampler, coord)
#define SMAASampleOffset(tex, coord, offset) tex.Sample(linear_sampler, coord, offset)

struct SmaaVertex {
    float4 position : SV_Position;
    float2 uv : TEXCOORD0;
};

SmaaVertex smaa_vertex(uint vertex_index) {
    float2 uv = float2(float((vertex_index << 1) & 2), float(vertex_index & 2));

    SmaaVertex vertex;
    vertex.position = float4(uv * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);
    vertex.uv = uv;
    return vertex;
}

//-----------------------------------------------------------------------------
// Misc functions

void SMAAMovc(bool2 cond, inout float2 variable, float2 value) {
    SMAA_FLATTEN
    if (cond.x) {
        variable.x = value.x;
    }

    SMAA_FLATTEN
    if (cond.y) {
        variable.y = value.y;
    }
}

void SMAAMovc(bool4 cond, inout float4 variable, float4 value) {
    SMAAMovc(cond.xy, variable.xy, value.xy);
    SMAAMovc(cond.zw, variable.zw, value.zw);
}

// Allows to decode two binary values from a bilinear-filtered access.
float2 SMAADecodeDiagBilinearAccess(float2 e) {
    e.r = e.r * abs(5.0 * e.r - 5.0 * 0.75);
    return round(e);
}

float4 SMAADecodeDiagBilinearAccess(float4 e) {
    e.rb = e.rb * abs(5.0 * e.rb - 5.0 * 0.75);
    return round(e);
}

//-----------------------------------------------------------------------------
// Edge detection (first pass)

float2 SMAALumaEdgeDetectionPS(float2 texcoord, float4 offset[3], Texture2D color_tex) {
    // Calculate the threshold:
    float2 threshold = float2(SMAA_THRESHOLD, SMAA_THRESHOLD);

    // Calculate lumas:
    float3 weights = float3(0.2126, 0.7152, 0.0722);
    float L = dot(SMAASamplePoint(color_tex, texcoord).rgb, weights);

    float Lleft = dot(SMAASamplePoint(color_tex, offset[0].xy).rgb, weights);
    float Ltop = dot(SMAASamplePoint(color_tex, offset[0].zw).rgb, weights);

    // We do the usual threshold:
    float4 delta;
    delta.xy = abs(L - float2(Lleft, Ltop));
    float2 edges = step(threshold, delta.xy);

    // Then discard if there is no edge:
    if (dot(edges, float2(1.0, 1.0)) == 0.0) {
        return float2(0.0, 0.0);
    }

    // Calculate right and bottom deltas:
    float Lright = dot(SMAASamplePoint(color_tex, offset[1].xy).rgb, weights);
    float Lbottom = dot(SMAASamplePoint(color_tex, offset[1].zw).rgb, weights);
    delta.zw = abs(L - float2(Lright, Lbottom));

    // Calculate the maximum delta in the direct neighborhood:
    float2 maxDelta = max(delta.xy, delta.zw);

    // Calculate left-left and top-top deltas:
    float Lleftleft = dot(SMAASamplePoint(color_tex, offset[2].xy).rgb, weights);
    float Ltoptop = dot(SMAASamplePoint(color_tex, offset[2].zw).rgb, weights);
    delta.zw = abs(float2(Lleft, Ltop) - float2(Lleftleft, Ltoptop));

    // Calculate the final maximum delta:
    maxDelta = max(maxDelta.xy, delta.zw);
    float finalDelta = max(maxDelta.x, maxDelta.y);

    // Local contrast adaptation:
    edges.xy *= step(finalDelta, SMAA_LOCAL_CONTRAST_ADAPTATION_FACTOR * delta.xy);

    return edges;
}

//-----------------------------------------------------------------------------
// Diagonal search functions

#ifndef SMAA_DISABLE_DIAG_DETECTION

// Diagonal pattern searches.
float2 SMAASearchDiag1(Texture2D edges_tex, float2 texcoord, float2 dir, out float2 e) {
    float4 coord = float4(texcoord, -1.0, 1.0);
    float3 t = float3(SMAA_RT_METRICS.xy, 1.0);

    while (coord.z < float(SMAA_MAX_SEARCH_STEPS_DIAG - 1) && coord.w > 0.9) {
        coord.xyz = mad(t, float3(dir, 1.0), coord.xyz);
        e = SMAASampleLevelZero(edges_tex, coord.xy).rg;
        coord.w = dot(e, float2(0.5, 0.5));
    }

    return coord.zw;
}

float2 SMAASearchDiag2(Texture2D edges_tex, float2 texcoord, float2 dir, out float2 e) {
    float4 coord = float4(texcoord, -1.0, 1.0);
    coord.x += 0.25 * SMAA_RT_METRICS.x; // See @SearchDiag2Optimization
    float3 t = float3(SMAA_RT_METRICS.xy, 1.0);

    while (coord.z < float(SMAA_MAX_SEARCH_STEPS_DIAG - 1) && coord.w > 0.9) {
        coord.xyz = mad(t, float3(dir, 1.0), coord.xyz);

        // Fetch both edges at once using bilinear filtering:
        e = SMAASampleLevelZero(edges_tex, coord.xy).rg;
        e = SMAADecodeDiagBilinearAccess(e);

        coord.w = dot(e, float2(0.5, 0.5));
    }

    return coord.zw;
}

// Calculates the area corresponding to a certain diagonal distance and
// crossing edges 'e'.
float2 SMAAAreaDiag(Texture2D area_tex, float2 dist, float2 e, float offset) {
    float2 texcoord = mad(float2(SMAA_AREATEX_MAX_DISTANCE_DIAG, SMAA_AREATEX_MAX_DISTANCE_DIAG), e, dist);

    // We do a scale and bias for mapping to texel space:
    texcoord = mad(SMAA_AREATEX_PIXEL_SIZE, texcoord, 0.5 * SMAA_AREATEX_PIXEL_SIZE);

    // Diagonal areas are on the second half of the texture:
    texcoord.x += 0.5;

    // Move to proper place, according to the subpixel offset:
    texcoord.y += SMAA_AREATEX_SUBTEX_SIZE * offset;

    return SMAASampleLevelZero(area_tex, texcoord).rg;
}

// This searches for diagonal patterns and returns the corresponding weights.
float2 SMAACalculateDiagWeights(Texture2D edges_tex, Texture2D area_tex, float2 texcoord, float2 e, float4 subsample_indices) {
    float2 weights = float2(0.0, 0.0);

    // Search for the line ends:
    float4 d;
    float2 end;

    if (e.r > 0.0) {
        d.xz = SMAASearchDiag1(edges_tex, texcoord, float2(-1.0, 1.0), end);
        d.x += float(end.y > 0.9);
    } else {
        d.xz = float2(0.0, 0.0);
    }

    d.yw = SMAASearchDiag1(edges_tex, texcoord, float2(1.0, -1.0), end);

    SMAA_BRANCH
    if (d.x + d.y > 2.0) { // d.x + d.y + 1 > 3
        // Fetch the crossing edges:
        float4 coords = mad(float4(-d.x + 0.25, d.x, d.y, -d.y - 0.25), SMAA_RT_METRICS.xyxy, texcoord.xyxy);
        float4 c;
        c.xy = SMAASampleLevelZeroOffset(edges_tex, coords.xy, int2(-1, 0)).rg;
        c.zw = SMAASampleLevelZeroOffset(edges_tex, coords.zw, int2(1, 0)).rg;
        c.yxwz = SMAADecodeDiagBilinearAccess(c.xyzw);

        // Merge crossing edges at each side into a single value:
        float2 cc = mad(float2(2.0, 2.0), c.xz, c.yw);

        // Remove the crossing edge if we didn't found the end of the line:
        SMAAMovc(bool2(step(0.9, d.zw)), cc, float2(0.0, 0.0));

        // Fetch the areas for this line:
        weights += SMAAAreaDiag(area_tex, d.xy, cc, subsample_indices.z);
    }

    // Search for the line ends:
    d.xz = SMAASearchDiag2(edges_tex, texcoord, float2(-1.0, -1.0), end);

    if (SMAASampleLevelZeroOffset(edges_tex, texcoord, int2(1, 0)).r > 0.0) {
        d.yw = SMAASearchDiag2(edges_tex, texcoord, float2(1.0, 1.0), end);
        d.y += float(end.y > 0.9);
    } else {
        d.yw = float2(0.0, 0.0);
    }

    SMAA_BRANCH
    if (d.x + d.y > 2.0) { // d.x + d.y + 1 > 3
        // Fetch the crossing edges:
        float4 coords = mad(float4(-d.x, -d.x, d.y, d.y), SMAA_RT_METRICS.xyxy, texcoord.xyxy);
        float4 c;
        c.x = SMAASampleLevelZeroOffset(edges_tex, coords.xy, int2(-1, 0)).g;
        c.y = SMAASampleLevelZeroOffset(edges_tex, coords.xy, int2(0, -1)).r;
        c.zw = SMAASampleLevelZeroOffset(edges_tex, coords.zw, int2(1, 0)).gr;
        float2 cc = mad(float2(2.0, 2.0), c.xz, c.yw);

        // Remove the crossing edge if we didn't found the end of the line:
        SMAAMovc(bool2(step(0.9, d.zw)), cc, float2(0.0, 0.0));

        // Fetch the areas for this line:
        weights += SMAAAreaDiag(area_tex, d.xy, cc, subsample_indices.w).gr;
    }

    return weights;
}

#endif // SMAA_DISABLE_DIAG_DETECTION

//-----------------------------------------------------------------------------
// Horizontal/vertical search functions

// Determines how much length should be added in the last step of the searches.
float SMAASearchLength(Texture2D search_tex, float2 e, float offset) {
    // The texture is flipped vertically, with left and right cases taking half
    // of the space horizontally:
    float2 scale = SMAA_SEARCHTEX_SIZE * float2(0.5, -1.0);
    float2 bias = SMAA_SEARCHTEX_SIZE * float2(offset, 1.0);

    // Scale and bias to access texel centers:
    scale += float2(-1.0, 1.0);
    bias += float2(0.5, -0.5);

    // Convert from pixel coordinates to texcoords:
    // (We use SMAA_SEARCHTEX_PACKED_SIZE because the texture is cropped)
    scale *= 1.0 / SMAA_SEARCHTEX_PACKED_SIZE;
    bias *= 1.0 / SMAA_SEARCHTEX_PACKED_SIZE;

    // Lookup the search texture:
    return SMAASampleLevelZero(search_tex, mad(scale, e, bias)).r;
}

float SMAASearchXLeft(Texture2D edges_tex, Texture2D search_tex, float2 texcoord, float end) {
    // @PSEUDO_GATHER4
    // This texcoord has been offset by (-0.25, -0.125) before calling this
    // function to sample between edges, thus fetching four edges in a row.
    float2 e = float2(0.0, 1.0);

    while (texcoord.x > end && e.g > 0.8281 && e.r == 0.0) {
        e = SMAASampleLevelZero(edges_tex, texcoord).rg;
        texcoord = mad(-float2(2.0, 0.0), SMAA_RT_METRICS.xy, texcoord);
    }

    float offset = mad(-(255.0 / 127.0), SMAASearchLength(search_tex, e, 0.0), 3.25);

    return mad(SMAA_RT_METRICS.x, offset, texcoord.x);
}

float SMAASearchXRight(Texture2D edges_tex, Texture2D search_tex, float2 texcoord, float end) {
    float2 e = float2(0.0, 1.0);

    while (texcoord.x < end && e.g > 0.8281 && e.r == 0.0) {
        e = SMAASampleLevelZero(edges_tex, texcoord).rg;
        texcoord = mad(float2(2.0, 0.0), SMAA_RT_METRICS.xy, texcoord);
    }

    float offset = mad(-(255.0 / 127.0), SMAASearchLength(search_tex, e, 0.5), 3.25);

    return mad(-SMAA_RT_METRICS.x, offset, texcoord.x);
}

float SMAASearchYUp(Texture2D edges_tex, Texture2D search_tex, float2 texcoord, float end) {
    float2 e = float2(1.0, 0.0);

    while (texcoord.y > end && e.r > 0.8281 && e.g == 0.0) {
        e = SMAASampleLevelZero(edges_tex, texcoord).rg;
        texcoord = mad(-float2(0.0, 2.0), SMAA_RT_METRICS.xy, texcoord);
    }

    float offset = mad(-(255.0 / 127.0), SMAASearchLength(search_tex, e.gr, 0.0), 3.25);

    return mad(SMAA_RT_METRICS.y, offset, texcoord.y);
}

float SMAASearchYDown(Texture2D edges_tex, Texture2D search_tex, float2 texcoord, float end) {
    float2 e = float2(1.0, 0.0);

    while (texcoord.y < end && e.r > 0.8281 && e.g == 0.0) {
        e = SMAASampleLevelZero(edges_tex, texcoord).rg;
        texcoord = mad(float2(0.0, 2.0), SMAA_RT_METRICS.xy, texcoord);
    }

    float offset = mad(-(255.0 / 127.0), SMAASearchLength(search_tex, e.gr, 0.5), 3.25);

    return mad(-SMAA_RT_METRICS.y, offset, texcoord.y);
}

// Calculates the area of each side of the current edge.
float2 SMAAArea(Texture2D area_tex, float2 dist, float e1, float e2, float offset) {
    // Rounding prevents precision errors of bilinear filtering:
    float2 texcoord = mad(float2(SMAA_AREATEX_MAX_DISTANCE, SMAA_AREATEX_MAX_DISTANCE), round(4.0 * float2(e1, e2)), dist);

    // We do a scale and bias for mapping to texel space:
    texcoord = mad(SMAA_AREATEX_PIXEL_SIZE, texcoord, 0.5 * SMAA_AREATEX_PIXEL_SIZE);

    // Move to proper place, according to the subpixel offset:
    texcoord.y = mad(SMAA_AREATEX_SUBTEX_SIZE, offset, texcoord.y);

    return SMAASampleLevelZero(area_tex, texcoord).rg;
}

//-----------------------------------------------------------------------------
// Corner detection functions

#ifndef SMAA_DISABLE_CORNER_DETECTION

void SMAADetectHorizontalCornerPattern(Texture2D edges_tex, inout float2 weights, float4 texcoord, float2 d) {
    float2 left_right = step(d.xy, d.yx);
    float2 rounding = (1.0 - SMAA_CORNER_ROUNDING_NORM) * left_right;

    // Reduce blending for pixels in the center of a line.
    rounding /= left_right.x + left_right.y;

    float2 factor = float2(1.0, 1.0);
    factor.x -= rounding.x * SMAASampleLevelZeroOffset(edges_tex, texcoord.xy, int2(0, 1)).r;
    factor.x -= rounding.y * SMAASampleLevelZeroOffset(edges_tex, texcoord.zw, int2(1, 1)).r;
    factor.y -= rounding.x * SMAASampleLevelZeroOffset(edges_tex, texcoord.xy, int2(0, -2)).r;
    factor.y -= rounding.y * SMAASampleLevelZeroOffset(edges_tex, texcoord.zw, int2(1, -2)).r;

    weights *= saturate(factor);
}

void SMAADetectVerticalCornerPattern(Texture2D edges_tex, inout float2 weights, float4 texcoord, float2 d) {
    float2 left_right = step(d.xy, d.yx);
    float2 rounding = (1.0 - SMAA_CORNER_ROUNDING_NORM) * left_right;

    rounding /= left_right.x + left_right.y;

    float2 factor = float2(1.0, 1.0);
    factor.x -= rounding.x * SMAASampleLevelZeroOffset(edges_tex, texcoord.xy, int2(1, 0)).g;
    factor.x -= rounding.y * SMAASampleLevelZeroOffset(edges_tex, texcoord.zw, int2(1, 1)).g;
    factor.y -= rounding.x * SMAASampleLevelZeroOffset(edges_tex, texcoord.xy, int2(-2, 0)).g;
    factor.y -= rounding.y * SMAASampleLevelZeroOffset(edges_tex, texcoord.zw, int2(-2, 1)).g;

    weights *= saturate(factor);
}

#endif // SMAA_DISABLE_CORNER_DETECTION

//-----------------------------------------------------------------------------
// Blending weight calculation (second pass)

float4 SMAABlendingWeightCalculationPS(
    float2 texcoord,
    float2 pixcoord,
    float4 offset[3],
    Texture2D edges_tex,
    Texture2D area_tex,
    Texture2D search_tex,
    float4 subsample_indices
) {
    float4 weights = float4(0.0, 0.0, 0.0, 0.0);

    float2 e = SMAASample(edges_tex, texcoord).rg;

    SMAA_BRANCH
    if (e.g > 0.0) { // Edge at north
        // Diagonals have both north and west edges, so searching for them in one
        // of the boundaries is enough.
        bool do_horizontal_processing;

#ifndef SMAA_DISABLE_DIAG_DETECTION
        weights.rg = SMAACalculateDiagWeights(edges_tex, area_tex, texcoord, e, subsample_indices);
        // We give priority to diagonals, so if we find a diagonal we skip
        // horizontal/vertical processing.
        do_horizontal_processing = weights.r == -weights.g; // weights.r + weights.g == 0.0
#else
        do_horizontal_processing = true;
#endif

        SMAA_BRANCH
        if (do_horizontal_processing) {
            float2 d;
            float3 coords;

            // Find the distance to the left:
            coords.x = SMAASearchXLeft(edges_tex, search_tex, offset[0].xy, offset[2].x);
            coords.y = offset[1].y; // offset[1].y = texcoord.y - 0.25 * SMAA_RT_METRICS.y (@CROSSING_OFFSET)
            d.x = coords.x;

            // Now fetch the left crossing edges, two at a time using bilinear
            // filtering. Sampling at -0.25 (see @CROSSING_OFFSET) enables to
            // discern what value each edge has:
            float e1 = SMAASampleLevelZero(edges_tex, coords.xy).r;

            // Find the distance to the right:
            coords.z = SMAASearchXRight(edges_tex, search_tex, offset[0].zw, offset[2].y);
            d.y = coords.z;

            // We want the distances to be in pixel units:
            d = abs(round(mad(SMAA_RT_METRICS.zz, d, -pixcoord.xx)));

            // SMAAArea below needs a sqrt, as the areas texture is compressed
            // quadratically:
            float2 sqrt_d = sqrt(d);

            // Fetch the right crossing edges:
            float e2 = SMAASampleLevelZeroOffset(edges_tex, coords.zy, int2(1, 0)).r;

            // Ok, we know how this pattern looks like, now it is time for getting
            // the actual area:
            weights.rg = SMAAArea(area_tex, sqrt_d, e1, e2, subsample_indices.y);

#ifndef SMAA_DISABLE_CORNER_DETECTION
            // Fix corners:
            coords.y = texcoord.y;
            SMAADetectHorizontalCornerPattern(edges_tex, weights.rg, coords.xyzy, d);
#endif
        } else {
            e.r = 0.0; // Skip vertical processing.
        }
    }

    SMAA_BRANCH
    if (e.r > 0.0) { // Edge at west
        float2 d;
        float3 coords;

        // Find the distance to the top:
        coords.y = SMAASearchYUp(edges_tex, search_tex, offset[1].xy, offset[2].z);
        coords.x = offset[0].x; // offset[1].x = texcoord.x - 0.25 * SMAA_RT_METRICS.x;
        d.x = coords.y;

        // Fetch the top crossing edges:
        float e1 = SMAASampleLevelZero(edges_tex, coords.xy).g;

        // Find the distance to the bottom:
        coords.z = SMAASearchYDown(edges_tex, search_tex, offset[1].zw, offset[2].w);
        d.y = coords.z;

        // We want the distances to be in pixel units:
        d = abs(round(mad(SMAA_RT_METRICS.ww, d, -pixcoord.yy)));

        // SMAAArea below needs a sqrt, as the areas texture is compressed
        // quadratically:
        float2 sqrt_d = sqrt(d);

        // Fetch the bottom crossing edges:
        float e2 = SMAASampleLevelZeroOffset(edges_tex, coords.xz, int2(0, 1)).g;

        // Get the area for this direction:
        weights.ba = SMAAArea(area_tex, sqrt_d, e1, e2, subsample_indices.x);

#ifndef SMAA_DISABLE_CORNER_DETECTION
        // Fix corners:
        coords.x = texcoord.x;
        SMAADetectVerticalCornerPattern(edges_tex, weights.ba, coords.xyxz, d);
#endif
    }

    return weights;
}

//-----------------------------------------------------------------------------
// Neighborhood blending (third pass)

float4 SMAANeighborhoodBlendingPS(float2 texcoord, float4 offset, Texture2D color_tex, Texture2D blend_tex) {
    // Fetch the blending weights for current pixel:
    float4 a;
    a.x = SMAASample(blend_tex, offset.xy).a; // Right
    a.y = SMAASample(blend_tex, offset.zw).g; // Top
    a.wz = SMAASample(blend_tex, texcoord).xz; // Bottom / Left

    // Is there any blending weight with a value greater than 0.0?
    SMAA_BRANCH
    if (dot(a, float4(1.0, 1.0, 1.0, 1.0)) < 1e-5) {
        return SMAASampleLevelZero(color_tex, texcoord);
    }

    bool h = max(a.x, a.z) > max(a.y, a.w); // max(horizontal) > max(vertical)

    // Calculate the blending offsets:
    float4 blending_offset = float4(0.0, a.y, 0.0, a.w);
    float2 blending_weight = a.yw;
    SMAAMovc(bool4(h, h, h, h), blending_offset, float4(a.x, 0.0, a.z, 0.0));
    SMAAMovc(bool2(h, h), blending_weight, a.xz);
    blending_weight /= dot(blending_weight, float2(1.0, 1.0));

    // Calculate the texture coordinates:
    float4 blending_coord = mad(blending_offset, float4(SMAA_RT_METRICS.xy, -SMAA_RT_METRICS.xy), texcoord.xyxy);

    // We exploit bilinear filtering to mix current pixel with the chosen
    // neighbor:
    float4 color = blending_weight.x * SMAASampleLevelZero(color_tex, blending_coord.xy);
    color += blending_weight.y * SMAASampleLevelZero(color_tex, blending_coord.zw);

    return color;
}
