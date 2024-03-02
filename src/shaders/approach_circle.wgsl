// Vertex shader
struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct OsuShaderState {
	time: f32,
	preempt: f32,
	fadein: f32,
	hit_offset: f32
}

@group(2) @binding(0)
var<uniform> shader_state: OsuShaderState;

struct VertexInput {
	@location(0) pos: vec2<f32>,
	@location(1) uv: vec2<f32>,
}

struct InstanceInput {
	@location(2) pos: vec4<f32>,
	@location(3) time: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
	@location(0) uv: vec2<f32>,
	@location(1) alpha: f32,
};

fn interpolate(x1: f32, y1: f32, x3: f32, y3: f32, x2: f32) -> f32 {
	return (x2-x1)*(y3-y1)/(x3-x1)+y1;
}

@vertex
fn vs_main(
	model: VertexInput,
	instance: InstanceInput
) -> VertexOutput {
    var out: VertexOutput;
	out.uv = model.uv;

	let model_matrix = mat4x4<f32>(
		1.0, 0.0, 0.0, 0.0,
		0.0, 1.0, 0.0, 0.0,
		0.0, 0.0, 1.0, 0.0,
		instance.pos.x, instance.pos.y, 0.0, 1.0,
	);

	let approach_scale: f32 = clamp(interpolate(
		instance.time,
		1.0,
		instance.time - shader_state.preempt,
		4.0,
		shader_state.time
	), 1.0, 4.0);

	let start_time = instance.time - shader_state.preempt;
	let end_time = start_time + shader_state.fadein;

	let fadein_alpha2 = (shader_state.time-start_time)/(end_time-start_time);

	if shader_state.time > instance.time {
		out.alpha = 0.0;
	} else {
		out.alpha = fadein_alpha2;
	}

	let scaled_pos = vec4<f32>(approach_scale, approach_scale, 0.0, 1.0) 
		* vec4<f32>(model.pos, 0.0, 1.0);

    out.clip_position = camera.view_proj 
		* model_matrix
		* scaled_pos;

    return out;
}


// Fragment shader

@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	var out = textureSample(texture, texture_sampler, in.uv);
	out.w = out.w * in.alpha;
	return out;
	//return vec4<f32>(1.0, 0.2, 0.1, in.alpha);
	//return vec4<f32>(1.0, 0.2, 0.1, in.alpha);
}
