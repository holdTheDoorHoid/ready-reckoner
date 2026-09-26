//! The string-level engine (`RawEngine` in `web/src/engine/index.ts`): every function takes JSON
//! (or bytes) and returns the JSON of an envelope, `{"ok":true,"value":…}` or
//! `{"ok":false,"error":{code,message,details?}}` (`docs/ENGINE-API.md`). The WebAssembly exports
//! in the crate root are one-line wrappers around these, and the native tests call them directly.
//!
//! One engine per WebAssembly instance (per thread when run natively), created on the first call
//! and kept in a `RefCell`; `load_pack` adds files to it one at a time. No function panics on bad
//! input. If a bug ever panics inside a call, the instance traps, the JavaScript adapter turns the
//! trap into an `internal` error, and every later call answers `internal` with the panic message
//! instead of trapping again (the engine may be half-updated), so the page can say "reload".

use std::cell::RefCell;
use std::sync::Once;

use rr_plan::Engine;
use rr_types::{
    EngineError, EngineInfo, Envelope, ErrorCode, ExplainRequest, LocationInput, PlanInput,
    Problem, parse_json,
};
use serde::Serialize;

use crate::source::WasmSource;

/// The engine the exports talk to.
pub type WasmEngine = Engine<WasmSource>;

thread_local! {
    /// `None` until the first call; then the engine, or the error that stopped it being built.
    static ENGINE: RefCell<Option<Result<WasmEngine, EngineError>>> = const { RefCell::new(None) };
    /// The message of the last panic, reported by the calls that follow it.
    static LAST_PANIC: RefCell<Option<String>> = const { RefCell::new(None) };
}

static PANIC_HOOK: Once = Once::new();

/// Keeps the message of any panic for the calls that follow it, then runs the previous hook.
fn install_panic_hook() {
    PANIC_HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let message = info.to_string();
            let _ = LAST_PANIC.try_with(|p| {
                if let Ok(mut p) = p.try_borrow_mut() {
                    *p = Some(message);
                }
            });
            previous(info);
        }));
    });
}

/// The error every call returns after a panic left the engine unusable.
fn stopped() -> EngineError {
    let reason = LAST_PANIC
        .try_with(|p| p.try_borrow().ok().and_then(|p| p.clone()))
        .ok()
        .flatten()
        .unwrap_or_else(|| "the engine is busy with another call".to_owned());
    EngineError::new(
        ErrorCode::Internal,
        format!(
            "Something went wrong inside the planner. Reload the page; your answers are saved \
             in this browser. (Technical detail for a bug report: {reason})"
        ),
    )
}

/// Builds the engine on the first call.
fn ensure_engine(
    cell: &RefCell<Option<Result<WasmEngine, EngineError>>>,
) -> Result<(), EngineError> {
    if cell.try_borrow().map_err(|_| stopped())?.is_some() {
        return Ok(());
    }
    install_panic_hook();
    let engine = WasmSource::new().and_then(Engine::new);
    *cell.try_borrow_mut().map_err(|_| stopped())? = Some(engine);
    Ok(())
}

/// Runs `f` on the engine and wraps its result in an envelope.
fn with_engine<T: Serialize>(f: impl FnOnce(&WasmEngine) -> Result<T, EngineError>) -> String {
    let result = ENGINE.with(|cell| {
        ensure_engine(cell)?;
        let guard = cell.try_borrow().map_err(|_| stopped())?;
        match guard.as_ref() {
            Some(Ok(engine)) => f(engine),
            Some(Err(e)) => Err(e.clone()),
            None => Err(stopped()),
        }
    });
    Envelope::from(result).to_json_string()
}

/// Runs `f` on the engine with write access (loading packs) and wraps its result.
fn with_engine_mut<T: Serialize>(
    f: impl FnOnce(&mut WasmEngine) -> Result<T, EngineError>,
) -> String {
    let result = ENGINE.with(|cell| {
        ensure_engine(cell)?;
        let mut guard = cell.try_borrow_mut().map_err(|_| stopped())?;
        match guard.as_mut() {
            Some(Ok(engine)) => f(engine),
            Some(Err(e)) => Err(e.clone()),
            None => Err(stopped()),
        }
    });
    Envelope::from(result).to_json_string()
}

/// The contract's `EngineInfo` for the engine's current state. `data_pack_version` is absent
/// and `packs_loaded` is empty while the built-in sample counties answer.
pub fn info_of(engine: &WasmEngine) -> EngineInfo {
    let mut info = engine.engine_info();
    info.data_pack_version = engine.store().loaded_pack_version();
    info
}

/// Checks a location the way `assess` checks the location of a whole input, with the same
/// problem codes, fields and messages: it validates a default input carrying this location and
/// keeps the location's problems (the defaults are otherwise valid).
pub fn validate_location(location: &LocationInput) -> Result<(), EngineError> {
    let mut probe = PlanInput::defaults();
    probe.location = location.clone();
    let problems: Vec<Problem> = probe
        .validate()
        .into_iter()
        .filter(|p| p.field == "location" || p.field.starts_with("location."))
        .collect();
    if problems.is_empty() {
        Ok(())
    } else {
        Err(EngineError::bad_input(problems))
    }
}

/// `engine_info()`: versions, what is loaded, and the credit lines the app must show.
pub fn engine_info() -> String {
    with_engine(|e| Ok(info_of(e)))
}

/// `load_pack(name, bytes)`: one data file, named by its path in `data/manifest.json`.
pub fn load_pack(name: &str, bytes: &[u8]) -> String {
    with_engine_mut(|e| e.store_mut().load_pack(name, bytes))
}

/// `county_search(query)`: up to ten counties, best first.
pub fn county_search(query: &str) -> String {
    with_engine(|e| match e.store().missing() {
        Some(err) => Err(err),
        None => Ok(e.county_search(query)),
    })
}

/// `resolve_location(json)`: a `LocationInput` to a `LocationResolved`.
pub fn resolve_location(input_json: &str) -> String {
    with_engine(|e| {
        let input: LocationInput = parse_json(input_json)?;
        validate_location(&input)?;
        e.resolve_location(&input)
    })
}

/// `assess(json)`: a `PlanInput` to the whole `PlanOutput`.
pub fn assess(input_json: &str) -> String {
    with_engine(|e| e.assess(&PlanInput::from_json(input_json)?))
}

/// `explain(json)`: an `ExplainRequest` to an `Explanation`.
pub fn explain(request_json: &str) -> String {
    with_engine(|e| e.explain_request(&parse_json::<ExplainRequest>(request_json)?))
}

/// `catalogue()`: items, citations, guidance and the plain name of every id.
pub fn catalogue() -> String {
    with_engine(|e| Ok(e.catalogue()))
}

/// `defaults()`: the input skeleton the interview starts from.
pub fn defaults() -> String {
    with_engine(|e| Ok(e.defaults()))
}
