use std::{io::Error, path::PathBuf};

use bevy::{
    asset::{io::Reader, AssetLoadError, AssetLoader, Handle, LoadContext, ReadAssetBytesError},
    utils::HashSet,
};

use gltf::mesh::Mode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{gltf::Gltf, Image};

use super::GltfAssetLabel;

pub struct GltfLoader;

#[derive(Serialize, Deserialize, Default)]
pub struct GltfLoaderSettings;

#[derive(Error, Debug)]
pub enum GltfError {
    #[error("unsupported primitive mode")]
    UnsupportedPrimitive { mode: Mode },
    #[error("invalid glTF file: {0}")]
    Gltf(#[from] gltf::Error),
    #[error("binary blob is missing")]
    MissingBlob,
    #[error("failed to decode base64 mesh data")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("unsupported buffer format")]
    BufferFormatUnsupported,
    #[error("invalid image mime type: {0}")]
    InvalidImageMimeType(String),
    #[error("failed to read bytes from an asset path: {0}")]
    ReadAssetBytesError(#[from] ReadAssetBytesError),
    #[error("failed to load asset from an asset path: {0}")]
    AssetLoadError(#[from] AssetLoadError),
    #[error("Missing sampler for animation {0}")]
    MissingAnimationSampler(usize),
    #[error("GLTF model must be a tree, found cycle instead at node indices: {0:?}")]
    CircularChildren(String),
    #[error("failed to load file: {0}")]
    Io(#[from] std::io::Error),
}

enum ImageOrPath {
    Image {
        image: Image,
        label: GltfAssetLabel,
    },
    Path {
        path: PathBuf,
        is_srgb: bool,
        // sampler_descriptor: ImageSamplerDescriptor,
    },
}

impl AssetLoader for GltfLoader {
    type Asset = Gltf;
    type Settings = GltfLoaderSettings;
    type Error = GltfError;
    async fn load<'a>(
        &'a self,
        reader: &'a mut dyn Reader,
        settings: &'a GltfLoaderSettings,
        load_context: &'a mut LoadContext<'_>,
    ) -> Result<Gltf, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        load_gltf(self, &bytes, load_context, settings).await
    }

    fn extensions(&self) -> &[&str] {
        &["gltf", "glb"]
    }
}

async fn load_gltf<'a, 'b, 'c>(
    loader: &GltfLoader,
    bytes: &'a [u8],
    load_context: &'b mut LoadContext<'c>,
    settings: &'b GltfLoaderSettings,
) -> Result<Gltf, GltfError> {
    let gltf = gltf::Gltf::from_slice(bytes)?;
    let file_name = load_context
        .asset_path()
        .path()
        .to_str()
        .ok_or(GltfError::Gltf(gltf::Error::Io(Error::new(
            std::io::ErrorKind::InvalidInput,
            "Gltf file name invalid",
        ))))?
        .to_string();
    let buffer_data = load_buffers(&gltf, load_context).await?;

    let mut linear_textures = HashSet::default();

    for material in gltf.materials() {
        if let Some(texture) = material.normal_texture() {
            linear_textures.insert(texture.texture().index());
        }
        if let Some(texture) = material.occlusion_texture() {
            linear_textures.insert(texture.texture().index());
        }
        if let Some(texture) = material
            .pbr_metallic_roughness()
            .metallic_roughness_texture()
        {
            linear_textures.insert(texture.texture().index());
        }
    }

    fn process_loaded_texture(
        load_context: &mut LoadContext,
        handles: &mut Vec<Handle<Image>>,
        texture: ImageOrPath,
    ) {
        let handle = match texture {
            ImageOrPath::Image { label, image } => {
                load_context.add_labeled_asset(label.to_string(), image)
            }
            ImageOrPath::Path {
                path,
                is_srgb,
                sampler_descriptor,
            } => load_context
                .loader()
                .with_settings(move |settings: &mut ImageLoaderSettings| {
                    settings.is_srgb = is_srgb;
                    settings.sampler = ImageSampler::Descriptor(sampler_descriptor.clone());
                })
                .load(path),
        };
        handles.push(handle);
    }

    // We collect handles to ensure loaded images from paths are not unloaded before they are used elsewhere
    // in the loader. This prevents "reloads", but it also prevents dropping the is_srgb context on reload.
    //
    // In theory we could store a mapping between texture.index() and handle to use
    // later in the loader when looking up handles for materials. However this would mean
    // that the material's load context would no longer track those images as dependencies.
    let mut _texture_handles = Vec::new();
    if gltf.textures().len() == 1 || cfg!(target_arch = "wasm32") {
        for texture in gltf.textures() {
            let parent_path = load_context.path().parent().unwrap();
            let image = load_image(
                texture,
                &buffer_data,
                &linear_textures,
                parent_path,
                loader.supported_compressed_formats,
                settings.load_materials,
            )
            .await?;
            process_loaded_texture(load_context, &mut _texture_handles, image);
        }
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        IoTaskPool::get()
            .scope(|scope| {
                gltf.textures().for_each(|gltf_texture| {
                    let parent_path = load_context.path().parent().unwrap();
                    let linear_textures = &linear_textures;
                    let buffer_data = &buffer_data;
                    scope.spawn(async move {
                        load_image(
                            gltf_texture,
                            buffer_data,
                            linear_textures,
                            parent_path,
                            loader.supported_compressed_formats,
                            settings.load_materials,
                        )
                        .await
                    });
                });
            })
            .into_iter()
            .for_each(|result| match result {
                Ok(image) => {
                    process_loaded_texture(load_context, &mut _texture_handles, image);
                }
                Err(err) => {
                    warn!("Error loading glTF texture: {}", err);
                }
            });
    }

    let mut materials = vec![];
    let mut named_materials = HashMap::default();
    // Only include materials in the output if they're set to be retained in the MAIN_WORLD and/or RENDER_WORLD by the load_materials flag
    if !settings.load_materials.is_empty() {
        // NOTE: materials must be loaded after textures because image load() calls will happen before load_with_settings, preventing is_srgb from being set properly
        for material in gltf.materials() {
            let handle = load_material(&material, load_context, &gltf.document, false);
            if let Some(name) = material.name() {
                named_materials.insert(name.into(), handle.clone());
            }
            materials.push(handle);
        }
    }
    let mut meshes = vec![];
    let mut named_meshes = HashMap::default();
    let mut meshes_on_skinned_nodes = HashSet::default();
    let mut meshes_on_non_skinned_nodes = HashSet::default();
    for gltf_node in gltf.nodes() {
        if gltf_node.skin().is_some() {
            if let Some(mesh) = gltf_node.mesh() {
                meshes_on_skinned_nodes.insert(mesh.index());
            }
        } else if let Some(mesh) = gltf_node.mesh() {
            meshes_on_non_skinned_nodes.insert(mesh.index());
        }
    }
    for gltf_mesh in gltf.meshes() {
        let mut primitives = vec![];
        for primitive in gltf_mesh.primitives() {
            let primitive_label = GltfAssetLabel::Primitive {
                mesh: gltf_mesh.index(),
                primitive: primitive.index(),
            };
            let primitive_topology = get_primitive_topology(primitive.mode())?;

            let mut mesh = Mesh::new(primitive_topology, settings.load_meshes);

            // Read vertex attributes
            for (semantic, accessor) in primitive.attributes() {
                if [Semantic::Joints(0), Semantic::Weights(0)].contains(&semantic) {
                    if !meshes_on_skinned_nodes.contains(&gltf_mesh.index()) {
                        warn!(
                        "Ignoring attribute {:?} for skinned mesh {:?} used on non skinned nodes (NODE_SKINNED_MESH_WITHOUT_SKIN)",
                        semantic,
                        primitive_label
                    );
                        continue;
                    } else if meshes_on_non_skinned_nodes.contains(&gltf_mesh.index()) {
                        error!("Skinned mesh {:?} used on both skinned and non skin nodes, this is likely to cause an error (NODE_SKINNED_MESH_WITHOUT_SKIN)", primitive_label);
                    }
                }
                match convert_attribute(
                    semantic,
                    accessor,
                    &buffer_data,
                    &loader.custom_vertex_attributes,
                ) {
                    Ok((attribute, values)) => mesh.insert_attribute(attribute, values),
                    Err(err) => warn!("{}", err),
                }
            }

            // Read vertex indices
            let reader = primitive.reader(|buffer| Some(buffer_data[buffer.index()].as_slice()));
            if let Some(indices) = reader.read_indices() {
                mesh.insert_indices(match indices {
                    ReadIndices::U8(is) => Indices::U16(is.map(|x| x as u16).collect()),
                    ReadIndices::U16(is) => Indices::U16(is.collect()),
                    ReadIndices::U32(is) => Indices::U32(is.collect()),
                });
            };

            {
                let morph_target_reader = reader.read_morph_targets();
                if morph_target_reader.len() != 0 {
                    let morph_targets_label = GltfAssetLabel::MorphTarget {
                        mesh: gltf_mesh.index(),
                        primitive: primitive.index(),
                    };
                    let morph_target_image = MorphTargetImage::new(
                        morph_target_reader.map(PrimitiveMorphAttributesIter),
                        mesh.count_vertices(),
                        RenderAssetUsages::default(),
                    )?;
                    let handle = load_context
                        .add_labeled_asset(morph_targets_label.to_string(), morph_target_image.0);

                    mesh.set_morph_targets(handle);
                    let extras = gltf_mesh.extras().as_ref();
                    if let Some(names) = extras.and_then(|extras| {
                        serde_json::from_str::<MorphTargetNames>(extras.get()).ok()
                    }) {
                        mesh.set_morph_target_names(names.target_names);
                    }
                }
            }

            if mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_none()
                && matches!(mesh.primitive_topology(), PrimitiveTopology::TriangleList)
            {
                bevy_utils::tracing::debug!(
                    "Automatically calculating missing vertex normals for geometry."
                );
                let vertex_count_before = mesh.count_vertices();
                mesh.duplicate_vertices();
                mesh.compute_flat_normals();
                let vertex_count_after = mesh.count_vertices();
                if vertex_count_before != vertex_count_after {
                    bevy_utils::tracing::debug!("Missing vertex normals in indexed geometry, computing them as flat. Vertex count increased from {} to {}", vertex_count_before, vertex_count_after);
                } else {
                    bevy_utils::tracing::debug!(
                        "Missing vertex normals in indexed geometry, computing them as flat."
                    );
                }
            }

            if let Some(vertex_attribute) = reader
                .read_tangents()
                .map(|v| VertexAttributeValues::Float32x4(v.collect()))
            {
                mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, vertex_attribute);
            } else if mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some()
                && material_needs_tangents(&primitive.material())
            {
                bevy_utils::tracing::debug!(
                    "Missing vertex tangents for {}, computing them using the mikktspace algorithm. Consider using a tool such as Blender to pre-compute the tangents.", file_name
                );

                let generate_tangents_span = info_span!("generate_tangents", name = file_name);

                generate_tangents_span.in_scope(|| {
                    if let Err(err) = mesh.generate_tangents() {
                        warn!(
                        "Failed to generate vertex tangents using the mikktspace algorithm: {:?}",
                        err
                    );
                    }
                });
            }

            let mesh_handle = load_context.add_labeled_asset(primitive_label.to_string(), mesh);
            primitives.push(super::GltfPrimitive::new(
                &gltf_mesh,
                &primitive,
                mesh_handle,
                primitive
                    .material()
                    .index()
                    .and_then(|i| materials.get(i).cloned()),
                get_gltf_extras(primitive.extras()),
                get_gltf_extras(primitive.material().extras()),
            ));
        }

        let mesh =
            super::GltfMesh::new(&gltf_mesh, primitives, get_gltf_extras(gltf_mesh.extras()));

        let handle = load_context.add_labeled_asset(mesh.asset_label().to_string(), mesh);
        if let Some(name) = gltf_mesh.name() {
            named_meshes.insert(name.into(), handle.clone());
        }
        meshes.push(handle);
    }

