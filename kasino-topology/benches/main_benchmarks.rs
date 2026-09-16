use criterion::{Criterion, criterion_group, criterion_main};

fn foo(_: &mut Criterion) {}

criterion_group!(benches, foo);
criterion_main!(benches);
