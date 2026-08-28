use nalgebra::{Isometry2, Vector2};
use parry2d::query::contact;
use parry2d::shape::SharedShape;

pub struct ShapeWithPosition {
    pub shape: SharedShape,
    pub position: Isometry2<f32>,
}

#[derive(Clone, Copy)]
struct Constraint {
    normal: Vector2<f32>,
    penetration: f32,
}

const CONTACT_MARGIN: f32 = 0.0;
const CONTACT_SLOP: f32 = 1e-2;
const MTV_EPSILON: f32 = 1e-6;
const MAX_SOLVE_ITERATIONS: usize = 8;

pub fn get_mtv(entity: &ShapeWithPosition, others: &[ShapeWithPosition]) -> Option<(f32, f32)> {
    if others.is_empty() {
        return None;
    }

    if let Some(mtv) = get_exact_ball_union_mtv(entity, others) {
        return Some((mtv.x, mtv.y));
    }

    let mut mtv = Vector2::new(0.0, 0.0);
    for _ in 0..MAX_SOLVE_ITERATIONS {
        let translated_position = Isometry2::new(
            entity.position.translation.vector - mtv,
            entity.position.rotation.angle(),
        );
        let constraints = contact_constraints(entity, &translated_position, others);
        if constraints.is_empty() {
            break;
        }

        let step = solve_min_norm_translation(&constraints)?;
        if step.magnitude_squared() <= MTV_EPSILON * MTV_EPSILON {
            break;
        }
        mtv += step;
    }

    if mtv.magnitude_squared() <= MTV_EPSILON * MTV_EPSILON {
        None
    } else {
        Some((mtv.x, mtv.y))
    }
}

#[derive(Clone, Copy)]
struct Disc {
    center: Vector2<f32>,
    radius: f32,
}

#[derive(Clone, Copy)]
struct RoundedRect {
    center: Vector2<f32>,
    axis_x: Vector2<f32>,
    axis_y: Vector2<f32>,
    half_width: f32,
    half_height: f32,
    radius: f32,
}

#[derive(Clone, Copy)]
enum InflatedObstacle {
    Disc(Disc),
    RoundedRect(RoundedRect),
}

#[derive(Clone, Copy)]
struct Segment {
    start: Vector2<f32>,
    end: Vector2<f32>,
}

#[derive(Clone, Copy)]
struct Arc {
    center: Vector2<f32>,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    full: bool,
}

#[derive(Clone, Copy)]
enum BoundaryPiece {
    Segment(Segment),
    Arc(Arc),
}

fn get_exact_ball_union_mtv(
    entity: &ShapeWithPosition,
    others: &[ShapeWithPosition],
) -> Option<Vector2<f32>> {
    let entity_ball = entity.shape.as_ball()?;
    // Work relative to the ball so large world coordinates cannot round a
    // boundary candidate back inside an obstacle and reject the nearest exit.
    let origin = entity.position.translation.vector;
    let obstacles: Option<Vec<InflatedObstacle>> = others
        .iter()
        .map(|other| inflated_obstacle(entity_ball.radius, other, origin))
        .collect();
    let obstacles = obstacles?;
    let start = Vector2::zeros();

    if !obstacles
        .iter()
        .any(|obstacle| obstacle_contains_point(start, *obstacle))
    {
        return None;
    }

    let mut best_point: Option<Vector2<f32>> = None;
    let piece_sets: Vec<Vec<BoundaryPiece>> = obstacles
        .iter()
        .map(|obstacle| obstacle_pieces(*obstacle))
        .collect();

    for obstacle in &obstacles {
        for candidate in obstacle_boundary_candidates(start, *obstacle, &obstacles) {
            consider_exact_candidate(start, candidate, &obstacles, &mut best_point);
        }
    }

    for left_index in 0..piece_sets.len() {
        for right_index in (left_index + 1)..piece_sets.len() {
            for left_piece in &piece_sets[left_index] {
                for right_piece in &piece_sets[right_index] {
                    for candidate in piece_intersections(*left_piece, *right_piece) {
                        consider_exact_candidate(start, candidate, &obstacles, &mut best_point);
                    }
                }
            }
        }
    }

    best_point.map(|point| start - point)
}

