use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use quadtree::quadtree::{Config, QuadTree, RelocationRequest};
use common::shapes::{Rectangle, ShapeEnum};
use rand::prelude::*;

fn build_tree(
    rng: &mut impl Rng,
    num_items: usize,
    entity_type_range: u32,
    config: Option<Config>,
) -> (QuadTree, Vec<u32>) {
    let mut quadtree = match config {
        Some(config) => QuadTree::new_with_config(
            Rectangle {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            config,
        ),
        None => QuadTree::new(Rectangle {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        }),
    };

    let mut ids = Vec::with_capacity(num_items);
    for _ in 0..num_items {
        let shape = ShapeEnum::Rectangle(Rectangle {
            x: rng.gen_range(0.0..100.0),
            y: rng.gen_range(0.0..100.0),
            width: 2.0,
            height: 2.0,
        });
        let value = rng.gen();
        let entity_type = Some(rng.gen_range(0..entity_type_range));
        quadtree.insert(value, shape, entity_type);
        ids.push(value);
    }

    (quadtree, ids)
}

fn insert_benchmark(c: &mut Criterion) {
    let mut rng = rand::thread_rng();
    let mut quadtree = QuadTree::new(Rectangle {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    });

    c.bench_function("quadtree_insert", |b| {
        b.iter(|| {
            let shape = ShapeEnum::Rectangle(Rectangle {
                x: rng.gen_range(0.0..100.0),
                y: rng.gen_range(0.0..100.0),
                width: 5.0,
                height: 5.0,
            });
            quadtree.insert(black_box(rng.gen()), shape, None);
        })
    });
}

fn delete_benchmark(c: &mut Criterion) {
    let mut rng = rand::thread_rng();
    let mut quadtree = QuadTree::new(Rectangle {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    });
    let mut items = Vec::new();
    for _ in 0..1000 {
        let shape = ShapeEnum::Rectangle(Rectangle {
            x: rng.gen_range(0.0..100.0),
            y: rng.gen_range(0.0..100.0),
            width: 5.0,
            height: 5.0,
        });
        let value = rng.gen();
        quadtree.insert(value, shape, None);
        items.push(value);
    }

    c.bench_function("quadtree_delete", |b| {
        b.iter(|| {
            let index = rng.gen_range(0..items.len());
            quadtree.delete(black_box(items[index]));
        })
    });
}

fn relocate_benchmark(c: &mut Criterion) {
    let mut rng = rand::thread_rng();
    let mut quadtree = QuadTree::new(Rectangle {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    });
    let mut relocation_requests = Vec::new();
    for _ in 0..1000 {
        let shape = ShapeEnum::Rectangle(Rectangle {
            x: rng.gen_range(0.0..100.0),
            y: rng.gen_range(0.0..100.0),
            width: 5.0,
            height: 5.0,
        });
        let value = rng.gen();
        quadtree.insert(value, shape.clone(), None);
        relocation_requests.push(RelocationRequest {
            value,
            shape,
            entity_type: None,
        });
    }

    c.bench_function("quadtree_relocate", |b| {
        b.iter(|| {
            quadtree.relocate_batch(black_box(relocation_requests.clone()));
        })
    });
}

fn collisions_benchmark(c: &mut Criterion) {
    let mut rng = rand::thread_rng();
    let mut quadtree = QuadTree::new(Rectangle {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    });
    // Insert random items into the quadtree
    for _ in 0..1000 {
        let shape = ShapeEnum::Rectangle(Rectangle {
            x: rng.gen_range(0.0..100.0),
            y: rng.gen_range(0.0..100.0),
            width: 5.0,
            height: 5.0,
        });
        quadtree.insert(rng.gen(), shape, None);
    }

    // Define a query rectangle
    let query_shape = ShapeEnum::Rectangle(Rectangle {
        x: 40.0,
        y: 40.0,
        width: 20.0,
        height: 20.0,
    });

    c.bench_function("quadtree_collisions", |b| {
        b.iter(|| {
            let mut _collisions: Vec<u32> = Vec::new();
            quadtree.collisions(black_box(query_shape.clone()), &mut _collisions);
        })
    });
}

fn collisions_filter_large_filter_benchmark(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(42);
    let (quadtree, _) = build_tree(&mut rng, 5000, 512, None);
    let filter: Vec<u32> = (0..512).collect();

    let query_shape = ShapeEnum::Rectangle(Rectangle {
        x: 45.0,
        y: 45.0,
        width: 30.0,
        height: 30.0,
    });

    c.bench_function("quadtree_collisions_filter_large_filter", |b| {
        b.iter(|| {
            let mut collisions = Vec::new();
            quadtree.collisions_filter(
                black_box(query_shape.clone()),
                Some(filter.clone()),
                &mut collisions,
            );
            black_box(collisions);
        })
    });
}

fn collisions_batch_filter_large_filter_benchmark(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(43);
    let (quadtree, _) = build_tree(&mut rng, 5000, 512, None);
    let filter: Vec<u32> = (0..512).collect();
    let mut shapes = Vec::with_capacity(128);
    for _ in 0..128 {
        shapes.push(ShapeEnum::Rectangle(Rectangle {
            x: rng.gen_range(0.0..100.0),
            y: rng.gen_range(0.0..100.0),
            width: 5.0,
            height: 5.0,
        }));
    }

    c.bench_function("quadtree_collisions_batch_filter_large_filter", |b| {
        b.iter(|| {
            let result = quadtree.collisions_batch_filter(black_box(shapes.clone()), Some(filter.clone()));
            black_box(result);
        })
    });
}

fn cleanup_merge_benchmark(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(99);
    let config = Config {
        pool_size: 8000,
        node_capacity: 4,
        max_depth: 8,
    };

    c.bench_function("quadtree_cleanup_merge", |b| {
        b.iter_batched(
            || build_tree(&mut rng, 8000, 32, Some(config.clone())),
            |(mut quadtree, ids)| {
                for id in ids {
                    quadtree.delete(black_box(id));
                }
            },
            BatchSize::SmallInput,
        )
    });
}

fn comprehensive_benchmark(c: &mut Criterion) {
    let mut rng = rand::thread_rng();
    let mut quadtree_groups: Vec<Vec<QuadTree>> = Vec::new();

    // Create 101 groups of 5 quadtrees
    for _ in 0..101 {
        let mut quadtree_group: Vec<QuadTree> = Vec::new();
        for _ in 0..5 {
            let quadtree = QuadTree::new(Rectangle {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            });
            quadtree_group.push(quadtree);
        }
        quadtree_groups.push(quadtree_group);
    }

    c.bench_function("quadtree_comprehensive", |b| {
        b.iter(|| {
            for quadtree_group in &mut quadtree_groups {
                for (i, quadtree) in quadtree_group.iter_mut().enumerate() {
                    // Perform relocate operations
                    let mut relocation_requests = Vec::new();
                    let num_relocate = match i {
                        0 | 1 | 2 => 2,
                        3 => rng.gen_range(30..=150),
                        4 => 100,
                        _ => unreachable!(),
                    };
                    for i in 0..num_relocate {
                        let shape = ShapeEnum::Rectangle(Rectangle {
                            x: rng.gen_range(0.0..100.0),
                            y: rng.gen_range(0.0..100.0),
                            width: 5.0,
                            height: 5.0,
                        });
                        relocation_requests.push(RelocationRequest {
                            value: i,
                            shape: shape.clone(),
                            entity_type: None,
                        });
                    }
                    quadtree.relocate_batch(relocation_requests);

                    // Perform collision queries
                    let num_queries = match i {
                        0 => rng.gen_range(1..=4),
                        1 => rng.gen_range(1..=180),
                        2 => rng.gen_range(2..=45),
                        3 => rng.gen_range(1..=11),
                        4 => 1,
                        _ => unreachable!(),
                    };
                    for _ in 0..num_queries {
                        let query_shape = ShapeEnum::Rectangle(Rectangle {
                            x: rng.gen_range(0.0..100.0),
                            y: rng.gen_range(0.0..100.0),
                            width: 5.0,
                            height: 5.0,
                        });
                        let mut collisions: Vec<u32> = Vec::new();
                        quadtree.collisions(query_shape, &mut collisions);
                    }
                }
            }
        })
    });
}

criterion_group!(
    quadtree_benchmarks,
    insert_benchmark,
    delete_benchmark,
    relocate_benchmark,
    collisions_benchmark,
    collisions_filter_large_filter_benchmark,
    collisions_batch_filter_large_filter_benchmark,
    cleanup_merge_benchmark,
    comprehensive_benchmark
);
criterion_main!(quadtree_benchmarks);
