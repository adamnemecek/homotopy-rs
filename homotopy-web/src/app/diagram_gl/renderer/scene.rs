use std::{mem, rc::Rc};

use homotopy_common::idx::IdxVec;
use homotopy_core::{Diagram, Diagram0, Generator};
use homotopy_gl::{array::VertexArray, vertex_array, GlCtx, Result};
use homotopy_graphics::{
    geom::{CubicalGeometry, SimplicialGeometry, VertData},
    style::{GeneratorStyle, SignatureStyleData, VertexShape},
};
use ultraviolet::{Vec3, Vec4};

use super::buffers::{
    buffer_cylinder_wireframe, buffer_projected_wireframe, buffer_tetras, buffer_tris,
};
use crate::model::proof::View;

pub struct Scene {
    pub diagram: Diagram,
    pub view: View,
    pub components: Vec<Component<VertexArray>>,
    pub wireframe_components: Vec<VertexArray>,
    pub cylinder_components: Vec<Component<VertexArray>>,
    pub animation_curves: Vec<AnimationCurve>,
    pub animation_singularities: Vec<Component<Vec4>>,
    pub sphere: Option<Rc<VertexArray>>,
    pub cube: Option<Rc<VertexArray>>,
    pub duration: f32,
}

pub struct Component<V> {
    pub generator: Diagram0,
    pub vertices: V,
    pub albedo: Vec3,
    pub vertex_shape: Option<Rc<VertexArray>>,
}

pub struct AnimationCurve {
    pub generator: Diagram0,
    pub begin: f32,
    pub end: f32,
    pub key_frames: Vec<Vec4>,
    pub albedo: Vec3,
    pub vertex_shape: Option<Rc<VertexArray>>,
}

impl AnimationCurve {
    pub fn at(&self, t: f32) -> Option<Vec4> {
        if t < self.begin || t > self.end {
            return None;
        }

        let (start, end) = self.search(t)?;
        let lerp = (t - start.w) / (end.w - start.w);

        Some(start + (end - start) * lerp)
    }

    fn search(&self, t: f32) -> Option<(Vec4, Vec4)> {
        // TODO(@doctorn) a proper search algorithm
        self.key_frames
            .iter()
            .copied()
            .zip(self.key_frames.iter().copied().skip(1))
            .find(|(u, v)| u.w <= t && v.w >= t)
    }
}