fn inflated_obstacle(
    entity_radius: f32,
    other: &ShapeWithPosition,
    origin: Vector2<f32>,
) -> Option<InflatedObstacle> {
    let center = other.position.translation.vector - origin;
    if let Some(other_ball) = other.shape.as_ball() {
        return Some(InflatedObstacle::Disc(Disc {
            center,
            radius: entity_radius + other_ball.radius,
        }));
    }

    let other_cuboid = other.shape.as_cuboid()?;
    let rotation = other.position.rotation.angle();
    let axis_x = Vector2::new(rotation.cos(), rotation.sin());
    let axis_y = Vector2::new(-rotation.sin(), rotation.cos());
    Some(InflatedObstacle::RoundedRect(RoundedRect {
        center,
        axis_x,
        axis_y,
        half_width: other_cuboid.half_extents.x,
        half_height: other_cuboid.half_extents.y,
        radius: entity_radius,
    }))
}

fn obstacle_contains_point(point: Vector2<f32>, obstacle: InflatedObstacle) -> bool {
    match obstacle {
        InflatedObstacle::Disc(disc) => point_inside_disc(point, disc),
        InflatedObstacle::RoundedRect(rect) => point_inside_rounded_rect(point, rect),
    }
}

fn obstacle_boundary_candidates(
    start: Vector2<f32>,
    obstacle: InflatedObstacle,
    all_obstacles: &[InflatedObstacle],
) -> Vec<Vector2<f32>> {
    match obstacle {
        InflatedObstacle::Disc(disc) => disc_boundary_candidates(start, disc, all_obstacles),
        InflatedObstacle::RoundedRect(rect) => obstacle_pieces(InflatedObstacle::RoundedRect(rect))
            .into_iter()
            .map(|piece| closest_point_on_piece(start, piece))
            .collect(),
    }
}

fn disc_boundary_candidates(
    start: Vector2<f32>,
    disc: Disc,
    all_obstacles: &[InflatedObstacle],
) -> Vec<Vector2<f32>> {
    let offset = start - disc.center;
    let offset_norm_sq = offset.magnitude_squared();
    if offset_norm_sq > MTV_EPSILON * MTV_EPSILON {
        let direction = offset / offset_norm_sq.sqrt();
        return vec![disc.center + direction * disc.radius];
    }

    let mut directions = vec![
        Vector2::new(1.0, 0.0),
        Vector2::new(-1.0, 0.0),
        Vector2::new(0.0, 1.0),
        Vector2::new(0.0, -1.0),
    ];
    for other in all_obstacles {
        let nearest = nearest_point_on_obstacle_boundary(disc.center, *other);
        let delta = nearest - disc.center;
        let delta_norm_sq = delta.magnitude_squared();
        if delta_norm_sq <= MTV_EPSILON * MTV_EPSILON {
            continue;
        }
        let direction = delta / delta_norm_sq.sqrt();
        directions.push(direction);
        directions.push(-direction);
    }

    directions
        .into_iter()
        .map(|direction| disc.center + direction * disc.radius)
        .collect()
}

fn nearest_point_on_obstacle_boundary(
    point: Vector2<f32>,
    obstacle: InflatedObstacle,
) -> Vector2<f32> {
    match obstacle {
        InflatedObstacle::Disc(disc) => {
            let offset = point - disc.center;
            let offset_norm_sq = offset.magnitude_squared();
            if offset_norm_sq <= MTV_EPSILON * MTV_EPSILON {
                disc.center + Vector2::new(disc.radius, 0.0)
            } else {
                disc.center + offset / offset_norm_sq.sqrt() * disc.radius
            }
        }
        InflatedObstacle::RoundedRect(rect) => {
            let mut best: Option<(f32, Vector2<f32>)> = None;
            for piece in obstacle_pieces(InflatedObstacle::RoundedRect(rect)) {
                let candidate = closest_point_on_piece(point, piece);
                let distance_sq = (candidate - point).magnitude_squared();
                match best {
                    None => best = Some((distance_sq, candidate)),
                    Some((current_distance_sq, _))
                        if distance_sq + MTV_EPSILON < current_distance_sq =>
                    {
                        best = Some((distance_sq, candidate));
                    }
                    _ => {}
                }
            }
            best.map(|(_, candidate)| candidate).unwrap_or(point)
        }
    }
}

