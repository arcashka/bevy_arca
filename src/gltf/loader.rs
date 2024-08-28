use std::mem;

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    math::Affine2,
    prelude::*,
    tasks::IoTaskPool,
    utils::HashMap,
};

use gltf::{
    accessor::{DataType, Dimensions},
    image::Source,
    mesh::{util::ReadIndices, Mode},
    texture::TextureTransform,
    Accessor, Node, Semantic,
};

use image::ImageError;
use thiserror::Error;
use windows::Win32::Graphics::Direct3D12::{
    D3D12_PRIMITIVE_TOPOLOGY_TYPE, D3D12_PRIMITIVE_TOPOLOGY_TYPE_LINE,
    D3D12_PRIMITIVE_TOPOLOGY_TYPE_POINT, D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
};

use crate::{gltf::Gltf, image::Image, material::Material, mesh::Mesh};

use super::{tree_iterator::GltfTreeIterator, GltfAssetLabel, GltfMesh, GltfNode, GltfPrimitive};

pub struct GltfLoader;

#[derive(Error, Debug)]
pub enum GltfError {
    #[error("invalid glTF file: {0}")]
    Gltf(#[from] gltf::Error),
    #[error("failed to load file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image crate error: {0}")]
    ImageCrateError(ImageError),
    #[error("Unsupported image format: {0}")]
    UnsupportedImageFormat(String),
    #[error("Unsupported buffer format: {0}")]
    UnsupportedBufferFormat(String),
    #[error("Missing blob")]
    MissingBlob,
    #[error("Unsupported primitive mode")]
    UnsupportedPrimitive { mode: gltf::json::mesh::Mode },
    #[error("GLTF model must be a tree, found cycle instead at node indices: {0:?}")]
    CircularChildren(String),
}

impl AssetLoader for GltfLoader {
    type Asset = Gltf;
    type Settings = ();
    type Error = GltfError;
    async fn load<'a>(
        &'a self,
        reader: &'a mut dyn Reader,
        _settings: &'a (),
        load_context: &'a mut LoadContext<'_>,
    ) -> Result<Gltf, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        load_gltf(&bytes, load_context).await
    }

    fn extensions(&self) -> &[&str] {
        &["gltf", "glb"]
    }
}

fn load_material(material: &gltf::Material, load_context: &mut LoadContext) -> Handle<Material> {
    let material_label = material
        .index()
        .map(|i| GltfAssetLabel::Material { index: i });
    let Some(material_label) = material_label else {
        return Handle::default();
    };

    let pbr = material.pbr_metallic_roughness();

    let color = pbr.base_color_factor();
    let base_color_texture = pbr
        .base_color_texture()
        .map(|info| image_handle(load_context, &info.texture()));

    let uv_transform = pbr
        .base_color_texture()
        .and_then(|info| {
            info.texture_transform()
                .map(convert_texture_transform_to_affine2)
        })
        .unwrap_or_default();

    let normal_map_texture: Option<Handle<Image>> = material
        .normal_texture()
        .map(|normal_texture| image_handle(load_context, &normal_texture.texture()));

    let occlusion_texture = material
        .occlusion_texture()
        .map(|occlusion_texture| image_handle(load_context, &occlusion_texture.texture()));

    load_context.add_labeled_asset(
        material_label.to_string(),
        Material {
            base_color: Color::srgba(color[0], color[1], color[2], color[3]),
            base_color_texture,
            normal_map_texture,
            occlusion_texture,
            uv_transform,
        },
    )
}

fn image_handle(load_context: &mut LoadContext, texture: &gltf::Texture) -> Handle<Image> {
    match texture.source().source() {
        Source::View { .. } => {
            load_context.get_label_handle(GltfAssetLabel::Texture(texture.index()).to_string())
        }
        _ => panic!("Not implemented"),
    }
}

fn convert_texture_transform_to_affine2(texture_transform: TextureTransform) -> Affine2 {
    Affine2::from_scale_angle_translation(
        texture_transform.scale().into(),
        -texture_transform.rotation(),
        texture_transform.offset().into(),
    )
}

