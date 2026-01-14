mod c_quadtree;

use bolt_quadtree::quadtree::{Config, QuadTree as BoltQuadTree};
use common::shapes::{Rectangle, ShapeEnum};
use quadtree_crate::{Quadtree as QtGeneric, Vec2, shapes::Rect as QtRect, vec2};
use quadtree_f32::{Item, ItemId, Point as F32Point, QuadTree as QtF32, Rect as RectF32};
use quadtree_rs::{Quadtree as QtRs, area::AreaBuilder, point::Point as RsPoint};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use spatialtree::{QuadTree as SpatialQuadTree, QuadVec};
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
const INITIAL_VELOCITY: f32 = 0.9;
const BOUNDS_VELOCITY_LOSS: f32 = 0.99;
const QUERIES_NUM: usize = 1000;
const QUERY_WIDTH: f32 = 1920.0;
const QUERY_HEIGHT: f32 = 1080.0;
const QUADTREE_F32_MAX_ENTITIES: usize = 30_000;

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
    fn to_rectangle(&self) -> Rectangle {
        Rectangle {
            x: self.center_x(),
            y: self.center_y(),
            width: self.width(),
            height: self.height(),
        }
    }

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

struct GridMapper {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    size: u32,
}

impl GridMapper {
    fn new(bounds: Bounds, depth: u8) -> Self {
        let size = 1u32 << depth;
        Self {
            min_x: bounds.min_x,
            max_x: bounds.max_x,
            min_y: bounds.min_y,
            max_y: bounds.max_y,
            size,
        }
    }

    fn map_pos(&self, value: f32, min: f32, max: f32) -> u32 {
        let mut t = (value - min) / (max - min);
        if t < 0.0 {
            t = 0.0;
        } else if t > 1.0 {
            t = 1.0;
        }
        (t * (self.size.saturating_sub(1) as f32)).round() as u32
    }

    fn map_dim(&self, value: f32, span: f32) -> u32 {
        let mut t = value / span;
        if t < 0.0 {
            t = 0.0;
        }
        let size = (t * self.size as f32).round() as u32;
        size.max(1)
    }

    fn map_area(&self, min_x: f32, min_y: f32, width: f32, height: f32) -> (u32, u32, u32, u32) {
        let span_x = self.max_x - self.min_x;
        let span_y = self.max_y - self.min_y;

        let w = self.map_dim(width, span_x).min(self.size);
        let h = self.map_dim(height, span_y).min(self.size);
        let max_anchor_x = self.size.saturating_sub(w).max(0);
        let max_anchor_y = self.size.saturating_sub(h).max(0);
        let anchor_x = self
            .map_pos(min_x, self.min_x, self.max_x)
            .min(max_anchor_x);
        let anchor_y = self
            .map_pos(min_y, self.min_y, self.max_y)
            .min(max_anchor_y);
        (anchor_x, anchor_y, w, h)
    }

    fn map_entity(&self, entity: &Entity) -> (u32, u32, u32, u32) {
        self.map_area(entity.min_x, entity.min_y, entity.width(), entity.height())
    }

    fn map_query(&self, rect: Rectangle) -> (u32, u32, u32, u32) {
        let min_x = rect.x - rect.width * 0.5;
        let min_y = rect.y - rect.height * 0.5;
        self.map_area(min_x, min_y, rect.width, rect.height)
    }

    fn map_point(&self, x: f32, y: f32) -> (u32, u32) {
        (
            self.map_pos(x, self.min_x, self.max_x),
            self.map_pos(y, self.min_y, self.max_y),
        )
    }
}

struct BenchResult {
    name: &'static str,
    collide_ms: Option<f64>,
    update_ms: f64,
    relocate_ms: f64,
    normalize_ms: f64,
    query_ms: f64,
    tick_total_ms: f64,
    tick_total_with_collide_ms: Option<f64>,
    notes: &'static str,
}

fn randf(rng: &mut StdRng) -> f32 {
    rng.r#gen::<f32>()
}

