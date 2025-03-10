use homotopy_common::tree::Tree;
use homotopy_core::{
    common::{Generator, SliceIndex},
    serialize::{Key, Store},
    Diagram,
};
use homotopy_graphics::style::Color;
use im::Vector;
use obake::AnyVersion;

use super::proof::{
    generators::GeneratorInfo, FolderInfo, Metadata, Signature, SignatureItem, View, Workspace,
};

#[obake::versioned]
#[obake(version("0.1.2"))]
#[obake(derive(serde::Serialize, serde::Deserialize))]
#[obake(serde(untagged))]
#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
struct WorkspaceData {
    diagram: Key<Diagram>,
    path: Vector<SliceIndex>,
    view: View,
}

#[obake::versioned]
#[obake(version("0.1.3"))]
#[obake(derive(serde::Serialize, serde::Deserialize))]
#[obake(serde(tag = "version"))]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Data {
    store: Store,
    signature: Tree<SignatureData>,
    workspace: Option<WorkspaceData>,
    metadata: Metadata,
}

impl std::fmt::Debug for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Data").finish()
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
enum SignatureData {
    Folder(String, bool),
    Item(GeneratorData),
}

impl Default for SignatureData {
    fn default() -> Self {
        Self::Folder(<_>::default(), true)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct GeneratorData {
    generator: Generator,
    name: String,
    color: Color,
    oriented: bool,
    invertible: bool,
    diagram: Key<Diagram>,
}

pub fn serialize(
    signature: Signature,
    workspace: Option<Workspace>,
    metadata: Metadata,
) -> Vec<u8> {
    let mut data = Data {
        store: <_>::default(),
        signature: <_>::default(),
        workspace: <_>::default(),
        metadata,
    };

    let mut signature = signature.into_tree();
    // Remove noise from signature tree
    signature.clean_up();
    // Pack signature data
    data.signature = signature.map(|item| match item {
        SignatureItem::Folder(info) => SignatureData::Folder(info.name, info.open),
        SignatureItem::Item(info) => SignatureData::Item(GeneratorData {
            generator: info.generator,
            diagram: data.store.pack_diagram(&info.diagram),
            name: info.name,
            color: info.color,
            oriented: info.oriented,
            invertible: info.invertible,
        }),
    });

    if let Some(workspace) = workspace {
        data.workspace = Some(WorkspaceData {
            diagram: data.store.pack_diagram(&workspace.diagram),
            path: workspace.path,
            view: workspace.view,
        });
    }

    // Tag data with version
    let data: AnyVersion<Data> = data.into();
    // Serialize
    rmp_serde::encode::to_vec_named(&data).unwrap()
}

pub fn deserialize(data: &[u8]) -> Option<((Signature, Option<Workspace>), Metadata)> {
    // Deserialize with version tag
    let data: AnyVersion<Data> = match rmp_serde::decode::from_slice(data) {
        Err(error) => {
            tracing::error!("Error while deserializing: {}", error);
            None
        }
        Ok(data) => Some(data),
    }?;
    // Migrate to current version
    let data: Data = data.into();
    let mut store = data.store;

    let mut folder_index = 0;
    let signature = data
        .signature
        .map(|s| {
            Some(match s {
                SignatureData::Folder(name, open) => {
                    folder_index += 1;
                    SignatureItem::Folder(FolderInfo {
                        id: folder_index,
                        name,
                        open: open.to_owned(),
                    })
                }
                SignatureData::Item(gd) => SignatureItem::Item(GeneratorInfo {
                    generator: gd.generator,
                    name: gd.name,
                    color: gd.color,
                    shape: <_>::default(),
                    diagram: store.unpack_diagram(gd.diagram)?,
                    oriented: gd.oriented,
                    invertible: gd.invertible,
                    // TODO: `single_preview` should be properly serialized
                    single_preview: true,
                }),
            })
        })
        .transpose()?
        .into();

    let mut workspace = None;
    if let Some(workspace_data) = data.workspace {
        workspace = Some(Workspace {
            diagram: store.unpack_diagram(workspace_data.diagram)?,
            path: workspace_data.path,
            view: workspace_data.view,
        });
    }
    Some(((signature, workspace), data.metadata))
}
