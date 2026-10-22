//! Scorer benchmarks (`cargo bench -p gecko-core`): full evaluation, one swap delta, swap
//! deltas/s across all cores, and scorer build vs one-key rebuild (`Scorer::set_geometry`). Uses the repo defaults (`gecko.toml`) with QWERTY. See
//! `context/testing.md`; `gecko perf --record` keeps the long-term history.

use std::hint::black_box;
use std::path::Path;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use gecko_core::rng::Rng;
use gecko_core::store::Store;
use gecko_core::{Corpus, Geometry, Layout, Profile, Scorer};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn inputs() -> (Geometry, Corpus, Profile, Layout) {
    let store = Store::open(Path::new(ROOT)).unwrap();
    let d = &store.defaults;
    let g = store.geometry(&d.geometry).unwrap();
    let c = store.corpus(&d.corpus).unwrap();
    let p = store.profile(&d.profile).unwrap();
    let l = store.layout("qwerty", &g.name).unwrap().fit(&g).unwrap();
    (g, c, p, l)
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
    let (geo, corpus, profile, layout) = inputs();
    let sc = Scorer::new(&geo, &corpus, &profile, &layout).unwrap();
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

    let mut g = c.benchmark_group("tables");
    g.sample_size(20);
    g.bench_function("build", |b| b.iter(|| Scorer::new(&geo, &corpus, &profile, black_box(&layout)).unwrap()));
    g.bench_function("rebuild_one_key", |b| {
        let (mut moved, mut board, mut k) = (sc.clone(), geo.clone(), 0usize);
        b.iter(|| {
            let i = k % board.len();
            board.key[i].pos[0] += if k.is_multiple_of(2) { 0.01 } else { -0.01 };
            k += 1;
            moved.set_geometry(black_box(&board)).unwrap()
        })
    });
    g.finish();
}

criterion_group!(scorer_benches, benches);
criterion_main!(scorer_benches);