fn gen_radius(rng: &mut StdRng) -> f32 {
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
    let a_half = rect_to_half_extent(entity_a);
    let b_half = rect_to_half_extent(entity_b);

    let diff_x = a_half.x - b_half.x;
    let diff_y = a_half.y - b_half.y;
    let overlap_x = (a_half.w + b_half.w) - diff_x.abs();
    let overlap_y = (a_half.h + b_half.h) - diff_y.abs();

    if overlap_x > 0.0 && overlap_y > 0.0 {
        let size_a = a_half.w * a_half.h * 4.0;
        let size_b = b_half.w * b_half.h * 4.0;
        let total_size = size_a + size_b;

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

    unsafe {
        let ptr = entities.as_mut_ptr();
        let (a_ptr, b_ptr) = if a < b {
            (ptr.add(a), ptr.add(b))
        } else {
            (ptr.add(b), ptr.add(a))
        };
        collide_entities(&mut *a_ptr, &mut *b_ptr);
    }
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn generate_entities(seed: u64, bounds: Bounds, count: usize) -> Vec<Entity> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut entities = Vec::with_capacity(count);

    for i in 0..count {
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

        entities.push(Entity {
            min_x,
            max_x: min_x + w,
            min_y,
            max_y: min_y + h,
            vx: (1.0 - 2.0 * randf(&mut rng)) * INITIAL_VELOCITY,
            vy: (1.0 - 2.0 * randf(&mut rng)) * INITIAL_VELOCITY,
        });
    }

    entities
}

fn bench_min_size() -> f32 {
    std::env::var("BOLT_MIN_SIZE")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(MIN_SIZE)
}

fn bench_looseness() -> f32 {
    std::env::var("BOLT_LOOSENESS")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(1.0)
}

fn bench_bolt(entities_seed: &[Entity], bounds: Bounds, ticks: usize) -> BenchResult {
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
        looseness: bench_looseness(),
        large_entity_threshold_factor: 0.0,
        profile_summary: false,
        profile_detail: false,
        profile_limit: 5,
    };
    let no_collide = env::var("BOLT_BENCH_NO_COLLIDE")
        .ok()
        .map(|value| value == "1")
        .unwrap_or(false);
    let use_raw = env::var("BOLT_BENCH_RAW")
        .ok()
        .map(|value| value == "1")
        .unwrap_or(true);

    let mut entities = entities_seed.to_vec();
    let mut quadtree = BoltQuadTree::new_with_config(
        Rectangle {
            x: 0.0,
            y: 0.0,
            width: ARENA_WIDTH,
            height: ARENA_HEIGHT,
        },
        config,
    );

    for (i, entity) in entities.iter().enumerate() {
        if use_raw {
            quadtree.insert_rect_extent(
                i as u32,
                entity.min_x,
                entity.min_y,
                entity.max_x,
                entity.max_y,
                None,
            );
        } else {
            quadtree.insert(i as u32, ShapeEnum::Rectangle(entity.to_rectangle()), None);
        }
    }

    let mut collide_total = Duration::ZERO;
    let mut update_total = Duration::ZERO;
    let mut relocate_total = Duration::ZERO;
    let mut normalize_total = Duration::ZERO;
    let mut query_total = Duration::ZERO;

    let query_count = QUERIES_NUM.min(entities.len());
    for _ in 0..ticks {
        let start = Instant::now();
        if no_collide {
            quadtree.for_each_collision_pair(|_, _| {});
        } else {
            quadtree.for_each_collision_pair(|a, b| resolve_pair(&mut entities, a, b));
        }
        collide_total += start.elapsed();

        let start = Instant::now();
        for entity in entities.iter_mut() {
            update_entity(entity, &bounds);
        }
        update_total += start.elapsed();

        let start = Instant::now();
        if use_raw {
            for (value, entity) in entities.iter().enumerate() {
                quadtree.relocate_rect_extent(
                    value as u32,
                    entity.min_x,
                    entity.min_y,
                    entity.max_x,
                    entity.max_y,
                    None,
                );
            }
        } else {
            for (value, entity) in entities.iter().enumerate() {
                quadtree.relocate(
                    value as u32,
                    ShapeEnum::Rectangle(entity.to_rectangle()),
                    None,
                );
            }
        }
        relocate_total += start.elapsed();

        let start = Instant::now();
        quadtree.update();
        normalize_total += start.elapsed();

        let start = Instant::now();
        let entities_ptr = entities.as_ptr();
        // Safety: query_count <= entities.len()
        if use_raw {
            let q_half_w = QUERY_WIDTH * 0.5;
            let q_half_h = QUERY_HEIGHT * 0.5;
            for i in 0..query_count {
                let entity = unsafe { *entities_ptr.add(i) };
                quadtree.collisions_rect_extent_with_mut(
                    entity.min_x - q_half_w,
                    entity.min_y - q_half_h,
                    entity.max_x + q_half_w,
                    entity.max_y + q_half_h,
                    |_| {},
                );
            }
        } else {
            for i in 0..query_count {
                let entity = unsafe { &*entities_ptr.add(i) };
                quadtree.collisions_with(ShapeEnum::Rectangle(entity.query_rectangle()), |_| {});
            }
        }
        query_total += start.elapsed();
    }

    #[cfg(feature = "query_stats")]
    let query_stats = quadtree.take_query_stats();
    #[cfg(feature = "query_stats")]
    let entity_node_stats = quadtree.entity_node_stats();

    let ticks_f = ticks as f64;
    let collide_ms = duration_ms(collide_total) / ticks_f;
    let update_ms = duration_ms(update_total) / ticks_f;
    let relocate_ms = duration_ms(relocate_total) / ticks_f;
    let normalize_ms = duration_ms(normalize_total) / ticks_f;
    let query_ms = duration_ms(query_total) / ticks_f;
    let tick_total_ms = update_ms + relocate_ms + normalize_ms;

    #[cfg(feature = "query_stats")]
    {
        let queries = query_stats.query_calls.max(1);
        println!(
            "  Query stats: nodes/query {:.2}, entities/query {:.2}",
            query_stats.node_visits as f64 / queries as f64,
            query_stats.entity_visits as f64 / queries as f64
        );
        println!(
            "  Entity nodes avg {:.2}, max {}",
            entity_node_stats.0, entity_node_stats.1
        );
    }

    let (node_count, node_entities_count, entity_count_out) = quadtree.storage_counts();
    println!(
        "  Storage: nodes {}, node_entities {}, entities {}",
        node_count, node_entities_count, entity_count_out
    );

    BenchResult {
        name: "bolt/quadtree",
        collide_ms: Some(collide_ms),
        update_ms,
        relocate_ms,
        normalize_ms,
        query_ms,
        tick_total_ms,
        tick_total_with_collide_ms: Some(tick_total_ms + collide_ms),
        notes: "full collide + relocate + normalize",
    }
}

