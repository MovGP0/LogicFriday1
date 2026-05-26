//! Native Rust age-summary support for the UCB BDD node heap.
//!
//! The legacy C implementation walks the active half-space blocks and prints a
//! distribution of node ages when age debugging is compiled in. This port keeps
//! that behavior as owned Rust data plus writer-based formatting. The old
//! compile-time-disabled path is represented by an explicit diagnostic string.

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::io::Write;

pub const LEGACY_AGE_BUCKETS: usize = 1000;

const AGE_DEBUG_DISABLED_MESSAGE: &str = "\
bdd_dump_node_ages: the bdd package is not compiled with DEBUG_AGE
\tso calling this function cannot produce any results
";

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddAge(pub usize);

impl BddAge {
    pub fn bucket(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddHeapNode {
    age: BddAge,
}

impl BddHeapNode {
    pub fn new(age: usize) -> Self {
        Self { age: BddAge(age) }
    }

    pub fn age(self) -> BddAge {
        self.age
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddNodeBlock {
    nodes: Vec<BddHeapNode>,
    used: usize,
}

impl BddNodeBlock {
    pub fn from_used_nodes<I>(nodes: I) -> Self
    where
        I: IntoIterator<Item = BddHeapNode>,
    {
        let nodes: Vec<_> = nodes.into_iter().collect();
        let used = nodes.len();

        Self { nodes, used }
    }

    pub fn with_capacity_nodes(nodes: Vec<BddHeapNode>, used: usize) -> Result<Self, NodeAgeError> {
        if used > nodes.len() {
            return Err(NodeAgeError::UsedExceedsBlockLength {
                used,
                length: nodes.len(),
            });
        }

        Ok(Self { nodes, used })
    }

    pub fn used(&self) -> usize {
        self.used
    }

    pub fn used_nodes(&self) -> &[BddHeapNode] {
        &self.nodes[..self.used]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeAgeReportStyle {
    HumanReadable,
    AutomatedStatistics,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NodeAgeDistribution {
    counts: BTreeMap<usize, usize>,
}

impl NodeAgeDistribution {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_blocks<'a, I>(blocks: I) -> Result<Self, NodeAgeError>
    where
        I: IntoIterator<Item = &'a BddNodeBlock>,
    {
        let mut distribution = Self::new();

        for block in blocks {
            distribution.add_block(block)?;
        }

        Ok(distribution)
    }

    pub fn add_node(&mut self, node: BddHeapNode) -> Result<(), NodeAgeError> {
        let age = node.age().bucket();
        if age >= LEGACY_AGE_BUCKETS {
            return Err(NodeAgeError::AgeOutOfLegacyRange {
                age,
                max_exclusive: LEGACY_AGE_BUCKETS,
            });
        }

        *self.counts.entry(age).or_insert(0) += 1;
        Ok(())
    }

    pub fn add_block(&mut self, block: &BddNodeBlock) -> Result<(), NodeAgeError> {
        for node in block.used_nodes() {
            self.add_node(*node)?;
        }

        Ok(())
    }

    pub fn count(&self, age: usize) -> usize {
        self.counts.get(&age).copied().unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.counts.iter().map(|(age, count)| (*age, *count))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeAgeError {
    AgeOutOfLegacyRange { age: usize, max_exclusive: usize },
    UsedExceedsBlockLength { used: usize, length: usize },
}

impl fmt::Display for NodeAgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AgeOutOfLegacyRange { age, max_exclusive } => {
                write!(
                    f,
                    "BDD node age {age} exceeds legacy bucket limit {max_exclusive}"
                )
            }
            Self::UsedExceedsBlockLength { used, length } => {
                write!(
                    f,
                    "BDD node block marks {used} nodes used but stores only {length}"
                )
            }
        }
    }
}

impl std::error::Error for NodeAgeError {}

pub fn age_debug_disabled_message() -> &'static str {
    AGE_DEBUG_DISABLED_MESSAGE
}

pub fn format_node_age_distribution(
    distribution: &NodeAgeDistribution,
    style: NodeAgeReportStyle,
) -> String {
    let mut output = Vec::new();
    write_node_age_distribution(distribution, style, &mut output)
        .expect("writing to Vec cannot fail");

    String::from_utf8(output).expect("BDD age report is ASCII")
}

pub fn write_node_age_distribution<W>(
    distribution: &NodeAgeDistribution,
    style: NodeAgeReportStyle,
    writer: &mut W,
) -> io::Result<()>
where
    W: Write,
{
    match style {
        NodeAgeReportStyle::HumanReadable => {
            write_human_readable_distribution(distribution, writer)
        }
        NodeAgeReportStyle::AutomatedStatistics => {
            write_automated_statistics_distribution(distribution, writer)
        }
    }
}

fn write_human_readable_distribution<W>(
    distribution: &NodeAgeDistribution,
    writer: &mut W,
) -> io::Result<()>
where
    W: Write,
{
    write!(
        writer,
        "\
Age Distribution in bdd_nodes
Age\tCount
"
    )?;

    for (age, count) in distribution.iter() {
        writeln!(writer, "{age}\t{count}")?;
    }

    Ok(())
}

fn write_automated_statistics_distribution<W>(
    distribution: &NodeAgeDistribution,
    writer: &mut W,
) -> io::Result<()>
where
    W: Write,
{
    writeln!(writer, "age-summary: start")?;
    for (age, count) in distribution.iter() {
        writeln!(writer, "age-summary: {age}\t{count}")?;
    }
    writeln!(writer, "age-summary: end")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_age_counts_from_used_nodes_only() {
        let first = BddNodeBlock::from_used_nodes([
            BddHeapNode::new(0),
            BddHeapNode::new(7),
            BddHeapNode::new(7),
        ]);
        let second = BddNodeBlock::with_capacity_nodes(
            vec![
                BddHeapNode::new(999),
                BddHeapNode::new(1),
                BddHeapNode::new(2),
            ],
            2,
        )
        .unwrap();

        let distribution = NodeAgeDistribution::from_blocks([&first, &second]).unwrap();

        assert_eq!(distribution.count(0), 1);
        assert_eq!(distribution.count(1), 1);
        assert_eq!(distribution.count(7), 2);
        assert_eq!(distribution.count(999), 1);
        assert_eq!(distribution.count(2), 0);
    }

    #[test]
    fn rejects_age_outside_legacy_fixed_bucket_range() {
        let block = BddNodeBlock::from_used_nodes([BddHeapNode::new(LEGACY_AGE_BUCKETS)]);

        let error = NodeAgeDistribution::from_blocks([&block]).unwrap_err();

        assert_eq!(
            error,
            NodeAgeError::AgeOutOfLegacyRange {
                age: LEGACY_AGE_BUCKETS,
                max_exclusive: LEGACY_AGE_BUCKETS,
            }
        );
    }

    #[test]
    fn validates_used_count_when_building_block_with_spare_capacity() {
        let error = BddNodeBlock::with_capacity_nodes(vec![BddHeapNode::new(1)], 2).unwrap_err();

        assert_eq!(
            error,
            NodeAgeError::UsedExceedsBlockLength { used: 2, length: 1 }
        );
    }

    #[test]
    fn human_readable_report_matches_legacy_shape() {
        let block = BddNodeBlock::from_used_nodes([
            BddHeapNode::new(3),
            BddHeapNode::new(3),
            BddHeapNode::new(8),
        ]);
        let distribution = NodeAgeDistribution::from_blocks([&block]).unwrap();

        let report = format_node_age_distribution(&distribution, NodeAgeReportStyle::HumanReadable);

        assert_eq!(
            report,
            "\
Age Distribution in bdd_nodes
Age\tCount
3\t2
8\t1
"
        );
    }

    #[test]
    fn automated_statistics_report_matches_legacy_shape() {
        let block = BddNodeBlock::from_used_nodes([
            BddHeapNode::new(1),
            BddHeapNode::new(4),
            BddHeapNode::new(4),
        ]);
        let distribution = NodeAgeDistribution::from_blocks([&block]).unwrap();

        let report =
            format_node_age_distribution(&distribution, NodeAgeReportStyle::AutomatedStatistics);

        assert_eq!(
            report,
            "\
age-summary: start
age-summary: 1\t1
age-summary: 4\t2
age-summary: end
"
        );
    }

    #[test]
    fn empty_distribution_still_prints_headers() {
        let distribution = NodeAgeDistribution::new();

        assert_eq!(
            format_node_age_distribution(&distribution, NodeAgeReportStyle::HumanReadable),
            "\
Age Distribution in bdd_nodes
Age\tCount
"
        );
        assert_eq!(
            format_node_age_distribution(&distribution, NodeAgeReportStyle::AutomatedStatistics),
            "\
age-summary: start
age-summary: end
"
        );
    }

    #[test]
    fn debug_disabled_diagnostic_preserves_legacy_message() {
        assert_eq!(
            age_debug_disabled_message(),
            "\
bdd_dump_node_ages: the bdd package is not compiled with DEBUG_AGE
\tso calling this function cannot produce any results
"
        );
    }

    #[test]
    fn writer_api_propagates_output_errors() {
        struct FailingWriter;

        impl Write for FailingWriter {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("sink failed"))
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let error = write_node_age_distribution(
            &NodeAgeDistribution::new(),
            NodeAgeReportStyle::HumanReadable,
            &mut FailingWriter,
        )
        .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::Other);
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_metadata_tokens_are_present() {
        let source = include_str!("dmp_node_age.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }
}
