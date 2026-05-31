//! Native bounded-wire-delay slow-down support.
//!
//! The legacy unit updates PI-to-PO minimum delay tables and inserts inverter
//! pairs on signals whose bounded-wire-delay hazard check fails. This Rust port
//! keeps that algorithm over an owned delay graph. A later integration layer can
//! translate real network/gate edits into the `SlowDownAction` plan produced
//! here.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BwdNodeKind
{
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BwdNode
{
    pub name: String,
    pub kind: BwdNodeKind,
    pub fanins: Vec<NodeId>,
    pub is_real_primary_output: bool,
}

impl BwdNode
{
    pub fn new(name: impl Into<String>, kind: BwdNodeKind) -> Self
    {
        Self
        {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            is_real_primary_output: false,
        }
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = NodeId>) -> Self
    {
        self.fanins = fanins.into_iter().collect();
        self
    }

    pub fn real_primary_output(mut self) -> Self
    {
        self.is_real_primary_output = true;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime
{
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime
{
    pub fn new(rise: f64, fall: f64) -> Self
    {
        Self
        {
            rise,
            fall,
        }
    }

    pub fn both(value: f64) -> Self
    {
        Self
        {
            rise: value,
            fall: value,
        }
    }

    fn min_edge(self) -> f64
    {
        self.rise.min(self.fall)
    }

    fn max_edge(self) -> f64
    {
        self.rise.max(self.fall)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinDelay
{
    pub rise: DelayTime,
    pub fall: DelayTime,
}

impl MinDelay
{
    pub fn from_delay(delay: DelayTime) -> Self
    {
        Self
        {
            rise: delay,
            fall: delay,
        }
    }

    fn minimum_component(self) -> f64
    {
        self.rise
            .rise
            .min(self.rise.fall)
            .min(self.fall.rise)
            .min(self.fall.fall)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayArc
{
    pub from: NodeId,
    pub to: NodeId,
    pub min_delay: DelayTime,
    pub max_delay: DelayTime,
}

impl DelayArc
{
    pub fn new(from: NodeId, to: NodeId, min_delay: DelayTime, max_delay: DelayTime) -> Self
    {
        Self
        {
            from,
            to,
            min_delay,
            max_delay,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BwdDelayNetwork
{
    nodes: Vec<BwdNode>,
    arcs: Vec<DelayArc>,
}

impl BwdDelayNetwork
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn add_node(&mut self, node: BwdNode) -> NodeId
    {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn add_arc(&mut self, arc: DelayArc)
    {
        self.arcs.push(arc);
    }

    pub fn node(&self, id: NodeId) -> Result<&BwdNode, BwdSlowError>
    {
        self.nodes.get(id.0).ok_or(BwdSlowError::MissingNode { id })
    }

    pub fn find_node(&self, name: &str) -> Option<NodeId>
    {
        self.nodes
            .iter()
            .position(|node| node.name == name)
            .map(NodeId)
    }

    pub fn primary_inputs(&self) -> Vec<NodeId>
    {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)|
            {
                if node.kind == BwdNodeKind::PrimaryInput
                {
                    Some(NodeId(index))
                }
                else
                {
                    None
                }
            })
            .collect()
    }

    pub fn primary_outputs(&self) -> Vec<NodeId>
    {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)|
            {
                if node.kind == BwdNodeKind::PrimaryOutput
                {
                    Some(NodeId(index))
                }
                else
                {
                    None
                }
            })
            .collect()
    }

    fn skips_direct_real_primary_output(&self, po: NodeId) -> bool
    {
        let Some(node) = self.nodes.get(po.0) else
        {
            return false;
        };

        if !node.is_real_primary_output || node.fanins.len() != 1
        {
            return false;
        }

        self.nodes
            .get(node.fanins[0].0)
            .is_some_and(|fanin| fanin.kind == BwdNodeKind::PrimaryInput)
    }
}

pub type ExternalDelayTable = HashMap<String, HashMap<String, MinDelay>>;
pub type SlowedAmounts = HashMap<String, f64>;
pub type HazardTable = HashMap<String, Vec<Hazard>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayMode
{
    Min,
    Max,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hazard
{
    pub s1: String,
    pub s2: String,
    pub dir1: char,
    pub dir2: char,
}

impl Hazard
{
    pub fn new(s1: impl Into<String>, s2: impl Into<String>, dir1: char, dir2: char) -> Self
    {
        Self
        {
            s1: s1.into(),
            s2: s2.into(),
            dir1,
            dir2,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SlowDownOptions
{
    pub tolerance: f64,
    pub default_delay: f64,
    pub min_delay_factor: f64,
    pub use_shortest_path: bool,
    pub iterate: bool,
    pub do_slow: bool,
    pub inverter_pair_min_delay: f64,
}

impl Default for SlowDownOptions
{
    fn default() -> Self
    {
        Self
        {
            tolerance: 0.0,
            default_delay: 0.0,
            min_delay_factor: 1.0,
            use_shortest_path: false,
            iterate: true,
            do_slow: true,
            inverter_pair_min_delay: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SlowDownAction
{
    pub slowed_signal: String,
    pub hazard_from: String,
    pub hazard_to: String,
    pub output: String,
    pub excess: f64,
    pub inserted_delay: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SlowDownReport
{
    pub actions: Vec<SlowDownAction>,
    pub external_delay_updated: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct D3Table
{
    names: Vec<String>,
    indexes: HashMap<String, usize>,
    delays: Vec<Vec<f64>>,
}

impl D3Table
{
    pub fn names(&self) -> &[String]
    {
        &self.names
    }

    pub fn get(&self, from: &str, to: &str) -> Option<f64>
    {
        let from = *self.indexes.get(from)?;
        let to = *self.indexes.get(to)?;
        Some(self.delays[from][to])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BwdSlowError
{
    MissingNode { id: NodeId },
    MissingNodeName { name: String },
    MissingHazards { output: String },
    NonPositiveInverterDelay,
}

impl fmt::Display for BwdSlowError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::MissingNode { id } =>
            {
                write!(formatter, "missing bounded-wire-delay node {}", id.0)
            }
            Self::MissingNodeName { name } =>
            {
                write!(formatter, "missing bounded-wire-delay node named {name}")
            }
            Self::MissingHazards { output } =>
            {
                write!(formatter, "missing hazard list for output {output}")
            }
            Self::NonPositiveInverterDelay =>
            {
                write!(formatter, "inverter pair delay must be positive")
            }
        }
    }
}

impl Error for BwdSlowError
{
}

pub fn bwd_po_name(name: &str) -> String
{
    name.trim_end_matches(['+', '-']).to_string()
}

pub fn delay_simulate(
    network: &BwdDelayNetwork,
    from: NodeId,
    to: NodeId,
    mode: DelayMode,
    min_delay_factor: f64,
) -> Result<DelayTime, BwdSlowError>
{
    network.node(from)?;
    network.node(to)?;

    let node_count = network.nodes.len();
    let mut rise = vec![initial_arrival(mode); node_count];
    let mut fall = vec![initial_arrival(mode); node_count];

    rise[from.0] = 0.0;
    fall[from.0] = 0.0;

    for _ in 0..node_count.saturating_sub(1)
    {
        let mut changed = false;

        for arc in &network.arcs
        {
            let edge = match mode
            {
                DelayMode::Min => arc.min_delay,
                DelayMode::Max => arc.max_delay,
            };
            changed |= relax_arrival(&mut rise, arc.from.0, arc.to.0, edge.rise, mode);
            changed |= relax_arrival(&mut fall, arc.from.0, arc.to.0, edge.fall, mode);
        }

        if !changed
        {
            break;
        }
    }

    let mut delay = DelayTime::new(rise[to.0], fall[to.0]);
    if mode == DelayMode::Min
    {
        delay.rise *= min_delay_factor;
        delay.fall *= min_delay_factor;
    }

    Ok(delay)
}

pub fn update_external_delay_table(
    network: &BwdDelayNetwork,
    external_delays: &mut ExternalDelayTable,
    min_delay_factor: f64,
) -> Result<bool, BwdSlowError>
{
    let mut updated = false;

    for pi in network.primary_inputs()
    {
        for po in network.primary_outputs()
        {
            if network.skips_direct_real_primary_output(po)
            {
                continue;
            }

            let delay = delay_simulate(network, pi, po, DelayMode::Min, min_delay_factor)?;
            if is_unbounded(delay.rise) && is_unbounded(delay.fall)
            {
                continue;
            }

            let from_name = bwd_po_name(&network.node(pi)?.name);
            let to_name = bwd_po_name(&network.node(po)?.name);
            let new_delay = MinDelay::from_delay(delay);

            if external_delay_is_at_most(external_delays, &from_name, &to_name, new_delay)
            {
                continue;
            }

            external_delays
                .entry(from_name)
                .or_default()
                .insert(to_name, new_delay);
            updated = true;
        }
    }

    Ok(updated)
}

pub fn fill_d3s(
    network: &BwdDelayNetwork,
    external_delays: &ExternalDelayTable,
    use_shortest_path: bool,
    default_delay: f64,
    min_delay_factor: f64,
) -> Result<D3Table, BwdSlowError>
{
    let mut names = Vec::<String>::new();
    let mut indexes = HashMap::<String, usize>::new();

    for node in network
        .primary_inputs()
        .into_iter()
        .chain(network.primary_outputs())
    {
        add_d3_name(
            &mut names,
            &mut indexes,
            bwd_po_name(&network.node(node)?.name),
        );
    }

    for (from_name, to_table) in external_delays
    {
        add_d3_name(&mut names, &mut indexes, from_name.clone());
        for to_name in to_table.keys()
        {
            add_d3_name(&mut names, &mut indexes, to_name.clone());
        }
    }

    let mut delays = vec![vec![f64::INFINITY; names.len()]; names.len()];

    for pi in network.primary_inputs()
    {
        let from = indexes[&bwd_po_name(&network.node(pi)?.name)];

        for po in network.primary_outputs()
        {
            if network.skips_direct_real_primary_output(po)
            {
                continue;
            }

            let to = indexes[&bwd_po_name(&network.node(po)?.name)];
            let delay = delay_simulate(network, pi, po, DelayMode::Min, min_delay_factor)?;
            let d3 = delay.min_edge();
            delays[from][to] = if d3 < 0.0
            {
                f64::INFINITY
            }
            else
            {
                d3
            };
        }
    }

    for (from_name, to_table) in external_delays
    {
        let from = indexes[from_name];
        for (to_name, delay) in to_table
        {
            let to = indexes[to_name];
            let new_delay = delay.minimum_component();
            if delays[from][to] > new_delay
            {
                delays[from][to] = new_delay;
            }
        }
    }

    if use_shortest_path
    {
        for iter in 0..names.len()
        {
            for from in 0..names.len()
            {
                for to in 0..names.len()
                {
                    let candidate = delays[from][iter] + delays[iter][to];
                    if delays[from][to] > candidate
                    {
                        delays[from][to] = candidate;
                    }
                }
            }
        }
    }

    for row in &mut delays
    {
        for delay in row
        {
            if is_unbounded(*delay)
            {
                *delay = default_delay;
            }
        }
    }

    Ok(D3Table
    {
        names,
        indexes,
        delays,
    })
}

pub fn slow_down(
    network: &BwdDelayNetwork,
    hazard_list: &HazardTable,
    slowed_amounts: &mut SlowedAmounts,
    external_delays: &mut ExternalDelayTable,
    options: &SlowDownOptions,
) -> Result<SlowDownReport, BwdSlowError>
{
    if options.do_slow && options.inverter_pair_min_delay <= 0.0
    {
        return Err(BwdSlowError::NonPositiveInverterDelay);
    }

    let d3s = fill_d3s(
        network,
        external_delays,
        options.use_shortest_path,
        options.default_delay,
        options.min_delay_factor,
    )?;
    let mut report = SlowDownReport::default();

    if options.do_slow
    {
        for po in network.primary_outputs()
        {
            if network.skips_direct_real_primary_output(po)
            {
                continue;
            }

            let output_name = bwd_po_name(&network.node(po)?.name);
            let hazards =
                hazard_list
                    .get(&output_name)
                    .ok_or_else(|| BwdSlowError::MissingHazards
                    {
                        output: output_name.clone(),
                    })?;

            loop
            {
                let mut worst: Option<WorstHazard> = None;

                for hazard in hazards
                {
                    let pi1 = network.find_node(&hazard.s1).ok_or_else(||
                    {
                        BwdSlowError::MissingNodeName
                        {
                            name: hazard.s1.clone(),
                        }
                    })?;
                    let pi2 = network.find_node(&hazard.s2).ok_or_else(||
                    {
                        BwdSlowError::MissingNodeName
                        {
                            name: hazard.s2.clone(),
                        }
                    })?;

                    let pi1_name = bwd_po_name(&network.node(pi1)?.name);
                    let pi2_name = bwd_po_name(&network.node(pi2)?.name);
                    let d1 = adjusted_min_delay(
                        network,
                        pi1,
                        po,
                        slowed_amounts.get(&pi1_name).copied().unwrap_or(0.0),
                        options.min_delay_factor,
                    )?;
                    let d2 = adjusted_max_delay(
                        network,
                        pi2,
                        po,
                        slowed_amounts.get(&pi2_name).copied().unwrap_or(0.0),
                        options.min_delay_factor,
                    )?;
                    let d3 = (d3s
                        .get(&pi2_name, &pi1_name)
                        .unwrap_or(options.default_delay)
                        + slowed_amounts.get(&pi1_name).copied().unwrap_or(0.0))
                    .max(0.0);
                    let diff = d2 - (d1 + d3);

                    if worst.as_ref().is_none_or(|current| diff > current.diff)
                    {
                        worst = Some(WorstHazard
                        {
                            pi1_name,
                            hazard_to: hazard.s1.clone(),
                            hazard_from: hazard.s2.clone(),
                            dir1: hazard.dir1,
                            dir2: hazard.dir2,
                            diff,
                        });
                    }
                }

                let Some(worst) = worst else
                {
                    break;
                };
                let excess = worst.diff + options.tolerance;
                if excess <= 0.0
                {
                    break;
                }

                let mut inserted_this_pass = 0.0;
                loop
                {
                    let inserted = options.inverter_pair_min_delay;
                    *slowed_amounts.entry(worst.pi1_name.clone()).or_insert(0.0) += inserted;
                    inserted_this_pass += inserted;
                    report.actions.push(SlowDownAction
                    {
                        slowed_signal: worst.pi1_name.clone(),
                        hazard_from: format!("{}{}", worst.hazard_from, worst.dir2),
                        hazard_to: format!("{}{}", worst.hazard_to, worst.dir1),
                        output: output_name.clone(),
                        excess,
                        inserted_delay: inserted,
                    });

                    if options.iterate || inserted_this_pass >= excess
                    {
                        break;
                    }
                }
            }
        }
    }

    report.external_delay_updated =
        update_external_delay_table(network, external_delays, options.min_delay_factor)?;

    Ok(report)
}

#[derive(Clone, Debug)]
struct WorstHazard
{
    pi1_name: String,
    hazard_to: String,
    hazard_from: String,
    dir1: char,
    dir2: char,
    diff: f64,
}

fn initial_arrival(mode: DelayMode) -> f64
{
    match mode
    {
        DelayMode::Min => f64::INFINITY,
        DelayMode::Max => f64::NEG_INFINITY,
    }
}

fn relax_arrival(
    values: &mut [f64],
    from: usize,
    to: usize,
    edge_delay: f64,
    mode: DelayMode,
) -> bool
{
    let candidate = values[from] + edge_delay;
    match mode
    {
        DelayMode::Min if candidate < values[to] =>
        {
            values[to] = candidate;
            true
        }
        DelayMode::Max if candidate > values[to] =>
        {
            values[to] = candidate;
            true
        }
        _ => false,
    }
}

fn external_delay_is_at_most(
    external_delays: &ExternalDelayTable,
    from_name: &str,
    to_name: &str,
    new_delay: MinDelay,
) -> bool
{
    const EPS: f64 = 1.0e-24;

    external_delays
        .get(from_name)
        .and_then(|to_table| to_table.get(to_name))
        .is_some_and(|old_delay|
        {
            old_delay.rise.rise <= new_delay.rise.rise + EPS
                && old_delay.rise.fall <= new_delay.rise.fall + EPS
                && old_delay.fall.rise <= new_delay.fall.rise + EPS
                && old_delay.fall.fall <= new_delay.fall.fall + EPS
        })
}

fn add_d3_name(names: &mut Vec<String>, indexes: &mut HashMap<String, usize>, name: String)
{
    if indexes.contains_key(&name)
    {
        return;
    }

    indexes.insert(name.clone(), names.len());
    names.push(name);
}

fn is_unbounded(delay: f64) -> bool
{
    delay.is_infinite() || delay > f64::MAX / 100.0
}

fn adjusted_min_delay(
    network: &BwdDelayNetwork,
    from: NodeId,
    to: NodeId,
    slowed: f64,
    min_delay_factor: f64,
) -> Result<f64, BwdSlowError>
{
    Ok(
        (delay_simulate(network, from, to, DelayMode::Min, min_delay_factor)?.min_edge() - slowed)
            .max(0.0),
    )
}

fn adjusted_max_delay(
    network: &BwdDelayNetwork,
    from: NodeId,
    to: NodeId,
    slowed: f64,
    min_delay_factor: f64,
) -> Result<f64, BwdSlowError>
{
    Ok(
        (delay_simulate(network, from, to, DelayMode::Max, min_delay_factor)?.max_edge() - slowed)
            .max(0.0),
    )
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn sample_network() -> BwdDelayNetwork
    {
        let mut network = BwdDelayNetwork::new();
        let a = network.add_node(BwdNode::new("a+", BwdNodeKind::PrimaryInput));
        let b = network.add_node(BwdNode::new("b", BwdNodeKind::PrimaryInput));
        let y = network.add_node(
            BwdNode::new("y-", BwdNodeKind::PrimaryOutput)
                .with_fanins([a])
                .real_primary_output(),
        );
        let z = network.add_node(BwdNode::new("z", BwdNodeKind::PrimaryOutput));

        network.add_arc(DelayArc::new(
            a,
            y,
            DelayTime::new(1.0, 1.5),
            DelayTime::new(4.0, 5.0),
        ));
        network.add_arc(DelayArc::new(
            a,
            z,
            DelayTime::new(2.0, 3.0),
            DelayTime::new(6.0, 8.0),
        ));
        network.add_arc(DelayArc::new(
            b,
            z,
            DelayTime::new(1.0, 2.0),
            DelayTime::new(10.0, 9.0),
        ));

        network
    }

    #[test]
    fn canonical_output_name_strips_transition_suffix()
    {
        assert_eq!(bwd_po_name("ack+"), "ack");
        assert_eq!(bwd_po_name("req-"), "req");
        assert_eq!(bwd_po_name("done"), "done");
    }

    #[test]
    fn delay_simulate_returns_min_or_max_arrival()
    {
        let network = sample_network();
        let a = network.find_node("a+").unwrap();
        let z = network.find_node("z").unwrap();

        assert_eq!(
            delay_simulate(&network, a, z, DelayMode::Min, 2.0).unwrap(),
            DelayTime::new(4.0, 6.0)
        );
        assert_eq!(
            delay_simulate(&network, a, z, DelayMode::Max, 1.0).unwrap(),
            DelayTime::new(6.0, 8.0)
        );
    }

    #[test]
    fn external_delay_update_skips_direct_real_primary_outputs_keeps_smaller_old_delay_and_adds_missing_pairs(
    )
    {
        let network = sample_network();
        let mut external = ExternalDelayTable::new();
        external
            .entry("a".to_string())
            .or_default()
            .insert("z".to_string(), MinDelay::from_delay(DelayTime::both(1.0)));

        let updated = update_external_delay_table(&network, &mut external, 1.0).unwrap();

        assert!(updated);
        assert!(!external.get("a").unwrap().contains_key("y"));
        assert_eq!(
            external.get("a").unwrap().get("z").copied(),
            Some(MinDelay::from_delay(DelayTime::both(1.0)))
        );
        assert_eq!(
            external.get("b").unwrap().get("z").copied(),
            Some(MinDelay::from_delay(DelayTime::new(1.0, 2.0)))
        );
    }

    #[test]
    fn external_delay_update_replaces_when_new_minimum_is_smaller()
    {
        let network = sample_network();
        let mut external = ExternalDelayTable::new();
        external
            .entry("a".to_string())
            .or_default()
            .insert("z".to_string(), MinDelay::from_delay(DelayTime::both(4.0)));

        let updated = update_external_delay_table(&network, &mut external, 1.0).unwrap();

        assert!(updated);
        assert_eq!(
            external.get("a").unwrap().get("z").copied(),
            Some(MinDelay::from_delay(DelayTime::new(2.0, 3.0)))
        );
    }

    #[test]
    fn fill_d3s_merges_external_delays_and_applies_shortest_paths_before_default()
    {
        let network = sample_network();
        let mut external = ExternalDelayTable::new();
        external
            .entry("z".to_string())
            .or_default()
            .insert("a".to_string(), MinDelay::from_delay(DelayTime::both(4.0)));
        external
            .entry("a".to_string())
            .or_default()
            .insert("b".to_string(), MinDelay::from_delay(DelayTime::both(2.0)));
        external
            .entry("b".to_string())
            .or_default()
            .insert("z".to_string(), MinDelay::from_delay(DelayTime::both(1.0)));

        let d3s = fill_d3s(&network, &external, true, 99.0, 1.0).unwrap();

        assert_eq!(d3s.get("a", "z"), Some(2.0));
        assert_eq!(d3s.get("z", "b"), Some(6.0));
        assert_eq!(d3s.get("b", "a"), Some(5.0));
    }

    #[test]
    fn slow_down_adds_inverter_pairs_until_hazard_is_covered()
    {
        let network = sample_network();
        let mut hazards = HazardTable::new();
        hazards.insert("z".to_string(), vec![Hazard::new("a+", "b", '+', '-')]);
        let mut slowed = SlowedAmounts::new();
        let mut external = ExternalDelayTable::new();

        let report = slow_down(
            &network,
            &hazards,
            &mut slowed,
            &mut external,
            &SlowDownOptions
            {
                tolerance: 0.0,
                default_delay: 0.0,
                min_delay_factor: 1.0,
                use_shortest_path: false,
                iterate: false,
                do_slow: true,
                inverter_pair_min_delay: 2.0,
            },
        )
        .unwrap();

        assert_eq!(report.actions.len(), 5);
        assert_eq!(slowed.get("a").copied(), Some(10.0));
        assert_eq!(report.actions[0].slowed_signal, "a");
        assert_eq!(report.actions[0].hazard_from, "b-");
        assert_eq!(report.actions[0].hazard_to, "a++");
    }

    #[test]
    fn slow_down_can_only_update_external_delay_table()
    {
        let network = sample_network();
        let hazards = HazardTable::new();
        let mut slowed = SlowedAmounts::new();
        let mut external = ExternalDelayTable::new();

        let report = slow_down(
            &network,
            &hazards,
            &mut slowed,
            &mut external,
            &SlowDownOptions
            {
                do_slow: false,
                ..SlowDownOptions::default()
            },
        )
        .unwrap();

        assert!(report.actions.is_empty());
        assert!(report.external_delay_updated);
        assert!(slowed.is_empty());
        assert!(external.get("a").unwrap().contains_key("z"));
    }

    #[test]
    fn source_contains_no_tracking_tokens_or_legacy_c_abi_exports()
    {
        let source = include_str!("bwd_slow.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday1", "-", "8j8")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
