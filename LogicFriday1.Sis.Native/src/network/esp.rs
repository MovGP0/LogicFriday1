//! Native Rust port of the SIS `network/esp.c` network-level Espresso path.
//!
//! The C implementation converts a network to a PLA, refreshes the off-set,
//! invokes Espresso, and converts the minimized PLA back to a two-level
//! network. This Rust port keeps that shape for the owned `Network` model:
//! small networks are converted to exact on/DC covers, minimized with a
//! deterministic two-level reducer, and rebuilt as a two-level network.
//! Larger networks fall back to a valid duplicate with local cover cleanup
//! until the full Espresso core is available as native Rust.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use super::network_util::{
    BoolExpr, CoverValue, Cube, Network, NetworkNode, NetworkUtilError, NodeId, NodeKind, SopCover,
};

const DEFAULT_EXACT_INPUT_LIMIT: usize = 12;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkEspressoOptions
{
    exact_input_limit: usize,
}

impl NetworkEspressoOptions
{
    pub fn exact_input_limit(&self) -> usize
    {
        self.exact_input_limit
    }

    pub fn with_exact_input_limit(mut self, exact_input_limit: usize) -> Self
    {
        self.exact_input_limit = exact_input_limit;
        self
    }
}

impl Default for NetworkEspressoOptions
{
    fn default() -> Self
    {
        Self {
            exact_input_limit: DEFAULT_EXACT_INPUT_LIMIT,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct NetworkEspressoStats
{
    pub outputs_processed: usize,
    pub original_cubes: usize,
    pub minimized_cubes: usize,
    pub exact_minimization_used: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetworkEspressoResult
{
    pub network: Network,
    pub stats: NetworkEspressoStats,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkEspressoError
{
    EmptyNetwork,
    MissingOutputDriver(NodeId),
    MissingInputName(String),
    Network(NetworkUtilError),
}

impl fmt::Display for NetworkEspressoError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::EmptyNetwork => write!(formatter, "network must have at least one input and one output"),
            Self::MissingOutputDriver(node) => {
                write!(formatter, "primary output {} has no driver", node.index())
            }
            Self::MissingInputName(name) => write!(formatter, "missing input named {name}"),
            Self::Network(error) => error.fmt(formatter),
        }
    }
}

impl Error for NetworkEspressoError {}

impl From<NetworkUtilError> for NetworkEspressoError
{
    fn from(value: NetworkUtilError) -> Self
    {
        Self::Network(value)
    }
}

pub type NetworkEspressoResultValue<T> = Result<T, NetworkEspressoError>;

pub fn network_espresso(network: &Network) -> NetworkEspressoResultValue<Network>
{
    Ok(network_espresso_with_options(network, &NetworkEspressoOptions::default())?.network)
}

pub fn network_espresso_with_options(
    network: &Network,
    options: &NetworkEspressoOptions,
) -> NetworkEspressoResultValue<NetworkEspressoResult>
{
    if network.num_pi() == 0 || network.num_po() == 0
    {
        return Err(NetworkEspressoError::EmptyNetwork);
    }

    if network.num_pi() > options.exact_input_limit
    {
        return duplicate_with_local_cover_cleanup(network);
    }

    exact_network_espresso(network)
}

fn exact_network_espresso(network: &Network) -> NetworkEspressoResultValue<NetworkEspressoResult>
{
    let input_ids = network.primary_inputs().to_vec();
    let input_names = collect_node_names(network, &input_ids)?;
    let output_ids = network.primary_outputs().to_vec();
    let output_names = collect_node_names(network, &output_ids)?;
    let assignments = enumerate_assignments(input_ids.len());
    let dc_network = network.dc_network();

    let mut output_covers = Vec::with_capacity(output_ids.len());
    let mut stats = NetworkEspressoStats {
        outputs_processed: output_ids.len(),
        exact_minimization_used: true,
        ..NetworkEspressoStats::default()
    };

    for output in &output_ids
    {
        let driver = output_driver(network, *output)?;
        let mut on_set = BTreeSet::new();
        let mut dc_set = BTreeSet::new();

        for (assignment_index, assignment) in assignments.iter().enumerate()
        {
            let values = assignment_values(&input_ids, assignment);
            if eval_node(network, driver, &values, &mut BTreeMap::new(), &mut BTreeSet::new())?
            {
                on_set.insert(assignment_index);
            }
        }

        if let Some(dc_network) = dc_network
        {
            let dc_values = align_dc_assignments(network, dc_network, &input_ids, &assignments)?;
            let output_name = network.node(*output)?.name.clone();
            if let Some(dc_output) = dc_network.find_node(&output_name)
            {
                let dc_driver = output_driver(dc_network, dc_output)?;
                for (assignment_index, values) in dc_values.iter().enumerate()
                {
                    if eval_node(
                        dc_network,
                        dc_driver,
                        values,
                        &mut BTreeMap::new(),
                        &mut BTreeSet::new(),
                    )?
                    {
                        dc_set.insert(assignment_index);
                    }
                }
            }
        }

        stats.original_cubes += on_set.len();
        let minimized = minimize_truth_cover(input_ids.len(), &on_set, &dc_set);
        stats.minimized_cubes += minimized.len();
        output_covers.push(minimized);
    }

    let mut result = Network::new();
    result.set_name(network.name().to_owned());
    result.set_area(network.area(), network.area_given());
    result.set_default_delay(network.default_delay().map(str::to_owned));
    if let Some(original) = network.original()
    {
        result.set_original(Some(original.duplicate()?));
    }
    if let Some(dc_network) = network.dc_network()
    {
        result.set_dc_network(Some(dc_network.duplicate()?));
    }

    let mut new_inputs = Vec::with_capacity(input_names.len());
    for name in input_names
    {
        new_inputs.push(result.add_primary_input(NetworkNode::new(name, NodeKind::PrimaryInput))?);
    }

    for (output_name, cover) in output_names.into_iter().zip(output_covers)
    {
        let driver = result.add_internal(output_name, new_inputs.clone(), SopCover::new(cover))?;
        result.add_primary_output(driver)?;
    }

    Ok(NetworkEspressoResult {
        network: result,
        stats,
    })
}

fn duplicate_with_local_cover_cleanup(network: &Network) -> NetworkEspressoResultValue<NetworkEspressoResult>
{
    let mut result = network.duplicate()?;
    let mut stats = NetworkEspressoStats::default();

    for (node_id, node) in network.nodes()
    {
        let Some(cover) = &node.cover else
        {
            continue;
        };

        stats.original_cubes += cover.cubes().len();
        let cleaned = cleanup_cover(cover);
        stats.minimized_cubes += cleaned.cubes().len();
        result.node_mut(node_id)?.cover = Some(cleaned);
    }

    Ok(NetworkEspressoResult {
        network: result,
        stats,
    })
}

fn collect_node_names(network: &Network, nodes: &[NodeId]) -> NetworkEspressoResultValue<Vec<String>>
{
    nodes
        .iter()
        .map(|node| Ok(network.node(*node)?.name.clone()))
        .collect()
}

fn output_driver(network: &Network, output: NodeId) -> NetworkEspressoResultValue<NodeId>
{
    let node = network.node(output)?;
    node.fanins
        .first()
        .copied()
        .ok_or(NetworkEspressoError::MissingOutputDriver(output))
}

fn enumerate_assignments(input_count: usize) -> Vec<Vec<bool>>
{
    let total = 1_usize << input_count;
    (0..total)
        .map(|value| {
            (0..input_count)
                .map(|input| (value & (1_usize << input)) != 0)
                .collect()
        })
        .collect()
}

fn assignment_values(input_ids: &[NodeId], assignment: &[bool]) -> BTreeMap<NodeId, bool>
{
    input_ids
        .iter()
        .copied()
        .zip(assignment.iter().copied())
        .collect()
}

fn align_dc_assignments(
    network: &Network,
    dc_network: &Network,
    input_ids: &[NodeId],
    assignments: &[Vec<bool>],
) -> NetworkEspressoResultValue<Vec<BTreeMap<NodeId, bool>>>
{
    let mut dc_inputs = Vec::with_capacity(input_ids.len());
    for input in input_ids
    {
        let name = network.node(*input)?.name.clone();
        let dc_input = dc_network
            .find_node(&name)
            .ok_or(NetworkEspressoError::MissingInputName(name))?;
        dc_inputs.push(dc_input);
    }

    Ok(assignments
        .iter()
        .map(|assignment| assignment_values(&dc_inputs, assignment))
        .collect())
}

fn eval_node(
    network: &Network,
    node: NodeId,
    values: &BTreeMap<NodeId, bool>,
    memo: &mut BTreeMap<NodeId, bool>,
    active: &mut BTreeSet<NodeId>,
) -> NetworkEspressoResultValue<bool>
{
    if let Some(value) = memo.get(&node)
    {
        return Ok(*value);
    }

    if !active.insert(node)
    {
        return Err(NetworkUtilError::CycleDetected.into());
    }

    let network_node = network.node(node)?;
    let value = match network_node.kind
    {
        NodeKind::PrimaryInput => values.get(&node).copied().unwrap_or(false),
        NodeKind::PrimaryOutput => {
            let driver = output_driver(network, node)?;
            eval_node(network, driver, values, memo, active)?
        }
        NodeKind::Internal | NodeKind::Unassigned => {
            if let Some(expression) = &network_node.expression
            {
                eval_expression(network, expression, values, memo, active)?
            }
            else if let Some(cover) = &network_node.cover
            {
                eval_cover(network, node, network_node.fanins.as_slice(), cover, values, memo, active)?
            }
            else
            {
                false
            }
        }
    };

    active.remove(&node);
    memo.insert(node, value);
    Ok(value)
}

fn eval_expression(
    network: &Network,
    expression: &BoolExpr,
    values: &BTreeMap<NodeId, bool>,
    memo: &mut BTreeMap<NodeId, bool>,
    active: &mut BTreeSet<NodeId>,
) -> NetworkEspressoResultValue<bool>
{
    match expression
    {
        BoolExpr::Constant(value) => Ok(*value),
        BoolExpr::Literal { node, phase } => {
            let value = eval_node(network, *node, values, memo, active)?;
            Ok(if *phase { value } else { !value })
        }
        BoolExpr::Not(inner) => Ok(!eval_expression(network, inner, values, memo, active)?),
        BoolExpr::And(items) => {
            for item in items
            {
                if !eval_expression(network, item, values, memo, active)?
                {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        BoolExpr::Or(items) => {
            for item in items
            {
                if eval_expression(network, item, values, memo, active)?
                {
                    return Ok(true);
                }
            }
            Ok(false)
        }
    }
}

fn eval_cover(
    network: &Network,
    node: NodeId,
    fanins: &[NodeId],
    cover: &SopCover,
    values: &BTreeMap<NodeId, bool>,
    memo: &mut BTreeMap<NodeId, bool>,
    active: &mut BTreeSet<NodeId>,
) -> NetworkEspressoResultValue<bool>
{
    for (cube_index, cube) in cover.cubes().iter().enumerate()
    {
        if cube.values().len() != fanins.len()
        {
            return Err(NetworkUtilError::InvalidCover {
                node,
                cube: cube_index,
            }
            .into());
        }

        let mut matched = true;
        for (fanin, expected) in fanins.iter().copied().zip(cube.values())
        {
            let actual = eval_node(network, fanin, values, memo, active)?;
            matched = match expected
            {
                CoverValue::Zero => !actual,
                CoverValue::One => actual,
                CoverValue::DontCare => true,
            };
            if !matched
            {
                break;
            }
        }

        if matched
        {
            return Ok(true);
        }
    }

    Ok(false)
}

fn cleanup_cover(cover: &SopCover) -> SopCover
{
    let mut cubes = cover.cubes().to_vec();
    sort_and_dedup_cubes(&mut cubes);
    remove_subsumed_cubes(&mut cubes);
    SopCover::new(cubes)
}

fn minimize_truth_cover(
    input_count: usize,
    on_set: &BTreeSet<usize>,
    dc_set: &BTreeSet<usize>,
) -> Vec<Cube>
{
    if on_set.is_empty()
    {
        return Vec::new();
    }

    let allowed = on_set.union(dc_set).copied().collect::<BTreeSet<_>>();
    let mut current = allowed
        .iter()
        .copied()
        .map(|minterm| Implicant::from_minterm(input_count, minterm))
        .collect::<Vec<_>>();
    let mut primes = Vec::new();

    loop
    {
        let mut used = vec![false; current.len()];
        let mut next = Vec::new();

        for left_index in 0..current.len()
        {
            for right_index in (left_index + 1)..current.len()
            {
                let Some(combined) = current[left_index].combine(&current[right_index]) else
                {
                    continue;
                };

                if !cube_only_covers_allowed(&combined.values, input_count, &allowed)
                {
                    continue;
                }

                used[left_index] = true;
                used[right_index] = true;
                next.push(combined);
            }
        }

        for (index, implicant) in current.into_iter().enumerate()
        {
            if !used[index] && implicant.covers_any(on_set)
            {
                primes.push(implicant);
            }
        }

        if next.is_empty()
        {
            break;
        }

        sort_and_dedup_implicants(&mut next);
        current = next;
    }

    select_cover(input_count, on_set, primes)
        .into_iter()
        .map(|implicant| Cube::new(implicant.values))
        .collect()
}

fn select_cover(input_count: usize, on_set: &BTreeSet<usize>, primes: Vec<Implicant>) -> Vec<Implicant>
{
    let mut remaining = on_set.clone();
    let mut selected = Vec::new();
    let mut candidates = primes
        .into_iter()
        .map(|implicant| {
            let covered = covered_on_set(&implicant.values, input_count, on_set);
            (implicant, covered)
        })
        .filter(|(_, covered)| !covered.is_empty())
        .collect::<Vec<_>>();

    loop
    {
        let mut essential_indexes = BTreeSet::new();
        for minterm in &remaining
        {
            let covering = candidates
                .iter()
                .enumerate()
                .filter(|(_, (_, covered))| covered.contains(minterm))
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if covering.len() == 1
            {
                essential_indexes.insert(covering[0]);
            }
        }

        if essential_indexes.is_empty()
        {
            break;
        }

        for index in essential_indexes.iter().rev()
        {
            let (implicant, covered) = candidates.remove(*index);
            for minterm in covered
            {
                remaining.remove(&minterm);
            }
            selected.push(implicant);
        }

        if remaining.is_empty()
        {
            return selected;
        }
    }

    while !remaining.is_empty()
    {
        let Some((best_index, _)) = candidates
            .iter()
            .enumerate()
            .max_by_key(|(_, (implicant, covered))| {
                let newly_covered = covered.intersection(&remaining).count();
                (newly_covered, implicant.dont_care_count(), usize::MAX - implicant.literal_count())
            })
        else
        {
            break;
        };

        let (implicant, covered) = candidates.remove(best_index);
        for minterm in covered
        {
            remaining.remove(&minterm);
        }
        selected.push(implicant);
    }

    selected
}

fn covered_on_set(
    values: &[CoverValue],
    input_count: usize,
    on_set: &BTreeSet<usize>,
) -> BTreeSet<usize>
{
    (0..(1_usize << input_count))
        .filter(|minterm| on_set.contains(minterm) && cube_covers_minterm(values, *minterm))
        .collect()
}

fn cube_only_covers_allowed(values: &[CoverValue], input_count: usize, allowed: &BTreeSet<usize>) -> bool
{
    (0..(1_usize << input_count))
        .all(|minterm| !cube_covers_minterm(values, minterm) || allowed.contains(&minterm))
}

fn cube_covers_minterm(values: &[CoverValue], minterm: usize) -> bool
{
    values.iter().enumerate().all(|(index, value)| {
        let bit = (minterm & (1_usize << index)) != 0;
        match value
        {
            CoverValue::Zero => !bit,
            CoverValue::One => bit,
            CoverValue::DontCare => true,
        }
    })
}

fn sort_and_dedup_cubes(cubes: &mut Vec<Cube>)
{
    cubes.sort_by_key(|cube| cover_sort_key(cube.values()));
    cubes.dedup_by(|left, right| left.values() == right.values());
}

fn remove_subsumed_cubes(cubes: &mut Vec<Cube>)
{
    let mut keep = vec![true; cubes.len()];
    for left in 0..cubes.len()
    {
        for right in 0..cubes.len()
        {
            if left != right && keep[right] && cube_subsumes(&cubes[left], &cubes[right])
            {
                keep[right] = false;
            }
        }
    }

    let mut index = 0;
    cubes.retain(|_| {
        let keep_cube = keep[index];
        index += 1;
        keep_cube
    });
}

fn cube_subsumes(left: &Cube, right: &Cube) -> bool
{
    left.values().iter().zip(right.values()).all(|(left, right)| {
        *left == CoverValue::DontCare || left == right
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Implicant
{
    values: Vec<CoverValue>,
}

impl Implicant
{
    fn from_minterm(input_count: usize, minterm: usize) -> Self
    {
        let values = (0..input_count)
            .map(|index| {
                if (minterm & (1_usize << index)) == 0
                {
                    CoverValue::Zero
                }
                else
                {
                    CoverValue::One
                }
            })
            .collect();
        Self { values }
    }

    fn combine(&self, other: &Self) -> Option<Self>
    {
        let mut differences = 0;
        let mut values = Vec::with_capacity(self.values.len());

        for (left, right) in self.values.iter().zip(&other.values)
        {
            if left == right
            {
                values.push(*left);
            }
            else if *left != CoverValue::DontCare && *right != CoverValue::DontCare
            {
                differences += 1;
                values.push(CoverValue::DontCare);
            }
            else
            {
                return None;
            }
        }

        if differences == 1
        {
            Some(Self { values })
        }
        else
        {
            None
        }
    }

    fn covers_any(&self, minterms: &BTreeSet<usize>) -> bool
    {
        minterms
            .iter()
            .any(|minterm| cube_covers_minterm(&self.values, *minterm))
    }

    fn dont_care_count(&self) -> usize
    {
        self.values
            .iter()
            .filter(|value| **value == CoverValue::DontCare)
            .count()
    }

    fn literal_count(&self) -> usize
    {
        self.values.len() - self.dont_care_count()
    }
}

fn sort_and_dedup_implicants(implicants: &mut Vec<Implicant>)
{
    implicants.sort_by_key(|implicant| cover_sort_key(&implicant.values));
    implicants.dedup_by(|left, right| left.values == right.values);
}

fn cover_sort_key(values: &[CoverValue]) -> Vec<u8>
{
    values
        .iter()
        .map(|value| match value
        {
            CoverValue::Zero => 0,
            CoverValue::One => 1,
            CoverValue::DontCare => 2,
        })
        .collect()
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn cube(values: &[CoverValue]) -> Cube
    {
        Cube::new(values.to_vec())
    }

    fn build_xor_network() -> Network
    {
        let mut network = Network::new();
        network.set_name("xor");
        let a = network
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let b = network
            .add_primary_input(NetworkNode::new("b", NodeKind::PrimaryInput))
            .unwrap();
        let y = network
            .add_internal(
                "y",
                [a, b],
                SopCover::new([
                    cube(&[CoverValue::Zero, CoverValue::One]),
                    cube(&[CoverValue::One, CoverValue::Zero]),
                ]),
            )
            .unwrap();
        network.add_primary_output(y).unwrap();
        network
    }

    #[test]
    fn minimizes_adjacent_minterms_to_one_cube()
    {
        let on_set = BTreeSet::from([1_usize, 3_usize]);
        let minimized = minimize_truth_cover(2, &on_set, &BTreeSet::new());

        assert_eq!(minimized, vec![cube(&[CoverValue::One, CoverValue::DontCare])]);
    }

    #[test]
    fn uses_dont_care_set_to_expand_on_set()
    {
        let on_set = BTreeSet::from([1_usize]);
        let dc_set = BTreeSet::from([3_usize]);
        let minimized = minimize_truth_cover(2, &on_set, &dc_set);

        assert_eq!(minimized, vec![cube(&[CoverValue::One, CoverValue::DontCare])]);
    }

    #[test]
    fn preserves_xor_when_no_safe_expansion_exists()
    {
        let on_set = BTreeSet::from([1_usize, 2_usize]);
        let minimized = minimize_truth_cover(2, &on_set, &BTreeSet::new());

        assert_eq!(
            minimized,
            vec![
                cube(&[CoverValue::Zero, CoverValue::One]),
                cube(&[CoverValue::One, CoverValue::Zero]),
            ]
        );
    }

    #[test]
    fn network_espresso_rebuilds_two_level_network()
    {
        let network = build_xor_network();
        let result = network_espresso_with_options(&network, &NetworkEspressoOptions::default()).unwrap();

        assert_eq!(result.network.name(), "xor");
        assert_eq!(result.network.num_pi(), 2);
        assert_eq!(result.network.num_po(), 1);
        assert_eq!(result.stats.outputs_processed, 1);
        assert!(result.stats.exact_minimization_used);

        let output = result.network.get_po(0).unwrap();
        let driver = result.network.node(output).unwrap().fanins[0];
        assert_eq!(
            result.network.node(driver).unwrap().cover.as_ref().unwrap().cubes(),
            &[
                cube(&[CoverValue::Zero, CoverValue::One]),
                cube(&[CoverValue::One, CoverValue::Zero]),
            ]
        );
    }

    #[test]
    fn external_dc_network_can_reduce_cover()
    {
        let mut care = Network::new();
        let a = care
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let b = care
            .add_primary_input(NetworkNode::new("b", NodeKind::PrimaryInput))
            .unwrap();
        let y = care
            .add_internal("y", [a, b], SopCover::new([cube(&[CoverValue::One, CoverValue::Zero])]))
            .unwrap();
        let output = care.add_primary_output(y).unwrap();
        let output_name = care.node(output).unwrap().name.clone();

        let mut dc = Network::new();
        let dc_a = dc
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let dc_b = dc
            .add_primary_input(NetworkNode::new("b", NodeKind::PrimaryInput))
            .unwrap();
        let dc_y = dc
            .add_internal(
                "dc_y",
                [dc_a, dc_b],
                SopCover::new([cube(&[CoverValue::One, CoverValue::One])]),
            )
            .unwrap();
        let dc_output = dc.add_primary_output(dc_y).unwrap();
        dc.change_node_name(dc_output, output_name).unwrap();
        care.set_dc_network(Some(dc));

        let result = network_espresso(&care).unwrap();
        let output = result.get_po(0).unwrap();
        let driver = result.node(output).unwrap().fanins[0];
        let cover = result.node(driver).unwrap().cover.as_ref().unwrap().cubes().to_vec();

        assert_eq!(cover, vec![cube(&[CoverValue::One, CoverValue::DontCare])]);
    }

    #[test]
    fn large_network_path_cleans_duplicate_local_cubes()
    {
        let mut network = Network::new();
        let a = network
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let y = network
            .add_internal(
                "y",
                [a],
                SopCover::new([
                    cube(&[CoverValue::One]),
                    cube(&[CoverValue::One]),
                    cube(&[CoverValue::DontCare]),
                ]),
            )
            .unwrap();
        network.add_primary_output(y).unwrap();

        let result = network_espresso_with_options(
            &network,
            &NetworkEspressoOptions::default().with_exact_input_limit(0),
        )
        .unwrap();
        let output = result.network.get_po(0).unwrap();
        let driver = result.network.node(output).unwrap().fanins[0];

        assert_eq!(
            result.network.node(driver).unwrap().cover.as_ref().unwrap().cubes(),
            &[cube(&[CoverValue::DontCare])]
        );
        assert!(!result.stats.exact_minimization_used);
    }
}
