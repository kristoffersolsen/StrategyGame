//! Texture-array terrain materials for map rendering.

use bevy::{
    asset::{load_internal_asset, uuid_handle},
    image::{Image, ImageSampler},
    pbr::{Material, MaterialPipeline, MaterialPipelineKey},
    prelude::{App, Asset, Handle},
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    },
    shader::{Shader, ShaderRef},
    sprite_render::{Material2d, Material2dKey},
};

use crate::rendering::hex_mesh::ATTRIBUTE_TEXTURE_INDEX;

const TERRAIN_TEXTURE_ARRAY_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("91b7e9fd-22d8-4ddf-a0c9-2cd3d939a8ac");
const TERRAIN_TEXTURE_ARRAY_3D_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("9c98d93d-83f4-46b0-8e43-6b0c8f2d3d64");

pub(crate) fn load_shader(app: &mut App) {
    load_internal_asset!(
        app,
        TERRAIN_TEXTURE_ARRAY_SHADER_HANDLE,
        "terrain_texture_array.wgsl",
        Shader::from_wgsl
    );
    load_internal_asset!(
        app,
        TERRAIN_TEXTURE_ARRAY_3D_SHADER_HANDLE,
        "terrain_texture_array_3d.wgsl",
        Shader::from_wgsl
    );
}

/// Custom 2D material that samples terrain tiles from a texture array.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(crate) struct TerrainTextureArrayMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    pub(crate) terrain_textures: Handle<Image>,
}

impl TerrainTextureArrayMaterial {
    pub(crate) fn from_texture(terrain_textures: Handle<Image>) -> Self {
        Self { terrain_textures }
    }
}

impl Material2d for TerrainTextureArrayMaterial {
    fn vertex_shader() -> ShaderRef {
        TERRAIN_TEXTURE_ARRAY_SHADER_HANDLE.into()
    }

    fn fragment_shader() -> ShaderRef {
        TERRAIN_TEXTURE_ARRAY_SHADER_HANDLE.into()
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            bevy::prelude::Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            bevy::prelude::Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            bevy::prelude::Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            ATTRIBUTE_TEXTURE_INDEX.at_shader_location(5),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

/// Custom 3D material that samples terrain from the authored texture array.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(crate) struct TerrainTextureArray3dMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    pub(crate) terrain_textures: Handle<Image>,
}

impl TerrainTextureArray3dMaterial {
    pub(crate) fn from_texture(terrain_textures: Handle<Image>) -> Self {
        Self { terrain_textures }
    }
}

impl Material for TerrainTextureArray3dMaterial {
    fn vertex_shader() -> ShaderRef {
        TERRAIN_TEXTURE_ARRAY_3D_SHADER_HANDLE.into()
    }

    fn fragment_shader() -> ShaderRef {
        TERRAIN_TEXTURE_ARRAY_3D_SHADER_HANDLE.into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            bevy::prelude::Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            bevy::prelude::Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            bevy::prelude::Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            ATTRIBUTE_TEXTURE_INDEX.at_shader_location(5),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

pub(crate) fn configure_placeholder_texture(image: &mut Image) {
    image.sampler = ImageSampler::nearest();
}
