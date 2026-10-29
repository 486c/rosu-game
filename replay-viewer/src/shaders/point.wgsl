// Vertex shader
struct CameraUniform {
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

/// Visible replay-frame window.
/// `selected` is NO_SELECTED_FRAME (0xFFFFFFFF) when nothing is selected.
struct WindowUniform {
    start: u32,
    end: u32,
    selected: u32,
    pad: u32,
};

@group(1) @binding(0)
var<uniform> window: WindowUniform;

struct VertexInput {
	@location(0) pos: vec2<f32>,
	@location(1) uv: vec2<f32>,
}

struct InstanceInput {
	@location(2) pos: vec3<f32>,
	@location(3) color: vec3<f32>,
	@location(4) alpha: f32,
	@location(5) scale: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
	@location(0) uv: vec2<f32>,
	@location(1) alpha: f32,
	@location(2) color: vec3<f32>,
};

@vertex
fn vs_main(
	model: VertexInput,
	instance: InstanceInput,
	@builtin(instance_index) frame_idx: u32,
) -> VertexOutput {
	// instance_index starts at firstInstance (= window.start here),
	// so it is the absolute replay frame index.
	let count = max(window.end - window.start, 1u);
	var alpha = clamp(f32(frame_idx - window.start) / f32(count), 0.0, 1.0);
	var color = instance.color;
	var scale = instance.scale;

	if (frame_idx == window.selected) {
		color = vec3<f32>(1.0, 0.0, 0.0);
		alpha = 1.0;
		scale = scale * 1.8;
	}

    var out: VertexOutput;
	out.uv = model.uv;
	out.alpha = alpha;
	out.color = color;

	let model_matrix = mat4x4<f32>(
		1.0, 0.0, 0.0, 0.0,
		0.0, 1.0, 0.0, 0.0,
		0.0, 0.0, 1.0, 0.0,
		instance.pos.x, instance.pos.y, instance.pos.z, 1.0,
	);

	let scaled_pos = vec4<f32>(scale, scale, 0.0, 1.0) * vec4<f32>(model.pos, 0.0, 1.0);

    out.clip_position = camera.proj * camera.view
		* model_matrix
		* scaled_pos;

    return out;
}


// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let thickness = 1.0;
	let fade = 1.0;

	let uv = in.uv * 2.0 - 1.0;

	let distance = 1.0 - length(uv);
	var color = vec3<f32>(smoothstep(0.0, fade, distance));
	color *= vec3<f32>(smoothstep(thickness + fade, thickness, distance));

	if (color.r == 0.0) {
		discard;
	}

	var out = vec4<f32>(color, in.alpha);
	out.r = in.color.x;
	out.g = in.color.y;
	out.b = in.color.z;

	return out;
}
