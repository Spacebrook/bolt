use criterion::{black_box, criterion_group, criterion_main, Criterion};
use quadtree::quadtree::{QuadTree, RelocationRequest};
use quadtree::shapes::{Rectangle, ShapeEnum};
use rand::prelude::*;

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
        relocation_requests.push(RelocationRequest { value, shape, entity_type: None });
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
    comprehensive_benchmark
);
criterion_main!(quadtree_benchmarks);