fn bench_quadtree_f32(entities_seed: &[Entity], bounds: Bounds, ticks: usize) -> BenchResult {
    let capped = entities_seed.len() > QUADTREE_F32_MAX_ENTITIES;
    let limit = entities_seed.len().min(QUADTREE_F32_MAX_ENTITIES);
    let mut entities = entities_seed[..limit].to_vec();
    let mut tree = QtF32::new();
    let mut items = Vec::with_capacity(entities.len());

    for (i, entity) in entities.iter().enumerate() {
        let item = Item::Point(F32Point::new(entity.center_x(), entity.center_y()));
        tree.insert(ItemId(i), item);
        items.push(item);
    }

    let mut update_total = Duration::ZERO;
    let mut relocate_total = Duration::ZERO;
    let mut query_total = Duration::ZERO;

    let query_count = QUERIES_NUM.min(entities.len());
    for _ in 0..ticks {
        let start = Instant::now();
        for entity in entities.iter_mut() {
            update_entity(entity, &bounds);
        }
        update_total += start.elapsed();

        let start = Instant::now();
        for (i, entity) in entities.iter().enumerate() {
            let new_item = Item::Point(F32Point::new(entity.center_x(), entity.center_y()));
            let id = ItemId(i);
            let old_item = items[i];
            tree.remove(id, old_item);
            tree.insert(id, new_item);
            items[i] = new_item;
        }
        relocate_total += start.elapsed();

        let start = Instant::now();
        let entities_ptr = entities.as_ptr();
        // Safety: query_count <= entities.len()
        for i in 0..query_count {
            let rect = unsafe { (*entities_ptr.add(i)).query_rectangle() };
            let query = RectF32 {
                min_x: rect.x - rect.width * 0.5,
                min_y: rect.y - rect.height * 0.5,
                max_x: rect.x + rect.width * 0.5,
                max_y: rect.y + rect.height * 0.5,
            };
            let results = tree.get_ids_that_overlap(&query);
            black_box(results.len());
        }
        query_total += start.elapsed();
    }

    let ticks_f = ticks as f64;
    let update_ms = duration_ms(update_total) / ticks_f;
    let relocate_ms = duration_ms(relocate_total) / ticks_f;
    let query_ms = duration_ms(query_total) / ticks_f;
    let tick_total_ms = update_ms + relocate_ms;

    BenchResult {
        name: "quadtree-f32",
        collide_ms: None,
        update_ms,
        relocate_ms,
        normalize_ms: 0.0,
        query_ms,
        tick_total_ms,
        tick_total_with_collide_ms: None,
        notes: if capped {
            "relocate = remove+insert per entity; capped at 30k to avoid stack overflow"
        } else {
            "relocate = remove+insert per entity"
        },
    }
}

