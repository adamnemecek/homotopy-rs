use std::{
    collections::HashMap,
    convert::TryInto,
    iter::FusedIterator,
    ops::{Deref, DerefMut},
};

use homotopy_common::{
    declare_idx,
    idx::{Idx, IdxVec},
};
use homotopy_core::DiagramN;
use ultraviolet::{Vec3, Vec4};

use crate::gl;

declare_idx! {
    pub struct Vertex = u16;
    pub struct Element = usize;
    pub struct Square = usize;
    pub struct Cube = usize;
}

pub type Dimension = u8;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Boundary {
    /// Corner - no freedom to move
    Zero = 0,
    /// Edge - free to move along line
    One = 1,
    /// Surface - free to move in space
    Two = 2,
    /// Volume - free to move in time and space
    Three = 3,
}

/// Represents a vertex in a 4-space
#[derive(Debug, Clone, PartialEq)]
pub struct VertexData {
    pub vertex: Vec4,
    pub boundary: Boundary,
    // generator: Generator
}

pub trait MeshData {
    type Idx: Idx;

    fn remap<T>(&mut self, remapper: &mut VertexRemapper<T>)
    where
        T: MeshData;
}

pub trait FromMesh<T>: MeshData + Sized
where
    T: MeshData,
{
    fn try_from(mesh: &Mesh<T>, element: T::Idx) -> Option<Self>;
}

impl<T> FromMesh<Self> for T
where
    T: MeshData + Clone,
{
    fn try_from(mesh: &Mesh<Self>, element: Self::Idx) -> Option<Self> {
        mesh.elements.get(element).cloned()
    }
}

/// Represents cubical surface elements (points, lines, squares, cubes, ...)
/// that have the cubical property (composed of exactly opposite subfaces).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ElementData {
    Cube0(Vertex),
    CubeN(CubeN),
}

impl MeshData for ElementData {
    type Idx = Element;

    fn remap<T>(&mut self, remapper: &mut VertexRemapper<T>)
    where
        T: MeshData,
    {
        match self {
            ElementData::Cube0(vertex) => *vertex = remapper.get(*vertex),
            ElementData::CubeN(_) => {}
        }
    }
}

/// Represents an n-cube by recording the two (n - 1)-cubes that make it and
/// its order.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CubeN {
    n: Dimension,
    subcube_0: Element,
    subcube_1: Element,
}

pub type SquareData = [Vertex; 4];
pub type CubeData = [Vertex; 8];

impl MeshData for SquareData {
    type Idx = Square;

    fn remap<T>(&mut self, remapper: &mut VertexRemapper<T>)
    where
        T: MeshData,
    {
        for v in self.iter_mut() {
            *v = remapper.get(*v);
        }
    }
}

impl MeshData for CubeData {
    type Idx = Cube;

    fn remap<T>(&mut self, remapper: &mut VertexRemapper<T>)
    where
        T: MeshData,
    {
        for v in self.iter_mut() {
            *v = remapper.get(*v);
        }
    }
}

impl FromMesh<ElementData> for SquareData {
    fn try_from(mesh: &Mesh<ElementData>, element: Element) -> Option<Self> {
        if mesh.order_of(element) == 2 {
            mesh.flatten(element).collect::<Vec<_>>().try_into().ok()
        } else {
            None
        }
    }
}

impl FromMesh<ElementData> for CubeData {
    fn try_from(mesh: &Mesh<ElementData>, element: Element) -> Option<Self> {
        if mesh.order_of(element) == 3 {
            mesh.flatten(element).collect::<Vec<_>>().try_into().ok()
        } else {
            None
        }
    }
}

/// Represents all cubical surface elements
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh<T = ElementData>
where
    T: MeshData,
{
    pub vertices: IdxVec<Vertex, VertexData>,
    pub elements: IdxVec<T::Idx, T>,
}

/// Represents concrete square mesh to be subdivided and rendered.
pub type SquareMesh = Mesh<SquareData>;
/// Represents concrete cube mesh to be subdivided and rendered.
pub type CubeMesh = Mesh<CubeData>;

impl Deref for VertexData {
    type Target = Vec4;

    fn deref(&self) -> &Self::Target {
        &self.vertex
    }
}

impl DerefMut for VertexData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vertex
    }
}

impl ElementData {
    fn order(&self) -> Dimension {
        match self {
            ElementData::Cube0(_) => 0,
            ElementData::CubeN(cube) => cube.n,
        }
    }
}