fn obstacle_pieces(obstacle: InflatedObstacle) -> Vec<BoundaryPiece> {
    match obstacle {
        InflatedObstacle::Disc(disc) => vec![BoundaryPiece::Arc(Arc {
            center: disc.center,
            radius: disc.radius,
            start_angle: 0.0,
            end_angle: 0.0,
            full: true,
        })],
        InflatedObstacle::RoundedRect(rect) => rounded_rect_pieces(rect),
    }
}

fn rounded_rect_pieces(rect: RoundedRect) -> Vec<BoundaryPiece> {
    let hw = rect.half_width;
    let hh = rect.half_height;
    let r = rect.radius;

    let top_left = rounded_rect_world_point(rect, Vector2::new(-hw, hh + r));
    let top_right = rounded_rect_world_point(rect, Vector2::new(hw, hh + r));
    let bottom_left = rounded_rect_world_point(rect, Vector2::new(-hw, -(hh + r)));
    let bottom_right = rounded_rect_world_point(rect, Vector2::new(hw, -(hh + r)));
    let left_top = rounded_rect_world_point(rect, Vector2::new(-(hw + r), hh));
    let left_bottom = rounded_rect_world_point(rect, Vector2::new(-(hw + r), -hh));
    let right_top = rounded_rect_world_point(rect, Vector2::new(hw + r, hh));
    let right_bottom = rounded_rect_world_point(rect, Vector2::new(hw + r, -hh));

    let top_right_corner = rounded_rect_world_point(rect, Vector2::new(hw, hh));
    let top_left_corner = rounded_rect_world_point(rect, Vector2::new(-hw, hh));
    let bottom_left_corner = rounded_rect_world_point(rect, Vector2::new(-hw, -hh));
    let bottom_right_corner = rounded_rect_world_point(rect, Vector2::new(hw, -hh));
    let rotation = rect.axis_x.y.atan2(rect.axis_x.x);

    vec![
        BoundaryPiece::Segment(Segment {
            start: top_left,
            end: top_right,
        }),
        BoundaryPiece::Segment(Segment {
            start: right_bottom,
            end: right_top,
        }),
        BoundaryPiece::Segment(Segment {
            start: bottom_right,
            end: bottom_left,
        }),
        BoundaryPiece::Segment(Segment {
            start: left_top,
            end: left_bottom,
        }),
        BoundaryPiece::Arc(Arc {
            center: top_right_corner,
            radius: r,
            start_angle: rotation,
            end_angle: rotation + std::f32::consts::FRAC_PI_2,
            full: false,
        }),
        BoundaryPiece::Arc(Arc {
            center: top_left_corner,
            radius: r,
            start_angle: rotation + std::f32::consts::FRAC_PI_2,
            end_angle: rotation + std::f32::consts::PI,
            full: false,
        }),
        BoundaryPiece::Arc(Arc {
            center: bottom_left_corner,
            radius: r,
            start_angle: rotation + std::f32::consts::PI,
            end_angle: rotation + 3.0 * std::f32::consts::FRAC_PI_2,
            full: false,
        }),
        BoundaryPiece::Arc(Arc {
            center: bottom_right_corner,
            radius: r,
            start_angle: rotation + 3.0 * std::f32::consts::FRAC_PI_2,
            end_angle: rotation + std::f32::consts::TAU,
            full: false,
        }),
    ]
}

fn rounded_rect_world_point(rect: RoundedRect, local: Vector2<f32>) -> Vector2<f32> {
    rect.center + rect.axis_x * local.x + rect.axis_y * local.y
}

fn rounded_rect_local_point(rect: RoundedRect, point: Vector2<f32>) -> Vector2<f32> {
    let delta = point - rect.center;
    Vector2::new(delta.dot(&rect.axis_x), delta.dot(&rect.axis_y))
}

