use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crabrl::Parser;

fn parse_small_file(c: &mut Criterion) {
    let parser = Parser::new();
    let content = include_bytes!("../tests/fixtures/small.xml");

    c.bench_function("parse_small", |b| {
        b.iter(|| parser.parse_bytes(black_box(content)));
    });
}

fn parse_medium_file(c: &mut Criterion) {
    let parser = Parser::new();
    let content = include_bytes!("../tests/fixtures/medium.xml");

    c.bench_function("parse_medium", |b| {
        b.iter(|| parser.parse_bytes(black_box(content)));
    });
}

criterion_group!(benches, parse_small_file, parse_medium_file);
criterion_main!(benches);

