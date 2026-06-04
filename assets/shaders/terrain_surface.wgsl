#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
}

struct TerrainMaterialParams {
    slope_triplanar_start: f32,
    slope_triplanar_end: f32,
    texture_scale: f32,
    ambient_light: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var terrain_textures: texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var terrain_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> material: TerrainMaterialParams;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) blend_layers: vec4<f32>,
    @location(3) blend_weights: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) blend_layers: vec4<f32>,
    @location(3) blend_weights: vec4<f32>,
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    let world_from_local = mesh_functions::get_world_from_local(input.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(input.position, 1.0),
    );

    var output: VertexOutput;
    output.clip_position = position_world_to_clip(world_position.xyz);
    output.world_position = world_position.xyz;
    output.world_normal = mesh_functions::mesh_normal_local_to_world(
        input.normal,
        input.instance_index,
    );
    output.blend_layers = input.blend_layers;
    output.blend_weights = input.blend_weights;
    return output;
}

fn sample_planar(layer: i32, world_position: vec3<f32>) -> vec4<f32> {
    let uv = fract(world_position.xy * material.texture_scale);
    return textureSample(terrain_textures, terrain_sampler, uv, layer);
}

fn sample_triplanar(layer: i32, world_position: vec3<f32>, normal: vec3<f32>) -> vec4<f32> {
    let axis = abs(normal);
    let powered = axis * axis * axis * axis;
    let weight_sum = max(powered.x + powered.y + powered.z, 0.0001);
    let weights = powered / weight_sum;
    let scale = material.texture_scale;
    let x_projection = textureSample(terrain_textures, terrain_sampler, fract(world_position.yz * scale), layer);
    let y_projection = textureSample(terrain_textures, terrain_sampler, fract(world_position.xz * scale), layer);
    let z_projection = textureSample(terrain_textures, terrain_sampler, fract(world_position.xy * scale), layer);
    return x_projection * weights.x + y_projection * weights.y + z_projection * weights.z;
}

fn sample_layer(layer: f32, world_position: vec3<f32>, normal: vec3<f32>, triplanar_amount: f32) -> vec4<f32> {
    let layer_index = i32(round(layer));
    let planar = sample_planar(layer_index, world_position);
    let triplanar = sample_triplanar(layer_index, world_position, normal);
    return mix(planar, triplanar, triplanar_amount);
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.world_normal);
    let slope = 1.0 - abs(normal.z);
    let triplanar_amount = smoothstep(
        material.slope_triplanar_start,
        material.slope_triplanar_end,
        slope,
    );
    let total_weight = max(
        input.blend_weights.x + input.blend_weights.y + input.blend_weights.z + input.blend_weights.w,
        0.0001,
    );
    let blend_weights = input.blend_weights / total_weight;
    var color =
        sample_layer(input.blend_layers.x, input.world_position, normal, triplanar_amount) * blend_weights.x;
    color +=
        sample_layer(input.blend_layers.y, input.world_position, normal, triplanar_amount) * blend_weights.y;
    color +=
        sample_layer(input.blend_layers.z, input.world_position, normal, triplanar_amount) * blend_weights.z;
    color +=
        sample_layer(input.blend_layers.w, input.world_position, normal, triplanar_amount) * blend_weights.w;

    let light_direction = normalize(vec3<f32>(-0.35, -0.45, 0.82));
    let diffuse = max(dot(normal, light_direction), 0.0);
    let lighting = material.ambient_light + (1.0 - material.ambient_light) * diffuse;
    return vec4<f32>(color.rgb * lighting, 1.0);
}