async fn load_gltf<'a, 'b, 'c>(
    bytes: &'a [u8],
    load_context: &'b mut LoadContext<'c>,
) -> Result<Gltf, GltfError> {
    let gltf = gltf::Gltf::from_slice(bytes)?;
    let buffer_data = load_buffers(&gltf).await?;

    IoTaskPool::get()
        .scope(|scope| {
            gltf.textures().for_each(|gltf_texture| {
                let buffer_data = &buffer_data;
                scope.spawn(async move { load_image(gltf_texture, buffer_data).await });
            });
        })
        .into_iter()
        .for_each(|result| match result {
            Ok((image, label)) => {
                load_context.add_labeled_asset(label, image);
            }
            Err(err) => {
                warn!("Error loading glTF texture: {}", err);
            }
        });

    let mut materials = vec![];
    for material in gltf.materials() {
        let handle = load_material(&material, load_context);
        materials.push(handle);
    }

    let mut meshes = vec![];
    for gltf_mesh in gltf.meshes() {
        let mut primitives = vec![];
        for primitive in gltf_mesh.primitives() {
            let primitive_label = GltfAssetLabel::Primitive {
                mesh: gltf_mesh.index(),
                primitive: primitive.index(),
            };
            let primitive_topology = get_primitive_topology(primitive.mode())?;

            let mut mesh = Mesh::new(primitive_topology);

            for (semantic, accessor) in primitive.attributes() {
                if semantic == Semantic::Positions {
                    assert_eq!(
                        accessor.dimensions(),
                        Dimensions::Vec3,
                        "Only vec3 position is supported"
                    );
                    assert_eq!(
                        accessor.data_type(),
                        DataType::F32,
                        "Only f32 positions are supported"
                    );
                    mesh.insert_positions(read_attributes(&accessor, &buffer_data));
                }
                if semantic == Semantic::Normals {
                    assert_eq!(
                        accessor.dimensions(),
                        Dimensions::Vec3,
                        "Only vec3 normals is supported"
                    );
                    assert_eq!(
                        accessor.data_type(),
                        DataType::F32,
                        "Only f32 normals are supported"
                    );
                    mesh.insert_normals(read_attributes(&accessor, &buffer_data));
                }
            }

            // Read vertex indices
            let reader = primitive.reader(|buffer| Some(buffer_data[buffer.index()].as_slice()));
            if let Some(indices) = reader.read_indices() {
                mesh.insert_indices(match indices {
                    ReadIndices::U8(is) => is.map(|x| x as u32).collect(),
                    ReadIndices::U16(is) => is.map(|x| x as u32).collect(),
                    ReadIndices::U32(is) => is.collect(),
                });
            };

            let mesh_handle = load_context.add_labeled_asset(primitive_label.to_string(), mesh);
            primitives.push(GltfPrimitive {
                index: primitive.index(),
                name: primitive_label.to_string(),
                mesh: mesh_handle,
                material: primitive
                    .material()
                    .index()
                    .map_or_else(Handle::default, |index| materials[index].clone()),
            });
        }

        let mesh = GltfMesh::new(&gltf_mesh, primitives);

        let handle = load_context.add_labeled_asset(mesh.asset_label().to_string(), mesh);
        meshes.push(handle);
    }

    let mut nodes = HashMap::<usize, Handle<GltfNode>>::new();
    for node in GltfTreeIterator::try_new(&gltf)? {
        let children = node
            .children()
            .map(|child| nodes.get(&child.index()).unwrap().clone())
            .collect();

        let mesh = node
            .mesh()
            .map(|mesh| mesh.index())
            .and_then(|i| meshes.get(i).cloned());

        let gltf_node = GltfNode::new(&node, children, mesh, node_transform(&node));

        let handle = load_context.add_labeled_asset(gltf_node.asset_label().to_string(), gltf_node);
        nodes.insert(node.index(), handle.clone());
    }

    let mut nodes_to_sort = nodes.into_iter().collect::<Vec<_>>();
    nodes_to_sort.sort_by_key(|(i, _)| *i);
    let nodes = nodes_to_sort
        .into_iter()
        .map(|(_, resolved)| resolved)
        .collect();

    let mut scenes = vec![];
    for scene in gltf.scenes() {
        let mut err = None;
        let mut world = World::default();
        let mut scene_load_context = load_context.begin_labeled_asset();

        world
            .spawn((Transform::IDENTITY, GlobalTransform::IDENTITY))
            .with_children(|parent| {
                for node in scene.nodes() {
                    let result = load_node(
                        &node,
                        parent,
                        &mut scene_load_context,
                        &Transform::default(),
                    );
                    if result.is_err() {
                        err = Some(result);
                        return;
                    }
                }
            });

        if let Some(Err(err)) = err {
            return Err(err);
        }

        let loaded_scene = scene_load_context.finish(Scene::new(world), None);

        let scene_label = GltfAssetLabel::Scene(scene.index()).to_string();
        let scene_handle = load_context.add_loaded_labeled_asset(scene_label, loaded_scene);

        scenes.push(scene_handle);
    }

    Ok(Gltf {
        default_scene: gltf
            .default_scene()
            .and_then(|scene| scenes.get(scene.index()))
            .cloned(),
        scenes,
        meshes,
        materials,
        nodes,
    })
}

async fn load_buffers(gltf: &gltf::Gltf) -> Result<Vec<Vec<u8>>, GltfError> {
    let mut buffer_data = Vec::new();
    for buffer in gltf.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Bin => {
                if let Some(blob) = gltf.blob.as_deref() {
                    buffer_data.push(blob.into());
                } else {
                    return Err(GltfError::MissingBlob);
                }
            }
            _ => {
                return Err(GltfError::UnsupportedBufferFormat(String::from("URI")));
            }
        }
    }

    Ok(buffer_data)
}

