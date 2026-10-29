// Ring overlay for tool area-of-effect visualization.
struct CameraUniform {
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct InstanceInput {
    @location(2) pos: vec3<f32>,
    @location(3) color: vec3<f32>,
    @location(4) alpha: f32,
    @location(5) scale: f32,
}

struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) alpha: f32,
    @location(2) color: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    out.uv = model.uv;
    out.alpha = instance.alpha;
    out.color = instance.color;

    let scaled_pos = vec4<f32>(instance.scale, instance.scale, 0.0, 1.0)
        * vec4<f32>(model.pos, 0.0, 1.0);

    out.clip_position = camera.proj * camera.view
        * mat4x4<f32>(
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            instance.pos.x, instance.pos.y, instance.pos.z, 1.0,
        )
        * scaled_pos;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let d = length(in.uv * 2.0 - 1.0);

    let ring = smoothstep(0.9, 0.95, d) * (1.0 - smoothstep(0.96, 1.0, d));
    let fill = (1.0 - smoothstep(0.0, 1.0, d)) * 0.08;

    let alpha = clamp(ring + fill, 0.0, 1.0) * in.alpha;

    if (alpha < 0.003) {
        discard;
    }

    return vec4<f32>(in.color, alpha);
}