impl Mesh {
    pub fn mk_element_0(&mut self, vertex: Vertex) -> Element {
        self.elements.push(ElementData::Cube0(vertex))
    }

    pub fn mk_element_n(&mut self, subcube_0: Element, subcube_1: Element) -> Element {
        assert_eq!(self.order_of(subcube_0), self.order_of(subcube_1));

        self.elements.push(ElementData::CubeN(CubeN {
            n: self.order_of(subcube_0) + 1,
            subcube_0,
            subcube_1,
        }))
    }

    pub fn mk_element_from(&mut self, verts: &[Vertex]) -> Element {
        assert!(!verts.is_empty());

        match verts {
            [] => panic!(),
            [v] => self.mk_element_0(*v),
            verts => {
                let subcube_0 = self.mk_element_from(&verts[..verts.len() / 2]);
                let subcube_1 = self.mk_element_from(&verts[verts.len() / 2..]);
                self.mk_element_n(subcube_0, subcube_1)
            }
        }
    }

    pub fn flatten(&self, element: Element) -> impl Iterator<Item = Vertex> + '_ {
        Flattener {
            mesh: self,
            to_visit: vec![element],
        }
    }

    pub fn order_of(&self, element: Element) -> Dimension {
        self.elements[element].order()
    }

    #[allow(dead_code)]
    pub fn build(_diagram: &DiagramN) -> Self {
        Self::new()
    }
}

impl<T> Mesh<T>
where
    T: MeshData,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn mk_vertex(&mut self, vertex: VertexData) -> Vertex {
        self.vertices.push(vertex)
    }

    pub fn into<U>(self) -> Mesh<U>
    where
        U: FromMesh<T>,
    {
        let mut remapper = VertexRemapper::new(&self);
        let elements = self
            .elements
            .keys()
            .filter_map(|element| {
                let mut element = U::try_from(&self, element)?;
                remapper.remap(&mut element);
                Some(element)
            })
            .collect();

        Mesh {
            vertices: remapper.into_verts(),
            elements,
        }
    }
}

pub struct SquareMeshBuffers {
    pub element_buffer: gl::buffer::ElementBuffer,
    pub vertex_buffer: gl::buffer::Buffer<Vec3>,
    pub normal_buffer: gl::buffer::Buffer<Vec3>,
}

impl SquareMesh {
    pub fn mk_square(&mut self, square: SquareData) -> Square {
        self.elements.push(square)
    }

    pub fn buffer(
        &self,
        ctx: &gl::GlCtx,
        kind: gl::buffer::ElementKind,
    ) -> gl::Result<SquareMeshBuffers> {
        let vertices = self
            .vertices
            .values()
            .map(|v| v.xyz())
            .collect::<IdxVec<_, _>>();
        let mut elements = Vec::with_capacity(self.elements.len() * 6);
        let mut normals = IdxVec::splat(Vec3::zero(), vertices.len());

        {
            let mut push_element = |i: Vertex, j: Vertex, k: Vertex| {
                match kind {
                    gl::buffer::ElementKind::Lines => {
                        elements.push(i.index() as u16);
                        elements.push(j.index() as u16);
                        elements.push(j.index() as u16);
                        elements.push(k.index() as u16);
                        elements.push(k.index() as u16);
                        elements.push(i.index() as u16);
                    }
                    gl::buffer::ElementKind::Triangles => {
                        elements.push(i.index() as u16);
                        elements.push(j.index() as u16);
                        elements.push(k.index() as u16);
                    }
                };
            };

            let mut push_tri = |i: Vertex, j: Vertex, k: Vertex| {
                if i != j && j != k && k != i {
                    push_element(i, j, k);

                    let a = vertices[i];
                    let b = vertices[j];
                    let c = vertices[k];
                    let n = (b - a).cross(c - a);

                    normals[i] += n;
                    normals[j] += n;
                    normals[k] += n;
                }
            };

            // Triangulate mesh
            for square in self.elements.values() {
                // Bottom right triangle
                push_tri(square[0], square[1], square[3]);
                // Top left triangle
                push_tri(square[0], square[3], square[2]);
            }
        }

        // Average normals
        for normal in normals.values_mut() {
            normal.normalize();
        }

        // Buffer data
        let element_buffer = ctx.mk_element_buffer(&elements, kind)?;
        let vertex_buffer = ctx.mk_buffer(&vertices.into_raw())?;
        let normal_buffer = ctx.mk_buffer(&normals.into_raw())?;

        Ok(SquareMeshBuffers {
            element_buffer,
            vertex_buffer,
            normal_buffer,
        })
    }
}

