//! Continuous terrain surface prototype code.

use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::Mesh,
    render::render_resource::PrimitiveTopology,
};

const COAST_HEIGHT: f32 = 0.0;
const OCEAN_DEPTH: f32 = -8.0;
const LAND_STEP: f32 = 4.0;
const CASE_SPACING: f32 = 54.0;
const CORNER_RADIUS: f32 = 16.0;
const BRIDGE_WIDTH: f32 = 9.0;

/// Prototype edge class used by the isolated R1 corner harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrototypeEdgeClass {
    /// Same tier.
    Flat,
    /// One-tier passable transition.
    Slope,
    /// Multi-tier or forced impassable transition.
    Cliff,
}

/// One synthetic three-tile corner case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CornerPrototypeCase {
    /// Human-readable case name.
    pub name: &'static str,
    /// Clockwise tile tiers around the corner.
    pub levels: [u8; 3],
    /// Edges forced to cliff independent of tier delta.
    pub forced_cliff_edges: [bool; 3],
}

/// CPU mesh data for an R1 prototype mesh.
#[derive(Debug, Clone)]
pub struct PrototypeMeshData {
    /// Vertex positions.
    pub positions: Vec<[f32; 3]>,
    /// Vertex normals.
    pub normals: Vec<[f32; 3]>,
    /// Vertex colors.
    pub colors: Vec<[f32; 4]>,
    /// Triangle indices.
    pub indices: Vec<u32>,
}