async fn load_image<'a, 'b>(
    gltf_texture: gltf::Texture<'a>,
    buffer_data: &[Vec<u8>],
) -> Result<(Image, String), GltfError> {
    match gltf_texture.source().source() {
        gltf::image::Source::View { view, mime_type } => {
            let start = view.offset();
            let end = view.offset() + view.length();
            let buffer = &buffer_data[view.buffer().index()][start..end];
            let Some(image_crate_format) = image::ImageFormat::from_mime_type(mime_type) else {
                warn!("Unsupported image mime type {}", mime_type);
                return Err(GltfError::UnsupportedImageFormat(mime_type.to_string()));
            };
            let mut reader = image::ImageReader::new(std::io::Cursor::new(buffer));
            reader.set_format(image_crate_format);
            reader.no_limits();
            match reader.decode() {
                Ok(image) => Ok((Image::from_dynamic(image), String::from("asd"))),
                Err(error) => Err(GltfError::ImageCrateError(error)),
            }
        }
        gltf::image::Source::Uri { .. } => {
            Err(GltfError::UnsupportedImageFormat(String::from("URI")))
        }
    }
}

#[allow(clippy::result_large_err)]
fn get_primitive_topology(mode: Mode) -> Result<D3D12_PRIMITIVE_TOPOLOGY_TYPE, GltfError> {
    match mode {
        Mode::Points => Ok(D3D12_PRIMITIVE_TOPOLOGY_TYPE_POINT),
        Mode::Lines => Ok(D3D12_PRIMITIVE_TOPOLOGY_TYPE_LINE),
        Mode::Triangles => Ok(D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE),
        mode => Err(GltfError::UnsupportedPrimitive { mode }),
    }
}

trait FromLeBytes: Sized {
    fn from_le_bytes(bytes: &[u8]) -> Self;
}

fn read_attributes<T, const N: usize>(accessor: &Accessor, data: &[Vec<u8>]) -> Vec<[T; N]>
where
    T: Copy + FromLeBytes + Default + num_traits::identities::Zero,
{
    let view = accessor.view().unwrap();
    let buffer = &data[view.buffer().index()];

    let start = view.offset();
    let end = start + view.length();

    let data = &buffer[start..end];
    let stride = view.stride().unwrap_or(12); // Vec3: 3 * 4 bytes = 12 bytes
    let count = accessor.count();

    let mut attributes = Vec::with_capacity(count);

    for i in 0..count {
        let offset = i * stride;
        let mut element = [T::zero(); N];

        (0..N).for_each(|j| {
            let component_offset = offset + j * mem::size_of::<T>();
            let bytes = &data[component_offset..component_offset + mem::size_of::<T>()];
            element[j] = T::from_le_bytes(bytes);
        });

        attributes.push(element);
    }

    attributes
}

impl FromLeBytes for f32 {
    fn from_le_bytes(bytes: &[u8]) -> Self {
        f32::from_le_bytes(bytes.try_into().expect("Invalid byte length for f32"))
    }
}

fn node_transform(node: &Node) -> Transform {
    match node.transform() {
        gltf::scene::Transform::Matrix { matrix } => {
            Transform::from_matrix(Mat4::from_cols_array_2d(&matrix))
        }
        gltf::scene::Transform::Decomposed {
            translation,
            rotation,
            scale,
        } => Transform {
            translation: Vec3::from(translation),
            rotation: Quat::from_array(rotation),
            scale: Vec3::from(scale),
        },
    }
}

#[allow(clippy::too_many_arguments, clippy::result_large_err)]
fn load_node(
    gltf_node: &Node,
    world_builder: &mut WorldChildBuilder,
    load_context: &mut LoadContext,
    parent_transform: &Transform,
) -> Result<(), GltfError> {
    let mut gltf_error = None;
    let transform = node_transform(gltf_node);
    let world_transform = *parent_transform * transform;
    let mut node = world_builder.spawn(transform);

    let name = node_name(gltf_node);
    node.insert(name.clone());

    node.with_children(|parent| {
        if let Some(mesh) = gltf_node.mesh() {
            for primitive in mesh.primitives() {
                let material = primitive.material();

                let primitive_label = GltfAssetLabel::Primitive {
                    mesh: mesh.index(),
                    primitive: primitive.index(),
                };
                let material_label = material
                    .index()
                    .map(|index| GltfAssetLabel::Material { index });

                let mesh_handle =
                    load_context.get_label_handle::<Mesh>(primitive_label.to_string());
                let material_handle = material_label.map_or(Handle::default(), |label| {
                    load_context.get_label_handle::<Material>(label.to_string())
                });
                parent.spawn((mesh_handle, material_handle));
            }
        }

        for child in gltf_node.children() {
            if let Err(err) = load_node(&child, parent, load_context, &world_transform) {
                gltf_error = Some(err);
                return;
            }
        }
    });

    if let Some(err) = gltf_error {
        Err(err)
    } else {
        Ok(())
    }
}

fn node_name(node: &Node) -> Name {
    let name = node
        .name()
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("GltfNode{}", node.index()));
    Name::new(name)
}
