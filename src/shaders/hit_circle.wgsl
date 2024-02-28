// Vertex shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(2) @binding(0)
var<uniform> camera: CameraUniform;

struct OsuShaderState {
	time: f32,
	preempt: f32,
	fadein: f32,
	hit_offset: f32
}

@group(3) @binding(0)
var<uniform> shader_state: OsuShaderState;

struct VertexInput {
	@location(0) pos: vec2<f32>,
	@location(1) uv: vec2<f32>,
}

struct InstanceInput {
	@location(2) row1: vec4<f32>,
	@location(3) row2: vec4<f32>,
	@location(4) row3: vec4<f32>,
	@location(5) row4: vec4<f32>,
	@location(6) time: f32,
	@location(7) is_approach: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
	@location(0) uv: vec2<f32>,
	@location(1) alpha: f32,
	@location(2) is_approach: u32,
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
		instance.row1,
		instance.row2,
		instance.row3,
		instance.row4,
	);

	var fadein_alpha = interpolate(
		instance.time - shader_state.preempt,
		0.0,
		(instance.time - shader_state.preempt) + shader_state.fadein,
		1.0,
		shader_state.time
	);

	if instance.time < shader_state.time + shader_state.hit_offset {
		out.alpha = 0.0;
	} else {
		out.alpha = fadein_alpha;
	}
	
	var scale = 1.0;
	if bool(instance.is_approach) {
		scale = 1.5;
	}

	var scaled_pos = vec4<f32>(scale, scale, 0.0, 1.0) 
		* vec4<f32>(model.pos, 0.0, 1.0);

    out.clip_position = camera.view_proj 
		* model_matrix
		* scaled_pos;

	out.is_approach = instance.is_approach;

    return out;
}


// Fragment shader

@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

@group(1) @binding(0)
var approach_texture: texture_2d<f32>;
@group(1) @binding(1)
var aprroach_texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	var out = vec4<f32>(1.0, 1.0, 1.0, 1.0);

	if bool(in.is_approach) {
		out = textureSample(approach_texture, aprroach_texture_sampler, in.uv);
	} else {
		out = textureSample(texture, texture_sampler, in.uv);
	}

	out.w = out.w * in.alpha;
	return out;
	//return vec4<f32>(1.0, 0.2, 0.1, in.alpha);
	//return vec4<f32>(1.0, 0.2, 0.1, in.alpha);
}
