// Inspired by c_quadtree: https://github.com/supahero1/quadtree
use common::shapes::{Rectangle, ShapeEnum};
use quadtree::quadtree::{Config, QuadTree};
use rand::prelude::*;
use std::env;
use std::time::{Duration, Instant};

const ITER: usize = 400_000;
const RADIUS_ODDS: f32 = 2000.0;
const RADIUS_MIN: f32 = 16.0;
const RADIUS_MAX: f32 = 2048.0;
const MIN_SIZE: f32 = 16.0;
const ARENA_WIDTH: f32 = 100000.0;
const ARENA_HEIGHT: f32 = 100000.0;
const MEASURE_TICKS: usize = 1000;
const INITIAL_VELOCITY: f32 = 0.9;
const BOUNDS_VELOCITY_LOSS: f32 = 0.99;
const QUERIES_NUM: usize = 1000;
const QUERY_WIDTH: f32 = 1920.0;
const QUERY_HEIGHT: f32 = 1080.0;

#[derive(Clone, Copy)]
struct Entity {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    vx: f32,
    vy: f32,
}

impl Entity {
    fn width(&self) -> f32 {
        self.max_x - self.min_x
    }

    fn height(&self) -> f32 {
        self.max_y - self.min_y
    }

    fn center_x(&self) -> f32 {
        (self.min_x + self.max_x) * 0.5
    }

    fn center_y(&self) -> f32 {
        (self.min_y + self.max_y) * 0.5
    }

    fn to_rectangle(&self) -> Rectangle {
        Rectangle {
            x: self.center_x(),
            y: self.center_y(),
            width: self.width(),
            height: self.height(),
        }
    }

    fn query_rectangle(&self) -> Rectangle {
        Rectangle {
            x: self.center_x(),
            y: self.center_y(),
            width: self.width() + QUERY_WIDTH,
            height: self.height() + QUERY_HEIGHT,
        }
    }
}

#[derive(Clone, Copy)]
struct HalfExtent {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Clone, Copy)]
struct Bounds {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
}

fn randf(rng: &mut impl Rng) -> f32 {
    rng.gen::<f32>()
}

fn gen_radius(rng: &mut impl Rng) -> f32 {
    let mut r = randf(rng) * RADIUS_ODDS;
    if r == 0.0 {
        return RADIUS_MAX;
    }
    let len = RADIUS_MAX - RADIUS_MIN;
    r = (1.0 / r) * len + RADIUS_MIN;
    r.min(RADIUS_MAX)
}

fn rect_to_half_extent(entity: &Entity) -> HalfExtent {
    let w = (entity.max_x - entity.min_x) * 0.5;
    let h = (entity.max_y - entity.min_y) * 0.5;
    HalfExtent {
        x: entity.min_x + w,
        y: entity.min_y + h,
        w,
        h,
    }
}

fn move_entity(entity: &mut Entity, dx: f32, dy: f32) {
    entity.min_x += dx;
    entity.max_x += dx;
    entity.min_y += dy;
    entity.max_y += dy;
}

fn collide_entities(entity_a: &mut Entity, entity_b: &mut Entity) {
    let extent_a = rect_to_half_extent(entity_a);
    let extent_b = rect_to_half_extent(entity_b);

    let diff_x = extent_a.x - extent_b.x;
    let diff_y = extent_a.y - extent_b.y;
    let overlap_x = (extent_a.w + extent_b.w) - diff_x.abs();
    let overlap_y = (extent_a.h + extent_b.h) - diff_y.abs();

    if overlap_x > 0.0 && overlap_y > 0.0 {
        let size_a = extent_a.w * extent_a.h * 4.0;
        let size_b = extent_b.w * extent_b.h * 4.0;
        let total_size = size_a + size_b;
        if total_size == 0.0 {
            return;
        }

        if overlap_x < overlap_y {
            let push_a = overlap_x * (size_b / total_size);
            let push_b = overlap_x * (size_a / total_size);

            if diff_x > 0.0 {
                move_entity(entity_a, push_a, 0.0);
                move_entity(entity_b, -push_b, 0.0);
            } else {
                move_entity(entity_a, -push_a, 0.0);
                move_entity(entity_b, push_b, 0.0);
            }

            let temp_vx = entity_a.vx;
            entity_a.vx =
                (entity_a.vx * (size_a - size_b) + 2.0 * size_b * entity_b.vx) / total_size;
            entity_b.vx = (entity_b.vx * (size_b - size_a) + 2.0 * size_a * temp_vx) / total_size;
        } else {
            let push_a = overlap_y * (size_b / total_size);
            let push_b = overlap_y * (size_a / total_size);

            if diff_y > 0.0 {
                move_entity(entity_a, 0.0, push_a);
                move_entity(entity_b, 0.0, -push_b);
            } else {
                move_entity(entity_a, 0.0, -push_a);
                move_entity(entity_b, 0.0, push_b);
            }

            let temp_vy = entity_a.vy;
            entity_a.vy =
                (entity_a.vy * (size_a - size_b) + 2.0 * size_b * entity_b.vy) / total_size;
            entity_b.vy = (entity_b.vy * (size_b - size_a) + 2.0 * size_a * temp_vy) / total_size;
        }
    }
}

