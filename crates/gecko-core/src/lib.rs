//! GECKO core: geometry, layouts, corpora, metrics, scoring and optimizers.
//! No CLI or HTTP code lives here. See `context/architecture.md`.

// Table code indexes several parallel arrays by the same symbol/slot id; range loops read clearer.
#![allow(clippy::needless_range_loop)]

pub mod bound;
pub mod calibrate;
pub mod certify;
pub mod cmaes;
pub mod corpus;
pub mod error;
pub mod explore;
pub mod geometry;
pub mod geomsearch;
pub mod hand;
pub mod hash;
pub mod interop;
pub mod layout;
pub mod magic;
pub mod metrics;
pub mod optimize;
pub mod profile;
pub mod report;
pub mod rng;
pub mod robust;
pub mod run;
pub mod scorer;
pub mod select;
pub mod store;

pub use corpus::Corpus;
pub use error::{Error, Result};
pub use geometry::flat2d::Flat;
pub use geometry::parametric::ParametricGeometry;
pub use geometry::{Finger, Geometry, Hand, Key, LayerMode};
pub use layout::Layout;
pub use profile::Profile;
pub use scorer::{Scorer, State};

/// Version of every JSON shape GECKO emits (`"schema": 1`).
pub const SCHEMA: u32 = 1;
