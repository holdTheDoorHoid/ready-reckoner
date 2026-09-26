//! Shared helpers for rr-plan's tests.
#![allow(dead_code)]

use std::sync::OnceLock;

use rr_data::DataStore;
use rr_plan::{Assessment, Engine};
use rr_types::{PlanInput, PlanOutput};

/// One engine on the repository's data packs for the whole test binary.
pub fn engine() -> &'static Engine<DataStore> {
    static ENGINE: OnceLock<Engine<DataStore>> = OnceLock::new();
    ENGINE.get_or_init(|| {
        Engine::with_data_dir(rr_plan::golden::data_dir()).expect("the data packs in data/ load")
    })
}

/// One engine on the seven sample counties (no data packs).
pub fn fixture_engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| Engine::with_fixtures().expect("fixture engine"))
}

pub fn household(name: &str) -> PlanInput {
    rr_types::fixtures::get(name).unwrap_or_else(|| panic!("no fixture household {name}"))
}

pub fn assess(input: &PlanInput) -> PlanOutput {
    engine().assess(input).expect("assess")
}

pub fn run(input: &PlanInput) -> Assessment {
    engine().run(input).expect("run")
}

/// Every fixture's name, input and output (computed once).
pub fn outputs() -> &'static Vec<(&'static str, PlanInput, PlanOutput)> {
    static OUT: OnceLock<Vec<(&'static str, PlanInput, PlanOutput)>> = OnceLock::new();
    OUT.get_or_init(|| {
        rr_types::fixtures::all()
            .into_iter()
            .map(|(name, input)| {
                let out = engine()
                    .assess(&input)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                (name, input, out)
            })
            .collect()
    })
}

/// The citation numbers used in a packet ("[3]", "[3, 7]").
pub fn cited_numbers(packet: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let bytes = packet.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            if let Some(end) = packet[i + 1..].find(']') {
                let inner = &packet[i + 1..i + 1 + end];
                let nums: Option<Vec<usize>> =
                    inner.split(", ").map(|n| n.parse::<usize>().ok()).collect();
                if let Some(nums) = nums.filter(|n| !n.is_empty()) {
                    out.extend(nums);
                }
            }
        }
        i += 1;
    }
    out
}
