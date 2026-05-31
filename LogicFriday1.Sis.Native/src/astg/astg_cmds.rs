//! Native model for the basic ASTG command layer.
//!
//! The legacy implementation registered SIS commands and handled command-line
//! parsing before delegating to ASTG algorithms. Most ASTG algorithms are ported
//! in separate units, so this file keeps the command surface, option semantics,
//! current-ASTG slot behavior, and usage diagnostics as native Rust data.

use std::sync::atomic::{AtomicI32, Ordering};

static ASTG_DEBUG_FLAG: AtomicI32 = AtomicI32::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AstgResult
{
    Ok,
    Error,
    BadOption,
}

impl AstgResult
{
    pub fn is_error(self) -> bool
    {
        self != Self::Ok
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgGraph
{
    pub name: String,
    pub filename: String,
    pub comments: Vec<String>,
    pub has_marking: bool,
    pub file_change_count: i64,
    pub change_count: i64,
    pub pure: bool,
    pub place_simple: bool,
    pub connected_components: usize,
    pub strong_components: usize,
    pub free_choice: bool,
    pub marked_graph: bool,
    pub state_machine: bool,
    pub state_machine_components: Option<usize>,
    pub marked_graph_components: Option<usize>,
}

impl AstgGraph
{
    pub fn new(name: impl Into<String>) -> Self
    {
        Self {
            name: name.into(),
            filename: String::new(),
            comments: Vec::new(),
            has_marking: false,
            file_change_count: 0,
            change_count: 0,
            pure: false,
            place_simple: false,
            connected_components: 0,
            strong_components: 0,
            free_choice: false,
            marked_graph: false,
            state_machine: false,
            state_machine_components: None,
            marked_graph_components: None,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SisNetwork
{
    astg: Option<AstgGraph>,
}

impl SisNetwork
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn with_astg(astg: AstgGraph) -> Self
    {
        Self {
            astg: Some(astg),
        }
    }

    pub fn astg(&self) -> Option<&AstgGraph>
    {
        self.astg.as_ref()
    }

    pub fn astg_mut(&mut self) -> Option<&mut AstgGraph>
    {
        self.astg.as_mut()
    }

    pub fn take_astg(&mut self) -> Option<AstgGraph>
    {
        self.astg.take()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AstgSetCurrentReport
{
    pub reset_network: bool,
    pub allocated_network: bool,
    pub installed_astg: bool,
    pub detached_astg: Option<AstgGraph>,
}

pub fn astg_debug_flag() -> i32
{
    ASTG_DEBUG_FLAG.load(Ordering::Relaxed)
}

pub fn set_astg_debug_flag(value: i32)
{
    ASTG_DEBUG_FLAG.store(value, Ordering::Relaxed);
}

pub fn astg_dup(old_astg: Option<&AstgGraph>) -> Option<AstgGraph>
{
    old_astg.cloned()
}

pub fn astg_free(_old_astg: Option<AstgGraph>)
{
}

pub fn astg_current(network: Option<&SisNetwork>) -> Option<&AstgGraph>
{
    network.and_then(SisNetwork::astg)
}

pub fn astg_set_current(
    network: &mut Option<SisNetwork>,
    stg: Option<AstgGraph>,
    reset: bool,
) -> AstgSetCurrentReport
{
    let mut report = AstgSetCurrentReport {
        reset_network: reset && network.is_some(),
        allocated_network: false,
        installed_astg: stg.is_some(),
        detached_astg: None,
    };

    if reset
    {
        *network = None;
    }
    else if let Some(current_network) = network.as_mut()
    {
        report.detached_astg = current_network.astg.take();
    }

    if let Some(stg) = stg
    {
        if network.is_none()
        {
            *network = Some(SisNetwork::new());
            report.allocated_network = true;
        }

        if let Some(current_network) = network.as_mut()
        {
            current_network.astg = Some(stg);
        }
    }

    report
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AstgBasicCommand
{
    Read,
    Flow,
    Current,
    Persist,
    LockGraph,
    Cycle,
    Write,
    Marking,
    Irredundant,
    Contract,
    StateMachineComponents,
    MarkedGraphComponents,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AstgCommandRegistration
{
    pub name: &'static str,
    pub kind: AstgBasicCommand,
    pub changes_network: bool,
}

pub const BASIC_ASTG_COMMANDS: &[AstgCommandRegistration] = &[
    AstgCommandRegistration {
        name: "read_astg",
        kind: AstgBasicCommand::Read,
        changes_network: true,
    },
    AstgCommandRegistration {
        name: "_astg_flow",
        kind: AstgBasicCommand::Flow,
        changes_network: true,
    },
    AstgCommandRegistration {
        name: "astg_current",
        kind: AstgBasicCommand::Current,
        changes_network: false,
    },
    AstgCommandRegistration {
        name: "astg_persist",
        kind: AstgBasicCommand::Persist,
        changes_network: true,
    },
    AstgCommandRegistration {
        name: "astg_lockgraph",
        kind: AstgBasicCommand::LockGraph,
        changes_network: true,
    },
    AstgCommandRegistration {
        name: "_astg_cycle",
        kind: AstgBasicCommand::Cycle,
        changes_network: false,
    },
    AstgCommandRegistration {
        name: "write_astg",
        kind: AstgBasicCommand::Write,
        changes_network: false,
    },
    AstgCommandRegistration {
        name: "astg_marking",
        kind: AstgBasicCommand::Marking,
        changes_network: true,
    },
    AstgCommandRegistration {
        name: "_astg_irred",
        kind: AstgBasicCommand::Irredundant,
        changes_network: true,
    },
    AstgCommandRegistration {
        name: "astg_contract",
        kind: AstgBasicCommand::Contract,
        changes_network: true,
    },
    AstgCommandRegistration {
        name: "_astg_smc",
        kind: AstgBasicCommand::StateMachineComponents,
        changes_network: false,
    },
    AstgCommandRegistration {
        name: "_astg_mgc",
        kind: AstgBasicCommand::MarkedGraphComponents,
        changes_network: false,
    },
];

pub fn astg_basic_command_registrations() -> &'static [AstgCommandRegistration]
{
    BASIC_ASTG_COMMANDS
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgUsage
{
    pub lines: &'static [&'static str],
}

impl AstgUsage
{
    pub fn render(&self, command: &str) -> String
    {
        self.lines
            .iter()
            .map(|line| line.replacen("%s", command, 1))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub const FLOW_USAGE: AstgUsage = AstgUsage {
    lines: &[
        "usage: %s [-x] [-l <latch-type>] [-o <outfile>] [-q]",
        "    -x   bypass one-token SM check before flow",
        "    -l   use specified latch type, default=as",
        "    -o   save BLIF description in <outfile>, use '-' for stdout",
        "    -q   turn off verbose option of flow",
    ],
};

pub const READ_USAGE: AstgUsage = AstgUsage {
    lines: &[
        "usage: %s [<stg_file>]",
        "    Reads from stdin if no file name is specified.",
    ],
};

pub const WRITE_USAGE: AstgUsage = AstgUsage {
    lines: &[
        "usage: %s [-p] [<outfile>]",
        "    -p  print all places even with 1 in edge and out edge",
    ],
};

pub const CYCLE_USAGE: AstgUsage = AstgUsage {
    lines: &[
        "usage: %s [-la] [-t <trans_name>] [<cycle index>]",
        "    select simple cycles in the STG (default = all)",
        "    -l  select longest delay",
        "    -a  append to existing set",
        "    -t  optionally only through specified trans",
        "    -c  count total number of simple cycles",
    ],
};

pub const LOCKGRAPH_USAGE: AstgUsage = AstgUsage {
    lines: &[
        "usage: %s [-l]",
        "    print lock graph for an STG",
        "    -l\ttry to form one lock class first",
    ],
};

pub const PERSIST_USAGE: AstgUsage = AstgUsage {
    lines: &[
        "usage: %s [-p]",
        "    -p    just print nonpersistent transitions (don't modify STG)",
        "    Otherwise, add persistency constraints to the STG.",
    ],
};

pub const IRREDUNDANT_USAGE: AstgUsage = AstgUsage {
    lines: &[
        "usage: %s [-p]",
        "    -p  print redundant edges instead of deleting them.",
    ],
};

pub const CURRENT_USAGE: AstgUsage = AstgUsage {
    lines: &[
        "usage: %s [-d #]",
        "    -d  set debug output (0=no debug output)",
        "    display information about the current stg",
    ],
};

pub const MARKING_USAGE: AstgUsage = AstgUsage {
    lines: &[
        "usage: %s [-s] [new_marking]",
        "    print or set initial marking for the current STG",
        "    -s  specify new marking using signal values",
        "        e.g. a 1 b 0",
        "    otherwise new marking in format: {place1 place2 ...}",
        "    where place is either place name or <t1,t2> transition pair.",
    ],
};

pub const MARKED_GRAPH_COMPONENT_USAGE: AstgUsage = AstgUsage {
    lines: &[
        "usage: %s [<n1> ...]",
        "    find marked graph (MG) components.  If no arguments are given",
        "    then any vertices which are not covered by the MG compoents",
        "    are printed, otherwise the components with the given numbers are",
        "    printed.",
    ],
};

pub const STATE_MACHINE_COMPONENT_USAGE: AstgUsage = AstgUsage {
    lines: &[
        "usage: %s [<n1> ...]",
        "    find state machine (SM) components.  If no arguments are given",
        "    then any vertices which are not covered by the SM compoents",
        "    are printed, otherwise the components with the given numbers are",
        "    printed.",
    ],
};

pub const CONTRACT_USAGE: AstgUsage = AstgUsage {
    lines: &[
        "usage: %s [-f] <output_signal>",
        "    -f  keep contracted nets Free Choice",
        "    generate the contracted net for the specified noninput signal",
    ],
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AstgCommandPlan
{
    Flow(AstgFlowOptions),
    Read(AstgReadOptions),
    Write(AstgWriteOptions),
    Cycle(AstgCycleOptions),
    LockGraph(AstgLockGraphOptions),
    Persist(AstgModifyOptions),
    Irredundant(AstgModifyOptions),
    Current(AstgCurrentOptions),
    Marking(AstgMarkingOptions),
    Components(Vec<usize>),
    Contract(AstgContractOptions),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgFlowOptions
{
    pub full_checks: bool,
    pub latch_type: String,
    pub outfile: Option<String>,
    pub verbose: bool,
}

impl Default for AstgFlowOptions
{
    fn default() -> Self
    {
        Self {
            full_checks: true,
            latch_type: "as".to_owned(),
            outfile: None,
            verbose: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgReadOptions
{
    pub infile: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgWriteOptions
{
    pub hide_places: bool,
    pub outfile: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgCycleOptions
{
    pub through_transition: Option<String>,
    pub longest: bool,
    pub append: bool,
    pub count: bool,
    pub cycle_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgLockGraphOptions
{
    pub form_one_lock_class: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgModifyOptions
{
    pub modify: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgCurrentOptions
{
    pub debug_level: Option<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AstgMarkingOptions
{
    Print {
        by_state_code: bool,
    },
    SetPlaces {
        marking: String,
    },
    SetSignals {
        values: Vec<(String, i32)>,
        ignored_signals: Vec<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgContractOptions
{
    pub keep_free_choice: bool,
    pub output_signal: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgCommandError
{
    pub result: AstgResult,
    pub usage: AstgUsage,
}

impl AstgCommandError
{
    fn bad_option(usage: AstgUsage) -> Self
    {
        Self {
            result: AstgResult::BadOption,
            usage,
        }
    }
}

pub fn plan_basic_command(
    kind: AstgBasicCommand,
    args: &[String],
    has_current_astg: bool,
) -> Result<AstgCommandPlan, AstgCommandError>
{
    match kind
    {
        AstgBasicCommand::Read => plan_read(args),
        AstgBasicCommand::Flow => require_current(has_current_astg, FLOW_USAGE)
            .and_then(|()| plan_flow(args)),
        AstgBasicCommand::Current => plan_current(args),
        AstgBasicCommand::Persist => require_current(has_current_astg, PERSIST_USAGE)
            .and_then(|()| plan_persist(args)),
        AstgBasicCommand::LockGraph => require_current(has_current_astg, LOCKGRAPH_USAGE)
            .and_then(|()| plan_lockgraph(args)),
        AstgBasicCommand::Cycle => require_current(has_current_astg, CYCLE_USAGE)
            .and_then(|()| plan_cycle(args)),
        AstgBasicCommand::Write => require_current(has_current_astg, WRITE_USAGE)
            .and_then(|()| plan_write(args)),
        AstgBasicCommand::Marking => require_current(has_current_astg, MARKING_USAGE)
            .and_then(|()| plan_marking(args)),
        AstgBasicCommand::Irredundant => require_current(has_current_astg, IRREDUNDANT_USAGE)
            .and_then(|()| plan_irredundant(args)),
        AstgBasicCommand::Contract => require_current(has_current_astg, CONTRACT_USAGE)
            .and_then(|()| plan_contract(args)),
        AstgBasicCommand::StateMachineComponents =>
            require_current(has_current_astg, STATE_MACHINE_COMPONENT_USAGE)
                .and_then(|()| plan_components(args, STATE_MACHINE_COMPONENT_USAGE)),
        AstgBasicCommand::MarkedGraphComponents =>
            require_current(has_current_astg, MARKED_GRAPH_COMPONENT_USAGE)
                .and_then(|()| plan_components(args, MARKED_GRAPH_COMPONENT_USAGE)),
    }
}

pub fn render_current_astg(stg: Option<&AstgGraph>) -> String
{
    let Some(stg) = stg
    else
    {
        return "No current ASTG.\n".to_owned();
    };

    let mut text = String::new();
    text.push_str(&stg.name);
    text.push('\n');

    for comment in &stg.comments
    {
        text.push_str("  ");
        text.push_str(comment);
        text.push('\n');
    }

    text.push_str("\tFile: ");
    text.push_str(&stg.filename);
    if stg.file_change_count != stg.change_count
    {
        text.push_str(" (modified)");
    }
    text.push('\n');
    text.push_str(&format!(
        "\tPure: {}  Place-simple: {}\n",
        yes_no(stg.pure),
        yes_no(stg.place_simple)
    ));
    text.push_str(&format!(
        "\tConnected: {}  Strongly Connected: {}\n",
        yes_no(stg.connected_components == 1),
        yes_no(stg.strong_components == 1)
    ));
    text.push_str(&format!(
        "\tFree Choice: {}  Marked Graph: {}  State Machine: {}\n",
        yes_no(stg.free_choice),
        yes_no(stg.marked_graph),
        yes_no(stg.state_machine)
    ));

    if let Some(count) = stg.state_machine_components
    {
        text.push_str(&format!("\tSM components: {count}\n"));
    }

    if let Some(count) = stg.marked_graph_components
    {
        text.push_str(&format!("\tMG components: {count}\n"));
    }

    text
}

fn require_current(has_current_astg: bool, usage: AstgUsage) -> Result<(), AstgCommandError>
{
    if has_current_astg
    {
        Ok(())
    }
    else
    {
        Err(AstgCommandError::bad_option(usage))
    }
}

fn plan_read(args: &[String]) -> Result<AstgCommandPlan, AstgCommandError>
{
    if args.iter().any(|arg| arg.starts_with('-')) || args.len() > 1
    {
        return Err(AstgCommandError::bad_option(READ_USAGE));
    }

    Ok(AstgCommandPlan::Read(AstgReadOptions {
        infile: args.first().cloned(),
    }))
}

fn plan_flow(args: &[String]) -> Result<AstgCommandPlan, AstgCommandError>
{
    let mut options = AstgFlowOptions::default();
    let mut index = 0;

    while index < args.len()
    {
        match args[index].as_str()
        {
            "-x" =>
            {
                options.full_checks = false;
                index += 1;
            }
            "-q" =>
            {
                options.verbose = false;
                index += 1;
            }
            "-l" =>
            {
                index += 1;
                let Some(value) = args.get(index)
                else
                {
                    return Err(AstgCommandError::bad_option(FLOW_USAGE));
                };
                options.latch_type = value.clone();
                index += 1;
            }
            "-o" =>
            {
                index += 1;
                let Some(value) = args.get(index)
                else
                {
                    return Err(AstgCommandError::bad_option(FLOW_USAGE));
                };
                options.outfile = Some(value.clone());
                index += 1;
            }
            _ =>
            {
                return Err(AstgCommandError::bad_option(FLOW_USAGE));
            }
        }
    }

    Ok(AstgCommandPlan::Flow(options))
}

fn plan_write(args: &[String]) -> Result<AstgCommandPlan, AstgCommandError>
{
    let mut hide_places = true;
    let mut outfile = None;
    let mut index = 0;

    while index < args.len()
    {
        match args[index].as_str()
        {
            "-p" =>
            {
                hide_places = false;
                index += 1;
            }
            arg if !arg.starts_with('-') && outfile.is_none() =>
            {
                outfile = Some(arg.to_owned());
                index += 1;
            }
            _ =>
            {
                return Err(AstgCommandError::bad_option(WRITE_USAGE));
            }
        }
    }

    Ok(AstgCommandPlan::Write(AstgWriteOptions {
        hide_places,
        outfile,
    }))
}

fn plan_cycle(args: &[String]) -> Result<AstgCommandPlan, AstgCommandError>
{
    let mut options = AstgCycleOptions {
        through_transition: None,
        longest: false,
        append: false,
        count: false,
        cycle_index: 0,
    };
    let mut saw_cycle_index = false;
    let mut index = 0;

    while index < args.len()
    {
        match args[index].as_str()
        {
            "-a" =>
            {
                options.append = true;
                index += 1;
            }
            "-l" =>
            {
                options.longest = true;
                index += 1;
            }
            "-c" =>
            {
                options.count = true;
                index += 1;
            }
            "-t" =>
            {
                index += 1;
                let Some(value) = args.get(index)
                else
                {
                    return Err(AstgCommandError::bad_option(CYCLE_USAGE));
                };
                options.through_transition = Some(value.clone());
                index += 1;
            }
            arg if !arg.starts_with('-') && !saw_cycle_index =>
            {
                options.cycle_index = parse_nonnegative(arg).unwrap_or(0);
                saw_cycle_index = true;
                index += 1;
            }
            _ =>
            {
                return Err(AstgCommandError::bad_option(CYCLE_USAGE));
            }
        }
    }

    Ok(AstgCommandPlan::Cycle(options))
}

fn plan_lockgraph(args: &[String]) -> Result<AstgCommandPlan, AstgCommandError>
{
    if args.is_empty()
    {
        return Ok(AstgCommandPlan::LockGraph(AstgLockGraphOptions {
            form_one_lock_class: false,
        }));
    }

    if args == ["-l"]
    {
        return Ok(AstgCommandPlan::LockGraph(AstgLockGraphOptions {
            form_one_lock_class: true,
        }));
    }

    Err(AstgCommandError::bad_option(LOCKGRAPH_USAGE))
}

fn plan_persist(args: &[String]) -> Result<AstgCommandPlan, AstgCommandError>
{
    plan_print_or_modify(args, PERSIST_USAGE).map(AstgCommandPlan::Persist)
}

fn plan_irredundant(args: &[String]) -> Result<AstgCommandPlan, AstgCommandError>
{
    plan_print_or_modify(args, IRREDUNDANT_USAGE).map(AstgCommandPlan::Irredundant)
}

fn plan_print_or_modify(
    args: &[String],
    usage: AstgUsage,
) -> Result<AstgModifyOptions, AstgCommandError>
{
    match args
    {
        [] => Ok(AstgModifyOptions {
            modify: true,
        }),
        [arg] if arg == "-p" => Ok(AstgModifyOptions {
            modify: false,
        }),
        _ => Err(AstgCommandError::bad_option(usage)),
    }
}

fn plan_current(args: &[String]) -> Result<AstgCommandPlan, AstgCommandError>
{
    match args
    {
        [] => Ok(AstgCommandPlan::Current(AstgCurrentOptions {
            debug_level: None,
        })),
        [flag, value] if flag == "-d" =>
        {
            let Ok(debug_level) = value.parse::<i32>()
            else
            {
                return Err(AstgCommandError::bad_option(CURRENT_USAGE));
            };
            set_astg_debug_flag(debug_level);
            Ok(AstgCommandPlan::Current(AstgCurrentOptions {
                debug_level: Some(debug_level),
            }))
        }
        _ => Err(AstgCommandError::bad_option(CURRENT_USAGE)),
    }
}

fn plan_marking(args: &[String]) -> Result<AstgCommandPlan, AstgCommandError>
{
    let (by_state_code, rest) = match args.first().map(String::as_str)
    {
        Some("-s") => (true, &args[1..]),
        Some(arg) if arg.starts_with('-') => return Err(AstgCommandError::bad_option(MARKING_USAGE)),
        _ => (false, args),
    };

    if rest.is_empty()
    {
        return Ok(AstgCommandPlan::Marking(AstgMarkingOptions::Print {
            by_state_code,
        }));
    }

    if by_state_code
    {
        let mut values = Vec::new();
        let mut ignored_signals = Vec::new();
        let mut index = 0;

        while index < rest.len()
        {
            let signal = rest[index].clone();
            index += 1;
            let Some(value) = rest.get(index)
            else
            {
                ignored_signals.push(signal);
                break;
            };

            if let Ok(parsed) = value.parse::<i32>()
            {
                values.push((signal, parsed));
            }
            else
            {
                ignored_signals.push(signal);
            }
            index += 1;
        }

        return Ok(AstgCommandPlan::Marking(AstgMarkingOptions::SetSignals {
            values,
            ignored_signals,
        }));
    }

    if rest.len() == 1
    {
        return Ok(AstgCommandPlan::Marking(AstgMarkingOptions::SetPlaces {
            marking: rest[0].clone(),
        }));
    }

    Err(AstgCommandError::bad_option(MARKING_USAGE))
}

fn plan_components(
    args: &[String],
    usage: AstgUsage,
) -> Result<AstgCommandPlan, AstgCommandError>
{
    let mut components = Vec::with_capacity(args.len());

    for arg in args
    {
        if arg.starts_with('-')
        {
            return Err(AstgCommandError::bad_option(usage));
        }

        components.push(parse_nonnegative(arg).unwrap_or(0));
    }

    Ok(AstgCommandPlan::Components(components))
}

fn plan_contract(args: &[String]) -> Result<AstgCommandPlan, AstgCommandError>
{
    let mut keep_free_choice = false;
    let mut output_signal = None;
    let mut index = 0;

    while index < args.len()
    {
        match args[index].as_str()
        {
            "-f" =>
            {
                keep_free_choice = true;
                index += 1;
            }
            arg if !arg.starts_with('-') && output_signal.is_none() =>
            {
                output_signal = Some(arg.to_owned());
                index += 1;
            }
            _ =>
            {
                return Err(AstgCommandError::bad_option(CONTRACT_USAGE));
            }
        }
    }

    let Some(output_signal) = output_signal
    else
    {
        return Err(AstgCommandError::bad_option(CONTRACT_USAGE));
    };

    Ok(AstgCommandPlan::Contract(AstgContractOptions {
        keep_free_choice,
        output_signal,
    }))
}

fn parse_nonnegative(value: &str) -> Option<usize>
{
    value.parse::<isize>().ok().map(|value| value.max(0) as usize)
}

fn yes_no(value: bool) -> char
{
    if value
    {
        'Y'
    }
    else
    {
        'N'
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn args(values: &[&str]) -> Vec<String>
    {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn basic_command_table_matches_legacy_order_and_mutability()
    {
        let commands = astg_basic_command_registrations();
        let names = commands
            .iter()
            .map(|command| command.name)
            .collect::<Vec<_>>();
        let read_only = commands
            .iter()
            .filter(|command| !command.changes_network)
            .map(|command| command.name)
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                "read_astg",
                "_astg_flow",
                "astg_current",
                "astg_persist",
                "astg_lockgraph",
                "_astg_cycle",
                "write_astg",
                "astg_marking",
                "_astg_irred",
                "astg_contract",
                "_astg_smc",
                "_astg_mgc",
            ]
        );
        assert_eq!(
            read_only,
            vec![
                "astg_current",
                "_astg_cycle",
                "write_astg",
                "_astg_smc",
                "_astg_mgc",
            ]
        );
    }

    #[test]
    fn usage_replaces_only_first_command_placeholder()
    {
        assert_eq!(
            FLOW_USAGE.render("_astg_flow").lines().next(),
            Some("usage: _astg_flow [-x] [-l <latch-type>] [-o <outfile>] [-q]")
        );
    }

    #[test]
    fn set_current_allocates_network_and_can_clear_astg_without_reset()
    {
        let mut network = None;
        let report = astg_set_current(&mut network, Some(AstgGraph::new("sample")), true);

        assert_eq!(
            report,
            AstgSetCurrentReport {
                reset_network: false,
                allocated_network: true,
                installed_astg: true,
                detached_astg: None,
            }
        );
        assert_eq!(astg_current(network.as_ref()).unwrap().name, "sample");

        let report = astg_set_current(&mut network, None, false);

        assert_eq!(
            report,
            AstgSetCurrentReport {
                reset_network: false,
                allocated_network: false,
                installed_astg: false,
                detached_astg: Some(AstgGraph::new("sample")),
            }
        );
        assert!(network.as_ref().unwrap().astg().is_none());
    }

    #[test]
    fn flow_options_match_legacy_defaults_and_flags()
    {
        let plan = plan_basic_command(
            AstgBasicCommand::Flow,
            &args(&["-x", "-l", "r", "-o", "-", "-q"]),
            true,
        )
        .unwrap();

        assert_eq!(
            plan,
            AstgCommandPlan::Flow(AstgFlowOptions {
                full_checks: false,
                latch_type: "r".to_owned(),
                outfile: Some("-".to_owned()),
                verbose: false,
            })
        );
    }

    #[test]
    fn command_requires_current_astg_where_legacy_command_did()
    {
        let error = plan_basic_command(AstgBasicCommand::Write, &[], false).unwrap_err();

        assert_eq!(error.result, AstgResult::BadOption);
        assert_eq!(error.usage, WRITE_USAGE);
    }

    #[test]
    fn marking_signal_values_keep_legacy_bad_value_behavior()
    {
        let plan = plan_basic_command(
            AstgBasicCommand::Marking,
            &args(&["-s", "a", "1", "b", "bad", "c"]),
            true,
        )
        .unwrap();

        assert_eq!(
            plan,
            AstgCommandPlan::Marking(AstgMarkingOptions::SetSignals {
                values: vec![("a".to_owned(), 1)],
                ignored_signals: vec!["b".to_owned(), "c".to_owned()],
            })
        );
    }

    #[test]
    fn current_command_sets_debug_flag_and_render_reports_modified_graph()
    {
        let plan = plan_basic_command(AstgBasicCommand::Current, &args(&["-d", "2"]), true)
            .unwrap();

        assert_eq!(
            plan,
            AstgCommandPlan::Current(AstgCurrentOptions {
                debug_level: Some(2),
            })
        );
        assert_eq!(astg_debug_flag(), 2);

        let mut stg = AstgGraph::new("demo");
        stg.filename = "demo.g".to_owned();
        stg.change_count = 2;
        stg.file_change_count = 1;
        stg.pure = true;
        stg.connected_components = 1;
        stg.strong_components = 1;
        stg.state_machine_components = Some(3);

        let text = render_current_astg(Some(&stg));

        assert!(text.contains("demo\n"));
        assert!(text.contains("\tFile: demo.g (modified)\n"));
        assert!(text.contains("\tPure: Y  Place-simple: N\n"));
        assert!(text.contains("\tSM components: 3\n"));
    }

    #[test]
    fn component_and_contract_options_are_planned()
    {
        assert_eq!(
            plan_basic_command(
                AstgBasicCommand::StateMachineComponents,
                &args(&["2", "7"]),
                true,
            )
            .unwrap(),
            AstgCommandPlan::Components(vec![2, 7])
        );
        assert_eq!(
            plan_basic_command(AstgBasicCommand::Contract, &args(&["-f", "z"]), true).unwrap(),
            AstgCommandPlan::Contract(AstgContractOptions {
                keep_free_choice: true,
                output_signal: "z".to_owned(),
            })
        );
    }

    #[test]
    fn source_contains_no_tracking_metadata_or_c_abi_exports()
    {
        let source = include_str!("astg_cmds.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("be", "ad", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday1", "-", "8j8")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
