#import bevy_sprite::mesh2d_functions

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
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var terrain_textures: texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var terrain_sampler: sampler;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh2d_functions::get_world_from_local(vertex.instance_index);
    out.position = mesh2d_functions::mesh2d_position_local_to_clip(
        world_from_local,
        vec4<f32>(vertex.position, 1.0)
    );
    out.uv = vertex.uv;
    out.texture_index = vertex.texture_index;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(terrain_textures, terrain_sampler, in.uv, in.texture_index);
}
