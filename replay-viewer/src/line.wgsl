// Vertex shader
struct CameraUniform {
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
	@location(0) pos: vec2<f32>,
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
) -> VertexOutput {
    var out: VertexOutput;
	out.alpha = model.alpha;
	//out.color = instance.color;

	let scaled_pos = vec4<f32>(5.0, 5.0, 0.0, 1.0) * vec4<f32>(model.pos, 0.0, 1.0);

	out.clip_position = camera.proj * camera.view
		* vec4<f32>(
			model.pos.x,
			model.pos.y,
			0.0,
			1.0
		);

    return out;
}


// Fragment shader
//@group(0) @binding(0)
//var texture: texture_2d<f32>;
//@group(0) @binding(1)
//var texture_sampler: sampler;


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	var out = vec4<f32>(1.0, 1.0, 0.0, in.alpha);

	return out;
}
