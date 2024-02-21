// Vertex shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct OsuShaderState {
	time: f32,
	preempt: f32,
	fadein: f32
}

@group(2) @binding(0)
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
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
	@location(0) uv: vec2<f32>,
	@location(1) alpha: f32,
};

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


	var x1 = instance.time - shader_state.preempt;
	var y1 = 0.0;

	var x3 = (instance.time - shader_state.preempt) + shader_state.fadein;
	var y3 = 1.0;

	var x2 = shader_state.time;

	var alpha = (x2-x1)*(y3-y1)/(x3-x1)+y1;

	if alpha > 1.0 {
		out.alpha = 0.0;
	} else {
		out.alpha = clamp(alpha, 0.0, 1.0);
	}

    out.clip_position = camera.view_proj 
		* model_matrix
		* vec4<f32>(model.pos, 0.0, 1.0);

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