fn point_inside_rounded_rect(point: Vector2<f32>, rect: RoundedRect) -> bool {
    let local = rounded_rect_local_point(rect, point);
    let dx = local.x.abs() - rect.half_width;
    let dy = local.y.abs() - rect.half_height;
    let outside_x = dx.max(0.0);
    let outside_y = dy.max(0.0);
    let outside_distance = (outside_x * outside_x + outside_y * outside_y).sqrt();
    let inside_distance = dx.max(dy).min(0.0);
    outside_distance + inside_distance < rect.radius - CONTACT_SLOP
}

fn consider_exact_candidate(
    start: Vector2<f32>,
    candidate: Vector2<f32>,
    obstacles: &[InflatedObstacle],
    best_point: &mut Option<Vector2<f32>>,
) {
    if obstacles
        .iter()
        .any(|obstacle| obstacle_contains_point(candidate, *obstacle))
    {
        return;
    }

    match best_point {
        None => *best_point = Some(candidate),
        Some(current_best) => {
            let candidate_distance_sq = (candidate - start).magnitude_squared();
            let current_distance_sq = (*current_best - start).magnitude_squared();
            if candidate_distance_sq + MTV_EPSILON < current_distance_sq {
                *best_point = Some(candidate);
            }
        }
    }
}

fn point_inside_disc(point: Vector2<f32>, disc: Disc) -> bool {
    (point - disc.center).magnitude_squared()
        < (disc.radius - CONTACT_SLOP) * (disc.radius - CONTACT_SLOP)
}

fn closest_point_on_piece(point: Vector2<f32>, piece: BoundaryPiece) -> Vector2<f32> {
    match piece {
        BoundaryPiece::Segment(segment) => closest_point_on_segment(point, segment),
        BoundaryPiece::Arc(arc) => closest_point_on_arc(point, arc),
    }
}

fn closest_point_on_segment(point: Vector2<f32>, segment: Segment) -> Vector2<f32> {
    let delta = segment.end - segment.start;
    if delta.x == 0.0 {
        return Vector2::new(
            segment.start.x,
            point.y.clamp(
                segment.start.y.min(segment.end.y),
                segment.start.y.max(segment.end.y),
            ),
        );
    }
    if delta.y == 0.0 {
        return Vector2::new(
            point.x.clamp(
                segment.start.x.min(segment.end.x),
                segment.start.x.max(segment.end.x),
            ),
            segment.start.y,
        );
    }
    let length_sq = delta.magnitude_squared();
    if length_sq <= MTV_EPSILON * MTV_EPSILON {
        return segment.start;
    }
    let t = ((point - segment.start).dot(&delta) / length_sq).clamp(0.0, 1.0);
    segment.start + delta * t
}

fn closest_point_on_arc(point: Vector2<f32>, arc: Arc) -> Vector2<f32> {
    let delta = point - arc.center;
    if delta.magnitude_squared() <= MTV_EPSILON * MTV_EPSILON {
        return point_on_arc(arc, arc.start_angle);
    }
    let angle = delta.y.atan2(delta.x);
    if arc.full || angle_in_arc(angle, arc) {
        return point_on_arc(arc, angle);
    }

    let start_point = point_on_arc(arc, arc.start_angle);
    let end_point = point_on_arc(arc, arc.end_angle);
    if (start_point - point).magnitude_squared() <= (end_point - point).magnitude_squared() {
        start_point
    } else {
        end_point
    }
}

fn point_on_arc(arc: Arc, angle: f32) -> Vector2<f32> {
    arc.center + Vector2::new(angle.cos(), angle.sin()) * arc.radius
}

fn piece_intersections(left: BoundaryPiece, right: BoundaryPiece) -> Vec<Vector2<f32>> {
    match (left, right) {
        (BoundaryPiece::Segment(left_segment), BoundaryPiece::Segment(right_segment)) => {
            segment_segment_intersections(left_segment, right_segment)
        }
        (BoundaryPiece::Segment(segment), BoundaryPiece::Arc(arc))
        | (BoundaryPiece::Arc(arc), BoundaryPiece::Segment(segment)) => {
            segment_arc_intersections(segment, arc)
        }
        (BoundaryPiece::Arc(left_arc), BoundaryPiece::Arc(right_arc)) => {
            arc_arc_intersections(left_arc, right_arc)
        }
    }
}

