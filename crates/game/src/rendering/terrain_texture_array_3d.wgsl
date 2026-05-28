#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(5) texture_index: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) texture_index: u32,
    @location(2) world_normal: vec3<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var terrain_textures: texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var terrain_sampler: sampler;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0)
    );
    out.position = position_world_to_clip(world_position.xyz);
    out.uv = vertex.uv;
    out.texture_index = vertex.texture_index;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex.instance_index
    );
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(terrain_textures, terrain_sampler, in.uv, in.texture_index);
    let normal = normalize(in.world_normal);
    let light_dir = normalize(vec3<f32>(-0.45, 0.55, 0.72));
    let diffuse = max(dot(normal, light_dir), 0.0);
    let shade = 0.46 + diffuse * 0.72;
    return vec4<f32>(texel.rgb * shade, texel.a);
}