impl CubeMesh {
    pub fn mk_cube(&mut self, cube: CubeData) -> Cube {
        self.elements.push(cube)
    }
}

impl<T> Default for Mesh<T>
where
    T: MeshData,
{
    fn default() -> Self {
        Self {
            vertices: Default::default(),
            elements: Default::default(),
        }
    }
}

pub struct Flattener<'a> {
    mesh: &'a Mesh,
    to_visit: Vec<Element>,
}

impl<'a> Iterator for Flattener<'a> {
    type Item = Vertex;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let element = self.to_visit.pop()?;
            match &self.mesh.elements[element] {
                ElementData::Cube0(vertex) => return Some(*vertex),
                ElementData::CubeN(cube) => {
                    self.to_visit.push(cube.subcube_1);
                    self.to_visit.push(cube.subcube_0);
                }
            }
        }
    }
}

impl<'a> FusedIterator for Flattener<'a> {}

pub struct VertexRemapper<'a, T>
where
    T: MeshData,
{
    mesh: &'a Mesh<T>,
    remapping: HashMap<Vertex, Vertex>,
    data: IdxVec<Vertex, VertexData>,
}

impl<'a, T> VertexRemapper<'a, T>
where
    T: MeshData,
{
    fn new(mesh: &'a Mesh<T>) -> Self {
        Self {
            mesh,
            remapping: Default::default(),
            data: Default::default(),
        }
    }

    fn get(&mut self, unmapped: Vertex) -> Vertex {
        if let Some(vertex) = self.remapping.get(&unmapped) {
            return *vertex;
        }

        let vertex = self.data.push(self.mesh.vertices[unmapped].clone());

        self.remapping.insert(unmapped, vertex);
        vertex
    }

    fn remap<U>(&mut self, data: &mut U)
    where
        U: MeshData,
    {
        U::remap(data, self);
    }

    fn into_verts(self) -> IdxVec<Vertex, VertexData> {
        self.data
    }
}

pub trait VertexExt {
    fn with_boundary(self, boundary: Boundary) -> VertexData;
}

impl VertexExt for Vec4 {
    fn with_boundary(self, boundary: Boundary) -> VertexData {
        VertexData {
            vertex: self,
            boundary,
        }
    }
}

// TODO(@doctorn) refactor
/*
impl CubeMesh {
    pub fn new() -> Self {
        Self {
            vertices: IdxVec::new(),
            cubes: IdxVec::new(),
            division_memory: HashMap::new(),
        }
    }

    fn create_new(&mut self, verts: &[VertexId]) -> VertexId {
        let vertices: Vec<&Vertex> = verts
            .iter()
            .map(|v_id| self.vertices.get(*v_id).unwrap())
            .collect();
        let first_bound = vertices[0].boundary;
        let mut bound = vertices.iter().fold(first_bound, |a, v| max(a, v.boundary));
        bound = max(
            bound,
            match verts.len() {
                2 => 1,
                4 => 2,
                _ => panic!(),
            },
        );

        let mut new_vert = Vertex::new(0.0, 0.0, 0.0, 0.0, bound);
        let scale = 1.0
            / match vertices.len() {
                2 => 2.0,
                4 => 4.0,
                _ => panic!("Unexpected number of vertices"),
            };
        for v in vertices {
            new_vert.add_scaled(v, scale);
        }
        let v_id = self.vertices.push(new_vert);
        self.division_memory.insert(verts.to_owned(), v_id);
        v_id
    }

    /// Returns a VertexId that coresponds to the average of the suplied vertices.
    pub fn linearly_divide(&mut self, mut verts: Vec<VertexId>) -> VertexId {
        verts.sort();
        let mut c = verts.clone();
        c.dedup();
        match (verts.len(), c.len()) {
            (2 | 4, 1) => c[0],
            (2, 2) | (4, 4 | 3) => self
                .division_memory
                .get(&verts)
                .copied()
                .unwrap_or_else(|| self.create_new(&verts)),
            (4, 2) => self.linearly_divide(c),
            _ => panic!(),
        }
    }

    pub fn mk_vertex(&mut self, vertex: Vertex) -> VertexId {
        self.vertices.push(vertex)
    }

    pub fn mk_cube(&mut self, vertices: [VertexId; 8]) -> CubeId {
        self.cubes.push(vertices)
    }
}
*/