    let skinned_mesh_inverse_bindposes: Vec<_> = gltf
        .skins()
        .map(|gltf_skin| {
            let reader = gltf_skin.reader(|buffer| Some(&buffer_data[buffer.index()]));
            let local_to_bone_bind_matrices: Vec<Mat4> = reader
                .read_inverse_bind_matrices()
                .unwrap()
                .map(|mat| Mat4::from_cols_array_2d(&mat))
                .collect();

            load_context.add_labeled_asset(
                inverse_bind_matrices_label(&gltf_skin),
                SkinnedMeshInverseBindposes::from(local_to_bone_bind_matrices),
            )
        })
        .collect();

    let mut nodes = HashMap::<usize, Handle<GltfNode>>::new();
    let mut named_nodes = HashMap::new();
    let mut skins = vec![];
    let mut named_skins = HashMap::default();
    for node in GltfTreeIterator::try_new(&gltf)? {
        let skin = node.skin().map(|skin| {
            let joints = skin
                .joints()
                .map(|joint| nodes.get(&joint.index()).unwrap().clone())
                .collect();

            let gltf_skin = GltfSkin::new(
                &skin,
                joints,
                skinned_mesh_inverse_bindposes[skin.index()].clone(),
                get_gltf_extras(skin.extras()),
            );

            let handle = load_context.add_labeled_asset(skin_label(&skin), gltf_skin);

            skins.push(handle.clone());
            if let Some(name) = skin.name() {
                named_skins.insert(name.into(), handle.clone());
            }

            handle
        });

        let children = node
            .children()
            .map(|child| nodes.get(&child.index()).unwrap().clone())
            .collect();

        let mesh = node
            .mesh()
            .map(|mesh| mesh.index())
            .and_then(|i| meshes.get(i).cloned());

        let gltf_node = GltfNode::new(
            &node,
            children,
            mesh,
            node_transform(&node),
            skin,
            get_gltf_extras(node.extras()),
        );

        #[cfg(feature = "bevy_animation")]
        let gltf_node = gltf_node.with_animation_root(animation_roots.contains(&node.index()));

        let handle = load_context.add_labeled_asset(gltf_node.asset_label().to_string(), gltf_node);
        nodes.insert(node.index(), handle.clone());
        if let Some(name) = node.name() {
            named_nodes.insert(name.into(), handle);
        }
    }