fn bench_quadtree_rs(
    entities_seed: &[Entity],
    bounds: Bounds,
    grid_depth: u8,
    ticks: usize,
) -> BenchResult {
    let mapper = GridMapper::new(bounds, grid_depth);
    let mut entities = entities_seed.to_vec();
    let mut tree = QtRs::<u32, u32>::new(grid_depth as usize);
    let mut handles = Vec::with_capacity(entities.len());

    for (i, entity) in entities.iter().enumerate() {
        let (x, y, w, h) = mapper.map_entity(entity);
        let area = AreaBuilder::default()
            .anchor(RsPoint { x, y })
            .dimensions((w, h))
            .build()
            .unwrap();
        let handle = tree.insert(area, i as u32).unwrap();
        handles.push(handle);
    }

    let mut update_total = Duration::ZERO;
    let mut relocate_total = Duration::ZERO;
    let mut query_total = Duration::ZERO;

    let query_count = QUERIES_NUM.min(entities.len());
    for _ in 0..ticks {
        let start = Instant::now();
        for entity in entities.iter_mut() {
            update_entity(entity, &bounds);
        }
        update_total += start.elapsed();

        let start = Instant::now();
        for (i, entity) in entities.iter().enumerate() {
            let (x, y, w, h) = mapper.map_entity(entity);
            let area = AreaBuilder::default()
                .anchor(RsPoint { x, y })
                .dimensions((w, h))
                .build()
                .unwrap();
            let handle = handles[i];
            tree.delete_by_handle(handle);
            let new_handle = tree.insert(area, i as u32).unwrap();
            handles[i] = new_handle;
        }
        relocate_total += start.elapsed();

        let start = Instant::now();
        let entities_ptr = entities.as_ptr();
        // Safety: query_count <= entities.len()
        for i in 0..query_count {
            let rect = unsafe { (*entities_ptr.add(i)).query_rectangle() };
            let (x, y, w, h) = mapper.map_query(rect);
            let area = AreaBuilder::default()
                .anchor(RsPoint { x, y })
                .dimensions((w, h))
                .build()
                .unwrap();
            let mut count = 0usize;
            for _ in tree.query(area) {
                count += 1;
            }
            black_box(count);
        }
        query_total += start.elapsed();
    }

    let ticks_f = ticks as f64;
    let update_ms = duration_ms(update_total) / ticks_f;
    let relocate_ms = duration_ms(relocate_total) / ticks_f;
    let query_ms = duration_ms(query_total) / ticks_f;
    let tick_total_ms = update_ms + relocate_ms;

    BenchResult {
        name: "quadtree_rs",
        collide_ms: None,
        update_ms,
        relocate_ms,
        normalize_ms: 0.0,
        query_ms,
        tick_total_ms,
        tick_total_with_collide_ms: None,
        notes: "coords quantized to grid; relocate = delete+insert by handle",
    }
}

