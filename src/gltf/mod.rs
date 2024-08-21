mod loader;

use bevy::{asset::AssetPath, prelude::*, utils::HashMap};

use crate::{Material, Mesh};

pub struct GltfPlugin;

impl Plugin for GltfPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Gltf>()
            .init_asset::<GltfNode>()
            .init_asset::<GltfPrimitive>()
            .init_asset::<GltfMesh>()
            .preregister_asset_loader::<GltfLoader>(&["gltf", "glb"]);
    }

    fn finish(&self, app: &mut App) {
        app.register_asset_loader(GltfLoader);
    }
}

#[derive(Asset, Debug, TypePath)]
pub struct Gltf {
    pub scenes: Vec<Handle<Scene>>,
    pub named_scenes: HashMap<Box<str>, Handle<Scene>>,
    pub meshes: Vec<Handle<GltfMesh>>,
    pub named_meshes: HashMap<Box<str>, Handle<GltfMesh>>,
    pub materials: Vec<Handle<Material>>,
    pub named_materials: HashMap<Box<str>, Handle<Material>>,
    pub nodes: Vec<Handle<GltfNode>>,
    pub named_nodes: HashMap<Box<str>, Handle<GltfNode>>,
    pub default_scene: Option<Handle<Scene>>,
    pub source: Option<gltf::Gltf>,
}

#[derive(Asset, Debug, Clone, TypePath)]
pub struct GltfMesh {
    pub index: usize,
    pub name: String,
    pub primitives: Vec<GltfPrimitive>,
}

#[derive(Asset, Debug, Clone, TypePath)]
pub struct GltfNode {
    pub index: usize,
    pub name: String,
    pub children: Vec<Handle<GltfNode>>,
    pub mesh: Option<Handle<GltfMesh>>,
    pub transform: Transform,
}

impl GltfNode {
    pub fn new(
        node: &gltf::Node,
        children: Vec<Handle<GltfNode>>,
        mesh: Option<Handle<GltfMesh>>,
        transform: Transform,
    ) -> Self {
        Self {
            index: node.index(),
            name: if let Some(name) = node.name() {
                name.to_string()
            } else {
                format!("GltfNode{}", node.index())
            },
            children,
            mesh,
            transform,
        }
    }

    pub fn asset_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Node(self.index)
    }
}

#[derive(Asset, Debug, Clone, TypePath)]
pub struct GltfPrimitive {
    pub index: usize,
    pub parent_mesh_index: usize,
    pub name: String,
    pub mesh: Handle<Mesh>,
    pub material: Option<Handle<Material>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GltfAssetLabel {
    Scene(usize),
    Node(usize),
    Mesh(usize),
    Primitive {
        mesh: usize,
        primitive: usize,
    },
    Texture(usize),
    Material {
        index: usize,
        is_scale_inverted: bool,
    },
    DefaultMaterial,
}

impl std::fmt::Display for GltfAssetLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GltfAssetLabel::Scene(index) => f.write_str(&format!("Scene{index}")),
            GltfAssetLabel::Node(index) => f.write_str(&format!("Node{index}")),
            GltfAssetLabel::Mesh(index) => f.write_str(&format!("Mesh{index}")),
            GltfAssetLabel::Primitive { mesh, primitive } => {
                f.write_str(&format!("Mesh{mesh}/Primitive{primitive}"))
            }
            GltfAssetLabel::Texture(index) => f.write_str(&format!("Texture{index}")),
            GltfAssetLabel::Material {
                index,
                is_scale_inverted,
            } => f.write_str(&format!(
                "Material{index}{}",
                if *is_scale_inverted {
                    " (inverted)"
                } else {
                    ""
                }
            )),
            GltfAssetLabel::DefaultMaterial => f.write_str("DefaultMaterial"),
        }
    }
}

impl GltfAssetLabel {
    pub fn from_asset(&self, path: impl Into<AssetPath<'static>>) -> AssetPath<'static> {
        path.into().with_label(self.to_string())
    }
}

impl GltfMesh {
    pub fn new(mesh: &gltf::Mesh, primitives: Vec<GltfPrimitive>) -> Self {
        Self {
            index: mesh.index(),
            name: if let Some(name) = mesh.name() {
                name.to_string()
            } else {
                format!("GltfMesh{}", mesh.index())
            },
            primitives,
        }
    }

    pub fn asset_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Mesh(self.index)
    }
}