impl PrototypeMeshData {
    /// Converts the prototype data into a Bevy mesh.
    pub fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

/// World-space height for a discrete terrain tier.
pub fn tier_height(height_level: u8, mountain_level: u8) -> f32 {
    match height_level {
        0 => OCEAN_DEPTH,
        1 => COAST_HEIGHT,
        level => LAND_STEP * f32::from(level.min(mountain_level).saturating_sub(1)),
    }
}

/// Builds the synthetic case suite used by the R1 corner prototype harness.
pub fn corner_prototype_cases() -> Vec<CornerPrototypeCase> {
    let mut cases = vec![
        CornerPrototypeCase {
            name: "equal",
            levels: [2, 2, 2],
            forced_cliff_edges: [false; 3],
        },
        CornerPrototypeCase {
            name: "one-step-a",
            levels: [2, 3, 2],
            forced_cliff_edges: [false; 3],
        },
        CornerPrototypeCase {
            name: "one-step-b",
            levels: [3, 2, 3],
            forced_cliff_edges: [false; 3],
        },
        CornerPrototypeCase {
            name: "multi-step",
            levels: [2, 4, 3],
            forced_cliff_edges: [false; 3],
        },
    ];

    for edge in 0..3 {
        let mut forced = [false; 3];
        forced[edge] = true;
        cases.push(CornerPrototypeCase {
            name: "forced-cliff",
            levels: [2, 3, 2],
            forced_cliff_edges: forced,
        });
    }

    for a in 2..=4 {
        for b in 2..=4 {
            for c in 2..=4 {
                cases.push(CornerPrototypeCase {
                    name: "exhaustive",
                    levels: [a, b, c],
                    forced_cliff_edges: [false; 3],
                });
            }
        }
    }

    cases
}

/// Classifies one prototype edge.
pub fn prototype_edge_class(
    left_level: u8,
    right_level: u8,
    forced_cliff: bool,
) -> PrototypeEdgeClass {
    if forced_cliff || left_level.abs_diff(right_level) >= 2 {
        PrototypeEdgeClass::Cliff
    } else if left_level == right_level {
        PrototypeEdgeClass::Flat
    } else {
        PrototypeEdgeClass::Slope
    }
}

/// Builds CPU mesh data for one isolated three-tier corner case.
pub fn build_three_tier_corner_prototype_data(case: CornerPrototypeCase) -> PrototypeMeshData {
    let mut data = PrototypeMeshData {
        positions: Vec::new(),
        normals: Vec::new(),
        colors: Vec::new(),
        indices: Vec::new(),
    };
    push_case(&mut data, case, [0.0, 0.0]);
    data
}

/// Builds CPU mesh data for every R1 case, laid out in a grid.
pub fn build_corner_prototype_suite_data() -> PrototypeMeshData {
    let cases = corner_prototype_cases();
    let columns = 7;
    let mut data = PrototypeMeshData {
        positions: Vec::new(),
        normals: Vec::new(),
        colors: Vec::new(),
        indices: Vec::new(),
    };

    for (index, case) in cases.into_iter().enumerate() {
        let column = index % columns;
        let row = index / columns;
        push_case(
            &mut data,
            case,
            [
                usize_to_f32(column) * CASE_SPACING,
                -usize_to_f32(row) * CASE_SPACING,
            ],
        );
    }

    data
}

fn push_case(data: &mut PrototypeMeshData, case: CornerPrototypeCase, offset: [f32; 2]) {
    let points = [
        [offset[0], offset[1] + CORNER_RADIUS],
        [
            offset[0] - CORNER_RADIUS * 0.866_025_4,
            offset[1] - CORNER_RADIUS * 0.5,
        ],
        [
            offset[0] + CORNER_RADIUS * 0.866_025_4,
            offset[1] - CORNER_RADIUS * 0.5,
        ],
    ];
    let center = [offset[0], offset[1]];
    let heights = case.levels.map(|level| tier_height(level, 5));

    push_triangle(
        data,
        [
            [points[0][0], points[0][1], heights[0]],
            [points[1][0], points[1][1], heights[1]],
            [points[2][0], points[2][1], heights[2]],
        ],
        color_for_class(corner_dominant_class(case)),
    );

    for edge in 0..3 {
        let next = (edge + 1) % 3;
        let class = prototype_edge_class(
            case.levels[edge],
            case.levels[next],
            case.forced_cliff_edges[edge],
        );
        push_bridge(
            data,
            center,
            points[edge],
            points[next],
            heights[edge],
            heights[next],
            class,
        );
    }
}

fn push_bridge(
    data: &mut PrototypeMeshData,
    center: [f32; 2],
    left: [f32; 2],
    right: [f32; 2],
    left_height: f32,
    right_height: f32,
    class: PrototypeEdgeClass,
) {
    let midpoint = [(left[0] + right[0]) * 0.5, (left[1] + right[1]) * 0.5];
    let outward = normalize([midpoint[0] - center[0], midpoint[1] - center[1]]);
    let outer_left = [
        left[0] + outward[0] * BRIDGE_WIDTH,
        left[1] + outward[1] * BRIDGE_WIDTH,
    ];
    let outer_right = [
        right[0] + outward[0] * BRIDGE_WIDTH,
        right[1] + outward[1] * BRIDGE_WIDTH,
    ];

    match class {
        PrototypeEdgeClass::Flat | PrototypeEdgeClass::Slope => {
            push_quad(
                data,
                [
                    [left[0], left[1], left_height],
                    [right[0], right[1], right_height],
                    [outer_right[0], outer_right[1], right_height],
                    [outer_left[0], outer_left[1], left_height],
                ],
                color_for_class(class),
            );
        }
        PrototypeEdgeClass::Cliff => {
            let low = left_height.min(right_height);
            let high = left_height.max(right_height);
            push_quad(
                data,
                [
                    [left[0], left[1], high],
                    [right[0], right[1], high],
                    [right[0], right[1], low],
                    [left[0], left[1], low],
                ],
                color_for_class(class),
            );
            push_quad(
                data,
                [
                    [left[0], left[1], low],
                    [right[0], right[1], low],
                    [outer_right[0], outer_right[1], low],
                    [outer_left[0], outer_left[1], low],
                ],
                color_for_class(class),
            );
        }
    }
}

fn push_quad(data: &mut PrototypeMeshData, vertices: [[f32; 3]; 4], color: [f32; 4]) {
    push_triangle(data, [vertices[0], vertices[1], vertices[2]], color);
    push_triangle(data, [vertices[0], vertices[2], vertices[3]], color);
}

fn push_triangle(data: &mut PrototypeMeshData, vertices: [[f32; 3]; 3], color: [f32; 4]) {
    let normal = triangle_normal(vertices);
    let start = u32::try_from(data.positions.len()).expect("prototype vertex count fits in u32");
    data.positions.extend_from_slice(&vertices);
    data.normals.extend_from_slice(&[normal; 3]);
    data.colors.extend_from_slice(&[color; 3]);
    data.indices
        .extend_from_slice(&[start, start + 1, start + 2]);
}

fn triangle_normal(vertices: [[f32; 3]; 3]) -> [f32; 3] {
    let a = subtract(vertices[1], vertices[0]);
    let b = subtract(vertices[2], vertices[0]);
    normalize3([
        a[1].mul_add(b[2], -(a[2] * b[1])),
        a[2].mul_add(b[0], -(a[0] * b[2])),
        a[0].mul_add(b[1], -(a[1] * b[0])),
    ])
}

fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn normalize(value: [f32; 2]) -> [f32; 2] {
    let length = (value[0].mul_add(value[0], value[1] * value[1])).sqrt();
    if length == 0.0 {
        [0.0, 1.0]
    } else {
        [value[0] / length, value[1] / length]
    }
}

fn normalize3(value: [f32; 3]) -> [f32; 3] {
    let length =
        (value[0].mul_add(value[0], value[1].mul_add(value[1], value[2] * value[2]))).sqrt();
    if length == 0.0 {
        [0.0, 0.0, 1.0]
    } else {
        [value[0] / length, value[1] / length, value[2] / length]
    }
}

fn corner_dominant_class(case: CornerPrototypeCase) -> PrototypeEdgeClass {
    let classes = [
        prototype_edge_class(case.levels[0], case.levels[1], case.forced_cliff_edges[0]),
        prototype_edge_class(case.levels[1], case.levels[2], case.forced_cliff_edges[1]),
        prototype_edge_class(case.levels[2], case.levels[0], case.forced_cliff_edges[2]),
    ];
    if classes.contains(&PrototypeEdgeClass::Cliff) {
        PrototypeEdgeClass::Cliff
    } else if classes.contains(&PrototypeEdgeClass::Slope) {
        PrototypeEdgeClass::Slope
    } else {
        PrototypeEdgeClass::Flat
    }
}

fn color_for_class(class: PrototypeEdgeClass) -> [f32; 4] {
    match class {
        PrototypeEdgeClass::Flat => [0.25, 0.55, 0.30, 1.0],
        PrototypeEdgeClass::Slope => [0.72, 0.58, 0.28, 1.0],
        PrototypeEdgeClass::Cliff => [0.46, 0.43, 0.40, 1.0],
    }
}

#[allow(clippy::cast_precision_loss)]
fn usize_to_f32(value: usize) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_height_uses_constant_land_step() {
        assert_height_eq(tier_height(0, 5), -8.0);
        assert_height_eq(tier_height(1, 5), 0.0);
        assert_height_eq(tier_height(2, 5), 4.0);
        assert_height_eq(tier_height(5, 5), 16.0);
        assert_height_eq(tier_height(7, 5), 16.0);
    }

