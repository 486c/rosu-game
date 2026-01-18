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
	@location(3) color: vec3<f32>,
	@location(4) alpha: f32,
	@location(5) degree: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
	@location(0) uv: vec2<f32>,
	@location(1) alpha: f32,
	@location(2) degree: f32,
	@location(3) color: vec3<f32>
};

@vertex
fn vs_main(
	model: VertexInput,
	instance: InstanceInput
) -> VertexOutput {
    var out: VertexOutput;
	out.uv = model.uv;
	out.alpha = instance.alpha;
	out.degree = instance.degree;
	out.color = instance.color;

    let rotation_matrix = mat3x3<f32>(
        cos(radians(instance.degree)), -sin(radians(instance.degree)), 0.0,
        sin(radians(instance.degree)), cos(radians(instance.degree)), 0.0,
        0.0, 0.0, 1.0
    );

    let rotated_position = vec3<f32>(
        rotation_matrix * vec3<f32>(
            model.pos.x + instance.pos.x,
            model.pos.y + instance.pos.y,
            instance.pos.z + 0.0
        )
    );

    out.clip_position = camera.proj * camera.view
		* vec4<f32>(
			model.pos.x + instance.pos.x,
			model.pos.y + instance.pos.y,
			instance.pos.z,
			1.0
		);

    return out;
}


// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let border_thickness = 0.01;
	
	let dist_from_left = in.uv.x;
    let dist_from_right = 1.0 - in.uv.x;
    let dist_from_top = in.uv.y;
    let dist_from_bottom = 1.0 - in.uv.y;

	let min_dist = min(min(dist_from_left, dist_from_right),
			min(dist_from_top, dist_from_bottom));

	if (min_dist > border_thickness) {
		discard;
	}

	var hc = vec4<f32>(in.color, in.alpha);

	return hc;
}
