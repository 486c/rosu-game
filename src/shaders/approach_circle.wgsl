// Vertex shader
struct CameraUniform {
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
	@location(0) pos: vec2<f32>,
	@location(1) uv: vec2<f32>,
}

struct InstanceInput {
	@location(2) pos: vec3<f32>,
	@location(3) alpha: f32,
	@location(4) scale: f32,
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
	out.alpha = instance.alpha;

	let model_matrix = mat4x4<f32>(
		1.0, 0.0, 0.0, 0.0,
		0.0, 1.0, 0.0, 0.0,
		0.0, 0.0, 1.0, 0.0,
		instance.pos.x, instance.pos.y, instance.pos.z, 1.0,
	);

	let scaled_pos = vec4<f32>(instance.scale, instance.scale, 0.0, 1.0) * vec4<f32>(model.pos, 0.0, 1.0);

    out.clip_position = camera.proj * camera.view
		* model_matrix
		* scaled_pos;

    return out;
}


// Fragment shader
//@group(0) @binding(0)
//var texture: texture_2d<f32>;
//@group(0) @binding(1)
//var texture_sampler: sampler;


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let circle_color = vec3<f32>(1.0, 1.0, 1.0);
	let thickness = 0.05;
	let fade = 0.001;

	let uv = in.uv * 2.0 - 1.0;

	let distance = 1.0 - length(uv);
	var color = vec3<f32>(smoothstep(0.0, fade, distance));
	color *= vec3<f32>(smoothstep(thickness + fade, thickness, distance));

	if (color.r == 0.0) {
		discard;
	}

	var out = vec4<f32>(color, in.alpha);
	out.r = out.r * circle_color.r;
	out.g = out.g * circle_color.g;
	out.b = out.b * circle_color.b;

	return out;
}