impl Scene {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ctx: &GlCtx,
        diagram: &Diagram,
        view: View,
        animated_3d: bool,
        cubical_subdivision: bool,
        smooth_time: bool,
        subdivision_depth: u8,
        geometry_samples: u8,
        signature_styles: &impl SignatureStyleData,
    ) -> Result<Self> {
        let diagram = diagram.clone();

        let mut scene = Self {
            diagram,
            view,
            components: vec![],
            wireframe_components: vec![],
            cylinder_components: vec![],
            animation_curves: vec![],
            animation_singularities: vec![],
            sphere: None,
            cube: None,
            duration: 0.,
        };

        scene.reload_meshes(
            ctx,
            animated_3d,
            cubical_subdivision,
            smooth_time,
            subdivision_depth,
            geometry_samples,
            signature_styles,
        )?;
        Ok(scene)
    }

    #[allow(clippy::many_single_char_names)]
    #[allow(clippy::too_many_arguments)]
    pub fn reload_meshes(
        &mut self,
        ctx: &GlCtx,
        animated_3d: bool,
        cubical_subdivision: bool,
        smooth_time: bool,
        subdivision_depth: u8,
        geometry_samples: u8,
        signature_styles: &impl SignatureStyleData,
    ) -> Result<()> {
        self.components.clear();
        self.wireframe_components.clear();
        self.cylinder_components.clear();
        self.animation_curves.clear();
        self.animation_singularities.clear();
        self.sphere = None;
        self.cube = None;

        let mut sphere_mesh = SimplicialGeometry::default();
        let p = sphere_mesh.mk_vert(VertData {
            position: Vec4::zero(),
            boundary: [false; 4],
            generator: Generator::new(0, 0).into(),
            k: usize::MAX,
        });
        sphere_mesh.mk_point(p);
        sphere_mesh.inflate_point_3d(p, geometry_samples, &VertexShape::Circle);
        if let Some(sphere_buffers) = buffer_tris(&sphere_mesh, ctx)?.into_iter().next() {
            self.sphere = Some(Rc::new(vertex_array!(
                ctx,
                &sphere_buffers.element_buffer,
                [&sphere_buffers.vertex_buffer, &sphere_buffers.normal_buffer]
            )?));
        }

        let mut cube_mesh = SimplicialGeometry::default();
        let p = cube_mesh.mk_vert(VertData {
            position: Vec4::zero(),
            boundary: [false; 4],
            generator: Generator::new(0, 0).into(),
            k: usize::MAX,
        });
        cube_mesh.mk_point(p);
        cube_mesh.inflate_point_3d(p, geometry_samples, &VertexShape::Square);
        if let Some(cube_buffers) = buffer_tris(&cube_mesh, ctx)?.into_iter().next() {
            self.cube = Some(Rc::new(vertex_array!(
                ctx,
                &cube_buffers.element_buffer,
                [&cube_buffers.vertex_buffer, &cube_buffers.normal_buffer]
            )?));
        }

        let n = self.view.dimension();
        let animated = n == 4 || n == 3 && animated_3d;

        let mut cubical = match self.view.dimension() {
            0 => CubicalGeometry::new::<0>(&self.diagram, animated).unwrap(),
            1 => CubicalGeometry::new::<1>(&self.diagram, animated).unwrap(),
            2 => CubicalGeometry::new::<2>(&self.diagram, animated).unwrap(),
            3 => CubicalGeometry::new::<3>(&self.diagram, animated).unwrap(),
            4 => CubicalGeometry::new::<4>(&self.diagram, animated).unwrap(),
            _ => unreachable!(),
        };

        if cubical_subdivision {
            cubical.subdivide(smooth_time, subdivision_depth);
        }

        let mut simplicial = SimplicialGeometry::from(cubical);

        if !cubical_subdivision {
            simplicial.subdivide(smooth_time, subdivision_depth);
        }

        let color_of = |diagram: Diagram0, k: usize| -> Vec3 {
            let d = self.diagram.dimension();
            let n = diagram.generator.dimension;
            let c = d.saturating_sub(n + k);
            signature_styles
                .generator_style(diagram.generator)
                .unwrap()
                .color()
                .lighten(c, diagram.orientation)
                .into_linear_f32_components()
                .into()
        };
        let shape_of = |diagram: Diagram0| -> Option<Rc<VertexArray>> {
            match signature_styles
                .generator_style(diagram.generator)
                .unwrap()
                .shape()
            {
                VertexShape::Circle => self.sphere.as_ref().map(Rc::clone),
                VertexShape::Square => self.cube.as_ref().map(Rc::clone),
            }
        };

        if animated {
            for tetra_buffers in buffer_tetras(&simplicial, ctx)? {
                let generator = tetra_buffers.generator;
                self.components.push(Component {
                    generator,
                    vertices: vertex_array!(
                        ctx,
                        &tetra_buffers.element_buffer,
                        [
                            &tetra_buffers.vert_start_buffer,
                            &tetra_buffers.vert_end_buffer,
                            &tetra_buffers.normal_start_buffer,
                            &tetra_buffers.normal_end_buffer,
                        ]
                    )?,
                    albedo: color_of(generator, tetra_buffers.k),
                    vertex_shape: shape_of(generator),
                });
            }

            for projected_buffers in buffer_projected_wireframe(&simplicial, ctx)? {
                self.wireframe_components.push(vertex_array!(
                    ctx,
                    &projected_buffers.element_buffer,
                    [&projected_buffers.vert_buffer]
                )?);
            }

            for cylinder_buffers in buffer_cylinder_wireframe(&simplicial, ctx)? {
                let generator = cylinder_buffers.generator;
                self.cylinder_components.push(Component {
                    generator,
                    vertices: vertex_array!(
                        ctx,
                        &cylinder_buffers.element_buffer,
                        [
                            &cylinder_buffers.vert_start_buffer,
                            &cylinder_buffers.vert_end_buffer
                        ]
                    )?,
                    albedo: color_of(generator, 1),
                    vertex_shape: shape_of(generator),
                });
            }

            let mut curves = IdxVec::new();
            mem::swap(&mut simplicial.curves, &mut curves);

            for mut curve in curves.into_values() {
                if curve.verts.len() < 2 {
                    continue;
                }

                let generator = curve.generator;
                let k = simplicial.verts[curve.verts[0]].k;

                curve.verts.sort_by(|i, j| simplicial.time_order(*i, *j));

                self.animation_curves.push(AnimationCurve {
                    generator,
                    begin: simplicial.verts[curve.verts[0]].position.w,
                    end: simplicial.verts[curve.verts[curve.verts.len() - 1]]
                        .position
                        .w,
                    key_frames: curve
                        .verts
                        .into_iter()
                        .map(|v| simplicial.verts[v].position)
                        .collect(),
                    albedo: color_of(generator, k),
                    vertex_shape: shape_of(generator),
                });
            }

            for point in simplicial.points.into_values() {
                let VertData {
                    generator,
                    position,
                    ..
                } = simplicial.verts[point];
                self.animation_singularities.push(Component {
                    generator,
                    vertices: position,
                    albedo: color_of(generator, 0),
                    vertex_shape: shape_of(generator),
                });
            }
        } else {
            simplicial.inflate_3d(geometry_samples, signature_styles);
            for tri_buffers in buffer_tris(&simplicial, ctx)? {
                let generator = tri_buffers.generator;
                self.components.push(Component {
                    generator,
                    vertices: vertex_array!(
                        ctx,
                        &tri_buffers.element_buffer,
                        [&tri_buffers.vertex_buffer, &tri_buffers.normal_buffer]
                    )?,
                    albedo: color_of(generator, tri_buffers.k),
                    vertex_shape: shape_of(generator),
                });

                self.wireframe_components.push(vertex_array!(
                    ctx,
                    &tri_buffers.wireframe_element_buffer,
                    [&tri_buffers.vertex_buffer]
                )?);
            }
        }

        Ok(())
    }
}