fn bench_quadtree_generic(entities_seed: &[Entity], bounds: Bounds, ticks: usize) -> BenchResult {
    let max_depth = (ARENA_WIDTH / bench_min_size()).log2().ceil() as usize;
    let bound = QtRect::new(
        vec2(bounds.min_x, bounds.min_y),
        vec2(bounds.max_x, bounds.max_y),
    );

    let mut entities = entities_seed.to_vec();
    let mut positions: Vec<Vec2> = entities
        .iter()
        .map(|entity| vec2(entity.center_x(), entity.center_y()))
        .collect();

    let mut update_total = Duration::ZERO;
    let mut relocate_total = Duration::ZERO;
    let mut query_total = Duration::ZERO;

    let query_count = QUERIES_NUM.min(entities.len());
    for _ in 0..ticks {
        let start = Instant::now();
        for entity in entities.iter_mut() {
            update_entity(entity, &bounds);
        }
        update_total += start.elapsed();

        let start = Instant::now();
        for (pos, entity) in positions.iter_mut().zip(entities.iter()) {
            *pos = vec2(entity.center_x(), entity.center_y());
        }
        let mut tree = QtGeneric::new(bound, 13, max_depth);
        tree.insert_many(&positions);
        relocate_total += start.elapsed();

        let start = Instant::now();
        let entities_ptr = entities.as_ptr();
        // Safety: query_count <= entities.len()
        for i in 0..query_count {
            let rect = unsafe { (*entities_ptr.add(i)).query_rectangle() };
            let query = QtRect::new(
                vec2(rect.x - rect.width * 0.5, rect.y - rect.height * 0.5),
                vec2(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5),
            );
            let results = tree.query(&query);
            black_box(results.len());
        }
        query_total += start.elapsed();
    }

    let ticks_f = ticks as f64;
    let update_ms = duration_ms(update_total) / ticks_f;
    let relocate_ms = duration_ms(relocate_total) / ticks_f;
    let query_ms = duration_ms(query_total) / ticks_f;
    let tick_total_ms = update_ms + relocate_ms;

    BenchResult {
        name: "quadtree (points)",
        collide_ms: None,
        update_ms,
        relocate_ms,
        normalize_ms: 0.0,
        query_ms,
        tick_total_ms,
        tick_total_with_collide_ms: None,
        notes: "rebuild tree each tick; points only",
    }
}

fn bench_spatialtree(
    entities_seed: &[Entity],
    bounds: Bounds,
    grid_depth: u8,
    ticks: usize,
) -> BenchResult {
    let mapper = GridMapper::new(bounds, grid_depth);
    let mut entities = entities_seed.to_vec();
    let mut positions: Vec<QuadVec<u32>> = entities
        .iter()
        .map(|entity| {
            let (x, y) = mapper.map_point(entity.center_x(), entity.center_y());
            QuadVec::build(x, y, grid_depth)
        })
        .collect();

    let mut update_total = Duration::ZERO;
    let mut relocate_total = Duration::ZERO;
    let mut query_total = Duration::ZERO;

    let query_count = QUERIES_NUM.min(entities.len());
    for _ in 0..ticks {
        let start = Instant::now();
        for entity in entities.iter_mut() {
            update_entity(entity, &bounds);
        }
        update_total += start.elapsed();

        let start = Instant::now();
        for (pos, entity) in positions.iter_mut().zip(entities.iter()) {
            let (x, y) = mapper.map_point(entity.center_x(), entity.center_y());
            *pos = QuadVec::build(x, y, grid_depth);
        }
        let mut tree = SpatialQuadTree::<(), QuadVec<u32>>::new();
        tree.insert_many(positions.iter().copied(), |_| ());
        relocate_total += start.elapsed();

        let start = Instant::now();
        let entities_ptr = entities.as_ptr();
        // Safety: query_count <= entities.len()
        for i in 0..query_count {
            let rect = unsafe { (*entities_ptr.add(i)).query_rectangle() };
            let (x, y, w, h) = mapper.map_query(rect);
            let min = QuadVec::build(x, y, grid_depth);
            let max_x = x.saturating_add(w).min(mapper.size.saturating_sub(1));
            let max_y = y.saturating_add(h).min(mapper.size.saturating_sub(1));
            let max = QuadVec::build(max_x, max_y, grid_depth);
            let mut count = 0usize;
            for _ in tree.iter_chunks_in_aabb(min, max) {
                count += 1;
            }
            black_box(count);
        }
        query_total += start.elapsed();
    }

    let ticks_f = ticks as f64;
    let update_ms = duration_ms(update_total) / ticks_f;
    let relocate_ms = duration_ms(relocate_total) / ticks_f;
    let query_ms = duration_ms(query_total) / ticks_f;
    let tick_total_ms = update_ms + relocate_ms;

    BenchResult {
        name: "spatialtree",
        collide_ms: None,
        update_ms,
        relocate_ms,
        normalize_ms: 0.0,
        query_ms,
        tick_total_ms,
        tick_total_with_collide_ms: None,
        notes: "quantized points; rebuild tree each tick",
    }
}

