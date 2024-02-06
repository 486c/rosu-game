// Vertex shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

@group(2) @binding(0)
var<uniform> cs: f32;

struct VertexInput {
	@location(0) pos: vec2<f32>,
	@location(1) uv: vec2<f32>,
}

struct InstanceInput {
	@location(2) row1: vec4<f32>,
	@location(3) row2: vec4<f32>,
	@location(4) row3: vec4<f32>,
	@location(5) row4: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
	@location(0) uv: vec2<f32>,
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

    out.clip_position = camera.view_proj 
		* model_matrix
		* vec4<f32>(model.pos * cs, 0.0, 1.0);

    return out;
}


// Fragment shader

@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	return textureSample(texture, texture_sampler, in.uv);
	//return vec4<f32>(1.0, 0.2, 0.1, 1.0);
}
