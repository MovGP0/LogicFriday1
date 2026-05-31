//! Native Rust reporting for outstanding UCB BDD external pointers.
//!
//! The C routine walks the manager's external-reference subheaps, counts live
//! `bdd_t` handles by allocation origin, and optionally dumps each non-create
//! pointer. This port keeps the same reporting rules over owned Rust data so it
//! can be tested without exposing the legacy manager layout.

use std::io;
use std::io::Write;

pub const DEFAULT_EXTERNAL_POINTER_BLOCK_LEN: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddPointerValue {
    Zero,
    One,
    Nodes(usize),
}

impl BddPointerValue {
    fn legacy_description(&self) -> String {
        match self {
            Self::Zero => "the zero".to_string(),
            Self::One => "the one".to_string(),
            Self::Nodes(size) => {
                let suffix = if *size == 1 { "" } else { "s" };
                format!("bdd of {size} node{suffix}")
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddExternalPointer {
    value: BddPointerValue,
    origin: Option<String>,
}

impl BddExternalPointer {
    pub fn new(value: BddPointerValue) -> Self {
        Self {
            value,
            origin: None,
        }
    }

    pub fn with_origin(value: BddPointerValue, origin: impl Into<String>) -> Self {
        Self {
            value,
            origin: Some(origin.into()),
        }
    }

    pub fn value(&self) -> &BddPointerValue {
        &self.value
    }

    pub fn origin(&self) -> Option<&str> {
        self.origin.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExternalPointerSlot {
    Free,
    Live(BddExternalPointer),
}

impl ExternalPointerSlot {
    pub fn live(pointer: BddExternalPointer) -> Self {
        Self::Live(pointer)
    }

    pub fn is_free(&self) -> bool {
        matches!(self, Self::Free)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddExternalPointerBlock {
    slots: Vec<ExternalPointerSlot>,
}

impl BddExternalPointerBlock {
    pub fn new(slots: impl Into<Vec<ExternalPointerSlot>>) -> Self {
        Self {
            slots: slots.into(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: vec![ExternalPointerSlot::Free; capacity],
        }
    }

    pub fn slots(&self) -> &[ExternalPointerSlot] {
        &self.slots
    }

    pub fn slots_mut(&mut self) -> &mut [ExternalPointerSlot] {
        &mut self.slots
    }
}

impl Default for BddExternalPointerBlock {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_EXTERNAL_POINTER_BLOCK_LEN)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddExternalPointerManager {
    blocks: Vec<BddExternalPointerBlock>,
}

impl BddExternalPointerManager {
    pub fn new(blocks: impl Into<Vec<BddExternalPointerBlock>>) -> Self {
        Self {
            blocks: blocks.into(),
        }
    }

    pub fn blocks(&self) -> &[BddExternalPointerBlock] {
        &self.blocks
    }

    pub fn push_block(&mut self, block: BddExternalPointerBlock) {
        self.blocks.push(block);
    }

    pub fn live_pointers(&self) -> impl Iterator<Item = (usize, &BddExternalPointer)> {
        self.blocks
            .iter()
            .enumerate()
            .flat_map(|(block_index, block)| {
                let block_len = block.slots.len();
                block
                    .slots
                    .iter()
                    .enumerate()
                    .filter_map(move |(slot_index, slot)| match slot {
                        ExternalPointerSlot::Free => None,
                        ExternalPointerSlot::Live(pointer) => {
                            Some((slot_index + block_index * block_len, pointer))
                        }
                    })
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExternalPointerDumpOptions {
    pub dump_all: bool,
    pub automated_statistics: bool,
    pub track_origins: bool,
}

impl ExternalPointerDumpOptions {
    pub fn normal() -> Self {
        Self {
            dump_all: false,
            automated_statistics: false,
            track_origins: true,
        }
    }

    pub fn with_dump_all(mut self, dump_all: bool) -> Self {
        self.dump_all = dump_all;
        self
    }

    pub fn with_automated_statistics(mut self, automated_statistics: bool) -> Self {
        self.automated_statistics = automated_statistics;
        self
    }

    pub fn with_tracked_origins(mut self, track_origins: bool) -> Self {
        self.track_origins = track_origins;
        self
    }
}

impl Default for ExternalPointerDumpOptions {
    fn default() -> Self {
        Self::normal()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExternalPointerDumpSummary {
    pub create_bdd: usize,
    pub other: usize,
}

pub fn format_external_pointers(
    manager: &BddExternalPointerManager,
    options: ExternalPointerDumpOptions,
) -> String {
    let mut output = Vec::new();
    write_external_pointers(manager, options, &mut output)
        .expect("writing to String buffer cannot fail");
    String::from_utf8(output).expect("external pointer report is ASCII")
}

pub fn write_external_pointers<W>(
    manager: &BddExternalPointerManager,
    options: ExternalPointerDumpOptions,
    writer: &mut W,
) -> io::Result<ExternalPointerDumpSummary>
where
    W: Write,
{
    if options.dump_all && options.automated_statistics {
        writeln!(writer, "all-external-pointers: start")?;
    }

    let mut summary = ExternalPointerDumpSummary::default();

    for (index, pointer) in manager.live_pointers() {
        let origin = pointer_origin(pointer, options.track_origins);

        if origin == "bdd_create_bdd" {
            summary.create_bdd += 1;
            continue;
        }

        summary.other += 1;

        if options.dump_all {
            write_pointer_detail(writer, options, index, pointer, origin)?;
        }
    }

    if options.dump_all && options.automated_statistics {
        writeln!(writer, "all-external-pointers: end")?;
    }

    if options.automated_statistics {
        writeln!(
            writer,
            "external-pointers: bdd_create_bdd: {}: other: {}",
            summary.create_bdd, summary.other
        )?;
    } else {
        write!(
            writer,
            "\
Outstanding External Pointers
    due to bdd_create_bdd   {}
    due to other operations {}
",
            summary.create_bdd, summary.other
        )?;
    }

    Ok(summary)
}

fn pointer_origin(pointer: &BddExternalPointer, track_origins: bool) -> &str {
    if track_origins {
        pointer.origin().unwrap_or("unknown")
    } else {
        "unknown"
    }
}

fn write_pointer_detail<W>(
    writer: &mut W,
    options: ExternalPointerDumpOptions,
    index: usize,
    pointer: &BddExternalPointer,
    origin: &str,
) -> io::Result<()>
where
    W: Write,
{
    let value = pointer.value().legacy_description();

    if options.automated_statistics {
        writeln!(
            writer,
            "all-external-pointers: bdd_t: {index} free: false node: {value} origin: {origin}"
        )
    } else {
        writeln!(
            writer,
            "bdd_t[{index}] = {{ free: false, node: {value}, origin: {origin} }}"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_report_counts_create_and_other_live_pointers() {
        let manager = sample_manager();

        let report = format_external_pointers(&manager, ExternalPointerDumpOptions::normal());

        assert_eq!(
            report,
            "\
Outstanding External Pointers
    due to bdd_create_bdd   1
    due to other operations 3
"
        );
    }

    #[test]
    fn dump_all_prints_non_create_pointer_details() {
        let manager = sample_manager();

        let report = format_external_pointers(
            &manager,
            ExternalPointerDumpOptions::normal().with_dump_all(true),
        );

        assert!(report.contains("bdd_t[1] = { free: false, node: the one, origin: simplify }\n"));
        assert!(
            report.contains("bdd_t[3] = { free: false, node: bdd of 3 nodes, origin: unknown }\n")
        );
        assert!(
            report.contains("bdd_t[4] = { free: false, node: bdd of 1 node, origin: compose }\n")
        );
        assert!(!report.contains("bdd_t[0]"));
        assert!(report.ends_with("    due to other operations 3\n"));
    }

    #[test]
    fn automated_mode_uses_statistics_labels() {
        let manager = sample_manager();
        let options = ExternalPointerDumpOptions::normal()
            .with_dump_all(true)
            .with_automated_statistics(true);

        let report = format_external_pointers(&manager, options);

        assert!(report.starts_with("all-external-pointers: start\n"));
        assert!(report.contains(
            "all-external-pointers: bdd_t: 1 free: false node: the one origin: simplify\n"
        ));
        assert!(report.contains("all-external-pointers: end\n"));
        assert!(report.ends_with("external-pointers: bdd_create_bdd: 1: other: 3\n"));
    }

    #[test]
    fn untracked_origins_are_reported_as_unknown() {
        let manager = BddExternalPointerManager::new([BddExternalPointerBlock::new([
            ExternalPointerSlot::live(BddExternalPointer::with_origin(
                BddPointerValue::Zero,
                "bdd_create_bdd",
            )),
            ExternalPointerSlot::live(BddExternalPointer::with_origin(
                BddPointerValue::Nodes(2),
                "real_origin",
            )),
        ])]);

        let report = format_external_pointers(
            &manager,
            ExternalPointerDumpOptions::normal()
                .with_dump_all(true)
                .with_tracked_origins(false),
        );

        assert!(report.contains("bdd_t[0] = { free: false, node: the zero, origin: unknown }\n"));
        assert!(
            report.contains("bdd_t[1] = { free: false, node: bdd of 2 nodes, origin: unknown }\n")
        );
        assert!(report.ends_with("    due to other operations 2\n"));
    }

    #[test]
    fn live_pointer_indices_match_subheap_positions() {
        let manager = sample_manager();

        let live_indices: Vec<_> = manager
            .live_pointers()
            .map(|(index, pointer)| (index, pointer.value().clone()))
            .collect();

        assert_eq!(
            live_indices,
            vec![
                (0, BddPointerValue::Zero),
                (1, BddPointerValue::One),
                (3, BddPointerValue::Nodes(3)),
                (4, BddPointerValue::Nodes(1)),
            ]
        );
    }

    #[test]
    fn writer_errors_are_propagated() {
        struct FailingWriter;

        impl Write for FailingWriter {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("sink failed"))
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let error = write_external_pointers(
            &sample_manager(),
            ExternalPointerDumpOptions::normal().with_dump_all(true),
            &mut FailingWriter,
        )
        .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::Other);
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_metadata_tokens_are_present() {
        let source = include_str!("dmp_ext_ptrs.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }

    fn sample_manager() -> BddExternalPointerManager {
        BddExternalPointerManager::new([
            BddExternalPointerBlock::new([
                ExternalPointerSlot::live(BddExternalPointer::with_origin(
                    BddPointerValue::Zero,
                    "bdd_create_bdd",
                )),
                ExternalPointerSlot::live(BddExternalPointer::with_origin(
                    BddPointerValue::One,
                    "simplify",
                )),
                ExternalPointerSlot::Free,
            ]),
            BddExternalPointerBlock::new([
                ExternalPointerSlot::live(BddExternalPointer::new(BddPointerValue::Nodes(3))),
                ExternalPointerSlot::live(BddExternalPointer::with_origin(
                    BddPointerValue::Nodes(1),
                    "compose",
                )),
                ExternalPointerSlot::Free,
            ]),
        ])
    }
}