fn print_result(result: &BenchResult) {
    println!("{}", result.name);
    if let Some(collide_ms) = result.collide_ms {
        println!("  Collide: {:.02}ms", collide_ms);
    } else {
        println!("  Collide: N/A");
    }
    println!("  Update: {:.02}ms", result.update_ms);
    println!("  Relocate: {:.02}ms", result.relocate_ms);
    println!("  Normalize: {:.02}ms", result.normalize_ms);
    println!("  Tick total (no collide): {:.02}ms", result.tick_total_ms);
    if let Some(total) = result.tick_total_with_collide_ms {
        println!("  Tick total: {:.02}ms", total);
    }
    println!("  1k Queries: {:.02}ms", result.query_ms);
    if !result.notes.is_empty() {
        println!("  Notes: {}", result.notes);
    }
    println!();
}

fn should_run(filter: Option<&str>, name: &str) -> bool {
    match filter {
        None => true,
        Some(filter) => filter
            .split(',')
            .map(|entry| entry.trim())
            .filter(|entry| !entry.is_empty())
            .any(|entry| name.contains(entry)),
    }
}

fn main() {
    let bounds = Bounds {
        min_x: -ARENA_WIDTH * 0.5,
        max_x: ARENA_WIDTH * 0.5,
        min_y: -ARENA_HEIGHT * 0.5,
        max_y: ARENA_HEIGHT * 0.5,
    };

    let ticks = env::var("BOLT_BENCH_TICKS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(MEASURE_TICKS);
    let grid_depth = (ARENA_WIDTH / bench_min_size()).log2().ceil() as u8;
    let entity_count = env::var("BOLT_BENCH_ENTITIES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(ITER);
    let filter = env::var("BOLT_BENCH_FILTER")
        .ok()
        .or_else(|| Some("bolt,c-quadtree".to_string()));

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
    println!("Seed: {}", 36207250);
    println!("Measure ticks:    {}", ticks);
    println!("Entity count:     {}", entity_count);
    println!("Grid depth:       {}", grid_depth);
    println!();

    let entities_seed = generate_entities(36207250, bounds, entity_count);

    let mut results = Vec::new();
    if should_run(filter.as_deref(), "bolt") {
        results.push(bench_bolt(&entities_seed, bounds, ticks));
    }
    if should_run(filter.as_deref(), "quadtree-f32") {
        results.push(bench_quadtree_f32(&entities_seed, bounds, ticks));
    }
    if should_run(filter.as_deref(), "quadtree_rs") {
        results.push(bench_quadtree_rs(&entities_seed, bounds, grid_depth, ticks));
    }
    if should_run(filter.as_deref(), "quadtree (points)") {
        results.push(bench_quadtree_generic(&entities_seed, bounds, ticks));
    }
    if should_run(filter.as_deref(), "spatialtree") {
        results.push(bench_spatialtree(&entities_seed, bounds, grid_depth, ticks));
    }
    if should_run(filter.as_deref(), "c-quadtree") {
        match c_quadtree::run() {
            Ok(metrics) => {
                let tick_total_ms = metrics.update_ms + metrics.normalize_ms;
                #[cfg(feature = "query_stats")]
                if let (Some(nodes), Some(entities)) =
                    (metrics.query_nodes_per, metrics.query_entities_per)
                {
                    println!(
                        "  Query stats (c-quadtree): nodes/query {:.2}, entities/query {:.2}",
                        nodes, entities
                    );
                }
                if let (Some(nodes), Some(node_entities), Some(entities)) = (
                    metrics.node_count,
                    metrics.node_entities_count,
                    metrics.entity_count,
                ) {
                    println!(
                        "  Storage (c-quadtree): nodes {}, node_entities {}, entities {}",
                        nodes, node_entities, entities
                    );
                }
                results.push(BenchResult {
                    name: "c-quadtree",
                    collide_ms: Some(metrics.collide_ms),
                    update_ms: metrics.update_ms,
                    relocate_ms: 0.0,
                    normalize_ms: metrics.normalize_ms,
                    query_ms: metrics.query_ms,
                    tick_total_ms,
                    tick_total_with_collide_ms: Some(tick_total_ms + metrics.collide_ms),
                    notes: "external headless (fixed 1000 ticks)",
                });
            }
            Err(err) => {
                eprintln!("c-quadtree benchmark failed: {err}");
            }
        }
    }

    for result in &results {
        print_result(result);
    }
}