    #[test]
    fn prototype_suite_covers_core_corner_combinations() {
        let cases = corner_prototype_cases();
        assert!(cases.iter().any(|case| case.levels == [2, 2, 2]));
        assert!(cases.iter().any(|case| case.levels == [2, 3, 2]));
        assert!(cases.iter().any(|case| case.levels == [2, 4, 3]));
        for edge in 0..3 {
            assert!(
                cases.iter().any(|case| case.forced_cliff_edges[edge]),
                "forced cliff edge {edge} must be represented"
            );
        }
    }

    #[test]
    fn one_step_edge_is_slope_unless_forced() {
        assert_eq!(prototype_edge_class(2, 3, false), PrototypeEdgeClass::Slope);
        assert_eq!(prototype_edge_class(2, 3, true), PrototypeEdgeClass::Cliff);
    }

    #[test]
    fn prototype_mesh_indices_are_triangles_in_bounds() {
        let data = build_corner_prototype_suite_data();
        assert_eq!(data.positions.len(), data.normals.len());
        assert_eq!(data.positions.len(), data.colors.len());
        assert_eq!(data.indices.len() % 3, 0);
        let vertex_count = u32::try_from(data.positions.len()).unwrap();
        assert!(data.indices.iter().all(|index| *index < vertex_count));
    }

    fn assert_height_eq(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= f32::EPSILON,
            "expected {expected}, got {actual}"
        );
    }
}
