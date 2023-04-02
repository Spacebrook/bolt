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
            quadtree.insert(black_box(rng.gen()), shape);
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
        quadtree.insert(value, shape);
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
        quadtree.insert(value, shape.clone());
        relocation_requests.push(RelocationRequest { value, shape });
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
        quadtree.insert(rng.gen(), shape);
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

criterion_group!(
    quadtree_benchmarks,
    insert_benchmark,
    delete_benchmark,
    relocate_benchmark,
    collisions_benchmark
);
criterion_main!(quadtree_benchmarks);