    let mut nodes_to_sort = nodes.into_iter().collect::<Vec<_>>();
    nodes_to_sort.sort_by_key(|(i, _)| *i);
    let nodes = nodes_to_sort
        .into_iter()
        .map(|(_, resolved)| resolved)
        .collect();

    let mut scenes = vec![];
    let mut named_scenes = HashMap::default();
    let mut active_camera_found = false;
    for scene in gltf.scenes() {
        let mut err = None;
        let mut world = World::default();
        let mut node_index_to_entity_map = HashMap::new();
        let mut entity_to_skin_index_map = EntityHashMap::default();
        let mut scene_load_context = load_context.begin_labeled_asset();

        let world_root_id = world
            .spawn(SpatialBundle::INHERITED_IDENTITY)
            .with_children(|parent| {
                for node in scene.nodes() {
                    let result = load_node(
                        &node,
                        parent,
                        load_context,
                        &mut scene_load_context,
                        settings,
                        &mut node_index_to_entity_map,
                        &mut entity_to_skin_index_map,
                        &mut active_camera_found,
                        &Transform::default(),
                        #[cfg(feature = "bevy_animation")]
                        &animation_roots,
                        #[cfg(feature = "bevy_animation")]
                        None,
                        &gltf.document,
                    );
                    if result.is_err() {
                        err = Some(result);
                        return;
                    }
                }
            })
            .id();

        if let Some(extras) = scene.extras().as_ref() {
            world.entity_mut(world_root_id).insert(GltfSceneExtras {
                value: extras.get().to_string(),
            });
        }

        if let Some(Err(err)) = err {
            return Err(err);
        }

        #[cfg(feature = "bevy_animation")]
        {
            // for each node root in a scene, check if it's the root of an animation
            // if it is, add the AnimationPlayer component
            for node in scene.nodes() {
                if animation_roots.contains(&node.index()) {
                    world
                        .entity_mut(*node_index_to_entity_map.get(&node.index()).unwrap())
                        .insert(bevy_animation::AnimationPlayer::default());
                }
            }
        }

        for (&entity, &skin_index) in &entity_to_skin_index_map {
            let mut entity = world.entity_mut(entity);
            let skin = gltf.skins().nth(skin_index).unwrap();
            let joint_entities: Vec<_> = skin
                .joints()
                .map(|node| node_index_to_entity_map[&node.index()])
                .collect();

            entity.insert(SkinnedMesh {
                inverse_bindposes: skinned_mesh_inverse_bindposes[skin_index].clone(),
                joints: joint_entities,
            });
        }
        let loaded_scene = scene_load_context.finish(Scene::new(world), None);
        let scene_handle = load_context.add_loaded_labeled_asset(scene_label(&scene), loaded_scene);

        if let Some(name) = scene.name() {
            named_scenes.insert(name.into(), scene_handle.clone());
        }
        scenes.push(scene_handle);
    }

