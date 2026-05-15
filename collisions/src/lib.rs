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