fn update_entity(entity: &mut Entity, bounds: &Bounds) {
    entity.min_x += entity.vx;
    entity.max_x += entity.vx;
    entity.min_y += entity.vy;
    entity.max_y += entity.vy;

    if entity.min_x < bounds.min_x {
        let width = entity.width();
        entity.min_x = bounds.min_x;
        entity.max_x = bounds.min_x + width;
        entity.vx = entity.vx.abs() * BOUNDS_VELOCITY_LOSS;
    } else if entity.max_x > bounds.max_x {
        let width = entity.width();
        entity.max_x = bounds.max_x;
        entity.min_x = bounds.max_x - width;
        entity.vx = -entity.vx.abs() * BOUNDS_VELOCITY_LOSS;
    }

    if entity.min_y < bounds.min_y {
        let height = entity.height();
        entity.min_y = bounds.min_y;
        entity.max_y = bounds.min_y + height;
        entity.vy = entity.vy.abs() * BOUNDS_VELOCITY_LOSS;
    } else if entity.max_y > bounds.max_y {
        let height = entity.height();
        entity.max_y = bounds.max_y;
        entity.min_y = bounds.max_y - height;
        entity.vy = -entity.vy.abs() * BOUNDS_VELOCITY_LOSS;
    }
}

fn resolve_pair(entities: &mut [Entity], a: u32, b: u32) {
    let a = a as usize;
    let b = b as usize;
    if a == b {
        return;
    }

    if a < b {
        let (left, right) = entities.split_at_mut(b);
        let entity_a = &mut left[a];
        let entity_b = &mut right[0];
        collide_entities(entity_a, entity_b);
    } else {
        let (left, right) = entities.split_at_mut(a);
        let entity_b = &mut left[b];
        let entity_a = &mut right[0];
        collide_entities(entity_a, entity_b);
    }
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn main() {
    let bounds = Bounds {
        min_x: -ARENA_WIDTH * 0.5,
        max_x: ARENA_WIDTH * 0.5,
        min_y: -ARENA_HEIGHT * 0.5,
        max_y: ARENA_HEIGHT * 0.5,
    };

    let max_depth = (ARENA_WIDTH / MIN_SIZE).log2().ceil() as usize;
    let config = Config {
        pool_size: 4000,
        node_capacity: 13,
        max_depth,
        min_size: MIN_SIZE,
        looseness: 1.0,
        large_entity_threshold_factor: 0.0,
        profile_summary: false,
        profile_detail: false,
        profile_limit: 5,
    };

    let ticks = env::var("BOLT_BENCH_TICKS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(MEASURE_TICKS);
    let query_push = env::var("BOLT_BENCH_QUERY_PUSH")
        .ok()
        .map_or(false, |value| value != "0");

    println!("Simulation settings:");
    println!("Initial count:    {}", ITER);
    println!(
        "Arena size:       {:.01} x {:.01}",
        ARENA_WIDTH, ARENA_HEIGHT
    );
    println!(
        "Initial radius:   From {:.01} to {:.01}",
        RADIUS_MIN, RADIUS_MAX
    );
    println!(
        "Query results:    {}",
        if query_push { "collect" } else { "ignore" }
    );
    println!("Seed: {}", 36207250);
    println!("Measure ticks:    {}", ticks);

    let mut rng = StdRng::seed_from_u64(36207250);
    let mut entities = Vec::with_capacity(ITER);
    let mut quadtree = QuadTree::new_with_config(
        Rectangle {
            x: 0.0,
            y: 0.0,
            width: ARENA_WIDTH,
            height: ARENA_HEIGHT,
        },
        config,
    );

    let start = Instant::now();
    for i in 0..ITER {
        let mut dim = [0.0_f32; 2];
        let idx = (i & 1) as usize;

        let w = gen_radius(&mut rng);
        let temp = gen_radius(&mut rng);
        let mut h = w + (temp * 0.5);
        if h > RADIUS_MAX {
            h = RADIUS_MAX;
        }
        if h < RADIUS_MIN {
            h = RADIUS_MIN;
        }

        dim[idx] = w;
        dim[1 - idx] = h;

        let w = dim[0];
        let h = dim[1];

        let qtw = ARENA_WIDTH - w;
        let qth = ARENA_HEIGHT - h;

        let min_x = bounds.min_x + qtw * randf(&mut rng);
        let min_y = bounds.min_y + qth * randf(&mut rng);

        let entity = Entity {
            min_x,
            max_x: min_x + w,
            min_y,
            max_y: min_y + h,
            vx: (1.0 - 2.0 * randf(&mut rng)) * INITIAL_VELOCITY,
            vy: (1.0 - 2.0 * randf(&mut rng)) * INITIAL_VELOCITY,
        };

        quadtree.insert(i as u32, ShapeEnum::Rectangle(entity.to_rectangle()), None);
        entities.push(entity);
    }

    let insert_time = start.elapsed();
    println!(
        "Queueing insertions took {:.02}ms",
        duration_ms(insert_time)
    );

    let mut collide_total = Duration::ZERO;
    let mut update_total = Duration::ZERO;
    let mut relocate_total = Duration::ZERO;
    let mut normalize_total = Duration::ZERO;
    let mut query_total = Duration::ZERO;
    let mut collisions = Vec::new();

    for _ in 0..ticks {
        let start = Instant::now();
        quadtree.for_each_collision_pair(|a, b| resolve_pair(&mut entities, a, b));
        collide_total += start.elapsed();

        let start = Instant::now();
        for entity in entities.iter_mut() {
            update_entity(entity, &bounds);
        }
        update_total += start.elapsed();

        let start = Instant::now();
        for (value, entity) in entities.iter().enumerate() {
            quadtree.relocate(
                value as u32,
                ShapeEnum::Rectangle(entity.to_rectangle()),
                None,
            );
        }
        relocate_total += start.elapsed();

        let start = Instant::now();
        quadtree.update();
        normalize_total += start.elapsed();

        let start = Instant::now();
        for i in 0..QUERIES_NUM {
            if query_push {
                collisions.clear();
                quadtree.collisions(
                    ShapeEnum::Rectangle(entities[i].query_rectangle()),
                    &mut collisions,
                );
            } else {
                quadtree
                    .collisions_with(ShapeEnum::Rectangle(entities[i].query_rectangle()), |_| {});
            }
        }
        query_total += start.elapsed();
    }

    let ticks_f = ticks as f64;
    let collide_ms = duration_ms(collide_total) / ticks_f;
    let update_ms = duration_ms(update_total) / ticks_f;
    let relocate_ms = duration_ms(relocate_total) / ticks_f;
    let normalize_ms = duration_ms(normalize_total) / ticks_f;
    let query_ms = duration_ms(query_total) / ticks_f;
    let total_ms = collide_ms + update_ms + relocate_ms + normalize_ms;

    println!();
    println!("Collide: {:.02}ms", collide_ms);
    println!("Update: {:.02}ms", update_ms);
    println!("Relocate: {:.02}ms", relocate_ms);
    println!("Normalize: {:.02}ms", normalize_ms);
    println!("Tick total: {:.02}ms", total_ms);
    println!("1k Queries: {:.02}ms", query_ms);
}