fn segment_segment_intersections(left: Segment, right: Segment) -> Vec<Vector2<f32>> {
    let left_delta = left.end - left.start;
    let right_delta = right.end - right.start;
    let cross_delta = cross(left_delta, right_delta);
    let delta_start = right.start - left.start;

    if cross_delta.abs() <= MTV_EPSILON {
        if cross(delta_start, left_delta).abs() > MTV_EPSILON {
            return Vec::new();
        }
        let left_length_sq = left_delta.magnitude_squared();
        if left_length_sq <= MTV_EPSILON * MTV_EPSILON {
            return Vec::new();
        }
        let right_start_t = delta_start.dot(&left_delta) / left_length_sq;
        let right_end_t = (right.end - left.start).dot(&left_delta) / left_length_sq;
        let overlap_start = right_start_t.min(right_end_t).max(0.0);
        let overlap_end = right_start_t.max(right_end_t).min(1.0);
        if overlap_end + MTV_EPSILON < overlap_start {
            return Vec::new();
        }
        return vec![
            left.start + left_delta * overlap_start,
            left.start + left_delta * overlap_end,
        ];
    }

    let left_t = cross(delta_start, right_delta) / cross_delta;
    let right_t = cross(delta_start, left_delta) / cross_delta;
    if !(-MTV_EPSILON..=1.0 + MTV_EPSILON).contains(&left_t)
        || !(-MTV_EPSILON..=1.0 + MTV_EPSILON).contains(&right_t)
    {
        return Vec::new();
    }
    vec![left.start + left_delta * left_t.clamp(0.0, 1.0)]
}

fn segment_arc_intersections(segment: Segment, arc: Arc) -> Vec<Vector2<f32>> {
    let delta = segment.end - segment.start;
    let offset = segment.start - arc.center;
    let a = delta.dot(&delta);
    if a <= MTV_EPSILON * MTV_EPSILON {
        return Vec::new();
    }
    let b = 2.0 * offset.dot(&delta);
    let c = offset.dot(&offset) - arc.radius * arc.radius;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < -MTV_EPSILON {
        return Vec::new();
    }
    if discriminant.abs() <= MTV_EPSILON {
        let t = -b / (2.0 * a);
        return segment_arc_candidate(segment, arc, t).into_iter().collect();
    }

    let root = discriminant.sqrt();
    let t1 = (-b - root) / (2.0 * a);
    let t2 = (-b + root) / (2.0 * a);
    let mut points = Vec::new();
    if let Some(point) = segment_arc_candidate(segment, arc, t1) {
        points.push(point);
    }
    if let Some(point) = segment_arc_candidate(segment, arc, t2) {
        points.push(point);
    }
    points
}

fn segment_arc_candidate(segment: Segment, arc: Arc, t: f32) -> Option<Vector2<f32>> {
    if !(-MTV_EPSILON..=1.0 + MTV_EPSILON).contains(&t) {
        return None;
    }
    let point = segment.start + (segment.end - segment.start) * t.clamp(0.0, 1.0);
    let angle = (point.y - arc.center.y).atan2(point.x - arc.center.x);
    if arc.full || angle_in_arc(angle, arc) {
        Some(point)
    } else {
        None
    }
}

fn arc_arc_intersections(left: Arc, right: Arc) -> Vec<Vector2<f32>> {
    circle_circle_intersections(
        Disc {
            center: left.center,
            radius: left.radius,
        },
        Disc {
            center: right.center,
            radius: right.radius,
        },
    )
    .into_iter()
    .filter(|point| {
        let left_angle = (point.y - left.center.y).atan2(point.x - left.center.x);
        let right_angle = (point.y - right.center.y).atan2(point.x - right.center.x);
        (left.full || angle_in_arc(left_angle, left))
            && (right.full || angle_in_arc(right_angle, right))
    })
    .collect()
}

