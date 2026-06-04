//! Terrain surface material and vertex attributes.

use bevy::{
    asset::Asset,
    image::Image,
    mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef},
    pbr::{Material, MaterialPipeline, MaterialPipelineKey},
    prelude::Handle,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
        VertexFormat,
    },
    shader::ShaderRef,
};

const TERRAIN_SURFACE_SHADER: &str = "shaders/terrain_surface.wgsl";

pub(crate) const ATTRIBUTE_TERRAIN_BLEND_LAYERS: MeshVertexAttribute =
    MeshVertexAttribute::new("TerrainBlendLayers", 871_420_001, VertexFormat::Float32x4);
pub(crate) const ATTRIBUTE_TERRAIN_BLEND_WEIGHTS: MeshVertexAttribute =
    MeshVertexAttribute::new("TerrainBlendWeights", 871_420_002, VertexFormat::Float32x4);

/// Shader controls for the terrain surface material.
#[derive(Debug, Clone, Copy, ShaderType)]
pub(crate) struct TerrainMaterialParams {
    slope_triplanar_start: f32,
    slope_triplanar_end: f32,
    texture_scale: f32,
    ambient_light: f32,
}

impl Default for TerrainMaterialParams {
    fn default() -> Self {
        Self {
            slope_triplanar_start: 0.28,
            slope_triplanar_end: 0.58,
            texture_scale: 0.055,
            ambient_light: 0.45,
        }
    }
}

/// Material for blended biome terrain.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(crate) struct TerrainMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    pub(crate) texture_array: Handle<Image>,
    #[uniform(2)]
    pub(crate) params: TerrainMaterialParams,
}

impl Material for TerrainMaterial {
    fn vertex_shader() -> ShaderRef {
        TERRAIN_SURFACE_SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        TERRAIN_SURFACE_SHADER.into()
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            bevy::prelude::Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            bevy::prelude::Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            ATTRIBUTE_TERRAIN_BLEND_LAYERS.at_shader_location(2),
            ATTRIBUTE_TERRAIN_BLEND_WEIGHTS.at_shader_location(3),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}
