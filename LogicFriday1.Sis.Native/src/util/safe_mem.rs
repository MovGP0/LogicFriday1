//! Retirement note for `LogicSynthesis/sis/util/safe_mem.c`.
//!
//! The original C file provides SIS' `MMalloc`, `MMrealloc`, `MMfree`, and
//! `MMoutOfMemory` compatibility layer around the process allocator. That was
//! useful for C ports because `ALLOC`, `REALLOC`, and `FREE` macros needed
//! process-wide behavior for zero-sized allocations, null frees, and
//! out-of-memory exits.
//!
//! This Rust port intentionally does not recreate that allocator layer as a
//! normal design. Rust translations should use ownership and standard
//! containers such as `Vec`, `String`, `Box`, and `Option`, letting Rust's type
//! system and allocation APIs define memory behavior.
//!
//! At the time this bead was handled, no translated Rust module requires the
//! external SIS `MMalloc`/`MMrealloc`/`MMfree` ABI. If a future translation must
//! link against C-style SIS code that still calls those symbols, add a minimal
//! compatibility-only shim at that call boundary rather than introducing a
//! general Rust allocator abstraction.

/// Marker used by focused checks so this retirement file remains a valid Rust
/// module without exposing allocator ABI.
pub const SAFE_MEM_RETIRED: bool = true;

// TODO: if module wiring later exposes this file, keep it documentation-only
// unless translated code has a concrete compatibility dependency on the legacy
// SIS allocator symbols.