fn angle_in_arc(angle: f32, arc: Arc) -> bool {
    if arc.full {
        return true;
    }
    let angle = normalize_angle(angle);
    let start = normalize_angle(arc.start_angle);
    let end = normalize_angle(arc.end_angle);
    if start <= end {
        angle + MTV_EPSILON >= start && angle - MTV_EPSILON <= end
    } else {
        angle + MTV_EPSILON >= start || angle - MTV_EPSILON <= end
    }
}

fn normalize_angle(angle: f32) -> f32 {
    angle.rem_euclid(std::f32::consts::TAU)
}

fn cross(left: Vector2<f32>, right: Vector2<f32>) -> f32 {
    left.x * right.y - left.y * right.x
}

fn circle_circle_intersections(left: Disc, right: Disc) -> Vec<Vector2<f32>> {
    let delta = right.center - left.center;
    let distance_sq = delta.magnitude_squared();
    if distance_sq <= MTV_EPSILON * MTV_EPSILON {
        return Vec::new();
    }

    let distance = distance_sq.sqrt();
    if distance > left.radius + right.radius + CONTACT_SLOP {
        return Vec::new();
    }
    if distance < (left.radius - right.radius).abs() - CONTACT_SLOP {
        return Vec::new();
    }

    let a =
        (left.radius * left.radius - right.radius * right.radius + distance_sq) / (2.0 * distance);
    let height_sq = left.radius * left.radius - a * a;
    if height_sq < -MTV_EPSILON {
        return Vec::new();
    }

    let midpoint = left.center + delta * (a / distance);
    if height_sq <= MTV_EPSILON {
        return vec![midpoint];
    }

    let height = height_sq.sqrt();
    let perpendicular = Vector2::new(-delta.y / distance, delta.x / distance);
    vec![
        midpoint + perpendicular * height,
        midpoint - perpendicular * height,
    ]
}

fn contact_constraints(
    entity: &ShapeWithPosition,
    translated_position: &Isometry2<f32>,
    others: &[ShapeWithPosition],
) -> Vec<Constraint> {
    others
        .iter()
        .filter_map(|other| {
            contact(
                translated_position,
                entity.shape.as_ref(),
                &other.position,
                other.shape.as_ref(),
                CONTACT_MARGIN,
            )
            .ok()
            .flatten()
        })
        .filter_map(|contact| {
            let penetration = -contact.dist;
            if penetration <= CONTACT_SLOP {
                return None;
            }
            Some(Constraint {
                normal: contact.normal1.into_inner(),
                penetration,
            })
        })
        .collect()
}

fn solve_min_norm_translation(constraints: &[Constraint]) -> Option<Vector2<f32>> {
    let mut best: Option<Vector2<f32>> = None;

    for constraint in constraints {
        consider_candidate(
            constraint.normal * constraint.penetration,
            constraints,
            &mut best,
        );
    }

    for left_index in 0..constraints.len() {
        let left = constraints[left_index];
        for right in &constraints[(left_index + 1)..] {
            let determinant = left.normal.x * right.normal.y - left.normal.y * right.normal.x;
            if determinant.abs() <= MTV_EPSILON {
                continue;
            }

            let candidate_x = (left.penetration * right.normal.y
                - left.normal.y * right.penetration)
                / determinant;
            let candidate_y = (left.normal.x * right.penetration
                - left.penetration * right.normal.x)
                / determinant;
            consider_candidate(
                Vector2::new(candidate_x, candidate_y),
                constraints,
                &mut best,
            );
        }
    }

    best
}

fn consider_candidate(
    candidate: Vector2<f32>,
    constraints: &[Constraint],
    best: &mut Option<Vector2<f32>>,
) {
    if !satisfies_constraints(candidate, constraints) {
        return;
    }
    match best {
        None => *best = Some(candidate),
        Some(current_best) => {
            if candidate.magnitude_squared() + MTV_EPSILON < current_best.magnitude_squared() {
                *best = Some(candidate);
            }
        }
    }
}

fn satisfies_constraints(candidate: Vector2<f32>, constraints: &[Constraint]) -> bool {
    constraints.iter().all(|constraint| {
        candidate.dot(&constraint.normal) + CONTACT_SLOP >= constraint.penetration
    })
}
