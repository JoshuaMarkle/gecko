//! Scorer benchmarks (`cargo bench -p gecko-core`): full evaluation, one swap delta, and swap
//! deltas/s across all cores. Uses the repo defaults (`gecko.toml`) with QWERTY. See
//! `context/testing.md`; `gecko perf --record` keeps the long-term history.

use std::hint::black_box;
use std::path::Path;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use gecko_core::rng::Rng;
use gecko_core::store::Store;
use gecko_core::Scorer;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn scorer() -> Scorer {
    let store = Store::open(Path::new(ROOT)).unwrap();
    let d = &store.defaults;
    let g = store.geometry(&d.geometry).unwrap();
    let c = store.corpus(&d.corpus).unwrap();
    let p = store.profile(&d.profile).unwrap();
    let l = store.layout("qwerty", &g.name).unwrap().fit(&g).unwrap();
    Scorer::new(&g, &c, &p, &l).unwrap()
}

/// `count` random swap deltas, applying improvements (the hill-climbing access pattern).
fn swaps(sc: &Scorer, seed: u64, count: u64) -> f64 {
    let mut st = sc.identity();
    let mut rng = Rng::new(seed);
    let mut sink = 0.0;
    for _ in 0..count {
        let a = rng.below(sc.n);
        let b = (a + 1 + rng.below(sc.n - 1)) % sc.n;
        let d = sc.delta(&st, a, b);
        sink += d;
        if d < 0.0 {
            sc.apply(&mut st, a, b, d);
        }
    }
    sink
}

fn benches(c: &mut Criterion) {
    let sc = scorer();
    let st = sc.identity();

    c.bench_function("full_eval", |b| b.iter(|| sc.full(black_box(&st))));

    let mut g = c.benchmark_group("swap_delta");
    g.throughput(Throughput::Elements(1));
    g.bench_function("single", |b| {
        let mut rng = Rng::new(1);
        b.iter(|| {
            let a = rng.below(sc.n);
            let bb = (a + 1 + rng.below(sc.n - 1)) % sc.n;
            sc.delta(black_box(&st), a, bb)
        })
    });
    g.finish();

    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    const PER_THREAD: u64 = 20_000;
    let mut g = c.benchmark_group("swaps_all_cores");
    g.throughput(Throughput::Elements(PER_THREAD * threads as u64));
    g.sample_size(10);
    g.bench_function(format!("{threads}_threads"), |b| {
        b.iter(|| {
            std::thread::scope(|s| {
                let hs: Vec<_> = (0..threads)
                    .map(|t| {
                        s.spawn({
                            let sc = &sc;
                            move || swaps(sc, t as u64 + 1, PER_THREAD)
                        })
                    })
                    .collect();
                hs.into_iter().map(|h| h.join().unwrap()).sum::<f64>()
            })
        })
    });
    g.finish();
}

criterion_group!(scorer_benches, benches);
criterion_main!(scorer_benches);
