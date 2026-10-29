// Vertex shader
struct CameraUniform {
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

/// Visible replay-frame window, updated once per rendered frame.
/// Fade alpha is computed on the GPU from the vertex index, so the
/// CPU never has to touch the big vertex buffers while scrubbing.
struct WindowUniform {
    start: u32,
    end: u32,
    selected: u32,
    pad: u32,
};

@group(1) @binding(0)
var<uniform> window: WindowUniform;

struct VertexInput {
	@location(0) pos: vec3<f32>,
	@location(1) alpha: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
	@location(1) alpha: f32,
	@location(2) color: vec3<f32>,
};

@vertex
fn vs_main(
	model: VertexInput,
	@builtin(vertex_index) frame_idx: u32,
) -> VertexOutput {
	// vertex_index starts at firstVertex (= window.start here),
	// so it is the absolute replay frame index.
	let count = max(window.end - window.start, 1u);
	var alpha = clamp(f32(frame_idx - window.start) / f32(count), 0.0, 1.0);
	var color = vec3<f32>(1.0, 1.0, 0.0);

	if (frame_idx == window.selected) {
		color = vec3<f32>(1.0, 0.0, 0.0);
		alpha = 1.0;
	}

    var out: VertexOutput;
	out.alpha = alpha;
	out.color = color;

	out.clip_position = camera.proj * camera.view
		* vec4<f32>(
			model.pos.x,
			model.pos.y,
			model.pos.z,
			1.0
		);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	var out = vec4<f32>(in.color, in.alpha);

	return out;
}