    Ok(Gltf {
        default_scene: gltf
            .default_scene()
            .and_then(|scene| scenes.get(scene.index()))
            .cloned(),
        scenes,
        named_scenes,
        meshes,
        named_meshes,
        skins,
        named_skins,
        materials,
        named_materials,
        nodes,
        named_nodes,
        #[cfg(feature = "bevy_animation")]
        animations,
        #[cfg(feature = "bevy_animation")]
        named_animations,
        source: if settings.include_source {
            Some(gltf)
        } else {
            None
        },
    })
}

async fn load_buffers(
    gltf: &gltf::Gltf,
    load_context: &mut LoadContext<'_>,
) -> Result<Vec<Vec<u8>>, GltfError> {
    const VALID_MIME_TYPES: &[&str] = &["application/octet-stream", "application/gltf-buffer"];

    let mut buffer_data = Vec::new();
    for buffer in gltf.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Uri(uri) => {
                let uri = percent_encoding::percent_decode_str(uri)
                    .decode_utf8()
                    .unwrap();
                let uri = uri.as_ref();
                let buffer_bytes = match DataUri::parse(uri) {
                    Ok(data_uri) if VALID_MIME_TYPES.contains(&data_uri.mime_type) => {
                        data_uri.decode()?
                    }
                    Ok(_) => return Err(GltfError::BufferFormatUnsupported),
                    Err(()) => {
                        // TODO: Remove this and add dep
                        let buffer_path = load_context.path().parent().unwrap().join(uri);
                        load_context.read_asset_bytes(buffer_path).await?
                    }
                };
                buffer_data.push(buffer_bytes);
            }
            gltf::buffer::Source::Bin => {
                if let Some(blob) = gltf.blob.as_deref() {
                    buffer_data.push(blob.into());
                } else {
                    return Err(GltfError::MissingBlob);
                }
            }
        }
    }

    Ok(buffer_data)
}
