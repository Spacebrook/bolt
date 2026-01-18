use bolt_quadtree::quadtree::{Config, QuadTree as BoltQuadTree};
use common::shapes::Rectangle;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::env;
use std::hint::black_box;
use std::time::{Duration, Instant};

const ITER: usize = 400_000;
const RADIUS_ODDS: f32 = 2000.0;
const RADIUS_MIN: f32 = 16.0;
const RADIUS_MAX: f32 = 2048.0;
const MIN_SIZE: f32 = 16.0;
const ARENA_WIDTH: f32 = 100000.0;
const ARENA_HEIGHT: f32 = 100000.0;
const MEASURE_TICKS: usize = 1000;
const QUERIES_NUM: usize = 1000;
const QUERY_WIDTH: f32 = 1920.0;
const QUERY_HEIGHT: f32 = 1080.0;

#[derive(Clone, Copy)]
struct Entity {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
}

impl Entity {
    fn query_rect_extent(&self) -> (f32, f32, f32, f32) {
        let half_w = QUERY_WIDTH * 0.5;
        let half_h = QUERY_HEIGHT * 0.5;
        (
            self.min_x - half_w,
            self.min_y - half_h,
            self.max_x + half_w,
            self.max_y + half_h,
        )
    }
}

#[derive(Clone, Copy)]
struct Bounds {
    min_x: f32,
    min_y: f32,
}

fn randf(rng: &mut StdRng) -> f32 {
    rng.r#gen::<f32>()
}

fn rand_range(rng: &mut StdRng, min: f32, max: f32) -> f32 {
    min + (max - min) * randf(rng)
}

fn bench_min_size() -> f32 {
    std::env::var("BOLT_MIN_SIZE")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(MIN_SIZE)
}

fn make_entities(bounds: Bounds, count: usize, seed: u64) -> Vec<Entity> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut entities = Vec::with_capacity(count);

    for _ in 0..count {
        let radius = if randf(&mut rng) * RADIUS_ODDS >= 1.0 {
            RADIUS_MIN
        } else {
            rand_range(&mut rng, RADIUS_MIN, RADIUS_MAX)
        };
        let w = radius * 2.0;
        let h = radius * 2.0;
        let qtw = ARENA_WIDTH - w;
        let qth = ARENA_HEIGHT - h;

        let min_x = bounds.min_x + qtw * randf(&mut rng);
        let min_y = bounds.min_y + qth * randf(&mut rng);

        entities.push(Entity {
            min_x,
            max_x: min_x + w,
            min_y,
            max_y: min_y + h,
        });
    }

    entities
}

fn main() {
    let bounds = Bounds {
        min_x: -ARENA_WIDTH * 0.5,
        min_y: -ARENA_HEIGHT * 0.5,
    };
    let ticks = env::var("BOLT_QUERY_ONLY_TICKS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(MEASURE_TICKS);
    let entity_count = env::var("BOLT_QUERY_ONLY_ENTITIES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(ITER);
    let seed = env::var("BOLT_QUERY_ONLY_SEED")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(36207250);

    let max_depth = (ARENA_WIDTH / bench_min_size()).log2().ceil() as usize;
    let node_capacity = env::var("BOLT_NODE_CAPACITY")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(64);
    let config = Config {
        pool_size: 4000,
        node_capacity,
        max_depth,
        min_size: bench_min_size(),
        looseness: 1.0,
        large_entity_threshold_factor: 0.0,
        profile_summary: false,
        profile_detail: false,
        profile_limit: 5,
    };

    let entities = make_entities(bounds, entity_count, seed);
    let mut quadtree = BoltQuadTree::new_with_config(
        Rectangle {
            x: 0.0,
            y: 0.0,
            width: ARENA_WIDTH,
            height: ARENA_HEIGHT,
        },
        config,
    ).unwrap();

    for (i, entity) in entities.iter().enumerate() {
        quadtree.insert_rect_extent(
            i as u32,
            entity.min_x,
            entity.min_y,
            entity.max_x,
            entity.max_y,
            None,
        ).unwrap();
    }

    quadtree.update();

    let query_count = QUERIES_NUM.min(entities.len());
    let mut query_total = Duration::ZERO;

    for _ in 0..ticks {
        let start = Instant::now();
        for entity in entities.iter().take(query_count) {
            let (min_x, min_y, max_x, max_y) = entity.query_rect_extent();
            quadtree
                .collisions_rect_extent_with(min_x, min_y, max_x, max_y, |_| {})
                .unwrap();
        }
        query_total += start.elapsed();
    }

    let query_ms = (query_total.as_secs_f64() * 1000.0) / (ticks as f64);
    println!("Query-only settings:");
    println!("Entities:       {}", entity_count);
    println!("Ticks:          {}", ticks);
    println!("Node capacity:  {}", node_capacity);
    println!();
    println!("bolt/quadtree");
    println!("  1k Queries: {:.02}ms", query_ms);

    black_box(query_ms);
}
