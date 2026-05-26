use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

pub type StateCode = u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalKind
{
    Input,
    Output,
    Internal,
    Dummy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signal
{
    pub name: String,
    pub state_bit: StateCode,
    pub kind: SignalKind,
}

impl Signal
{
    pub fn new(name: impl Into<String>, state_bit: StateCode, kind: SignalKind) -> Self
    {
        Self
        {
            name: name.into(),
            state_bit,
            kind,
        }
    }

    pub fn is_noninput(&self) -> bool
    {
        matches!(self.kind, SignalKind::Output | SignalKind::Internal)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgState
{
    pub code: StateCode,
    pub enabled: StateCode,
}

impl AstgState
{
    pub fn new(code: StateCode, enabled: StateCode) -> Self
    {
        Self
        {
            code,
            enabled,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgGraph
{
    pub name: String,
    pub signals: Vec<Signal>,
    pub states: Vec<AstgState>,
    pub initial_state: StateCode,
}

impl AstgGraph
{
    pub fn new(
        name: impl Into<String>,
        signals: Vec<Signal>,
        states: Vec<AstgState>,
        initial_state: StateCode,
    ) -> Self
    {
        Self
        {
            name: name.into(),
            signals,
            states,
            initial_state,
        }
    }

    fn signal_names(&self) -> BTreeSet<&str>
    {
        self.signals
            .iter()
            .map(|signal| signal.name.as_str())
            .collect()
    }

    fn noninput_signals(&self) -> impl Iterator<Item = &Signal>
    {
        self.signals.iter().filter(|signal| signal.is_noninput())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgBlif
{
    pub text: String,
    pub endpoints: Vec<SignalEndpoints>,
}

impl AstgBlif
{
    pub fn find_pi_or_po(&self, signal_name: &str, endpoint: AstgIoKind) -> Option<&str>
    {
        self.endpoints
            .iter()
            .find(|candidate| candidate.signal_name == signal_name)
            .map(|candidate| match endpoint
            {
                AstgIoKind::RealPi => candidate.real_pi.as_deref(),
                AstgIoKind::RealPo => candidate.real_po.as_deref(),
                AstgIoKind::FakePi => candidate.fake_pi.as_deref(),
                AstgIoKind::FakePo => candidate.fake_po.as_deref(),
            })?
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignalEndpoints
{
    pub signal_name: String,
    pub real_pi: Option<String>,
    pub real_po: Option<String>,
    pub fake_pi: Option<String>,
    pub fake_po: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AstgIoKind
{
    RealPi,
    RealPo,
    FakePi,
    FakePo,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AstgBlifError
{
    MissingModelName,
    MissingSignalName,
    EmptyLatchType,
    DuplicateStateBit
    {
        signal_name: String,
        state_bit: StateCode,
    },
    UnknownEnabledBit
    {
        state_code: StateCode,
        enabled_bit: StateCode,
    },
}

impl fmt::Display for AstgBlifError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::MissingModelName => write!(formatter, "ASTG model name is empty"),
            Self::MissingSignalName => write!(formatter, "ASTG signal name is empty"),
            Self::EmptyLatchType => write!(formatter, "ASTG BLIF latch type is empty"),
            Self::DuplicateStateBit
            {
                signal_name,
                state_bit,
            } => write!(
                formatter,
                "signal {signal_name} uses duplicate state bit {state_bit:#x}"
            ),
            Self::UnknownEnabledBit
            {
                state_code,
                enabled_bit,
            } => write!(
                formatter,
                "state {state_code:#x} enables unknown signal bit {enabled_bit:#x}"
            ),
        }
    }
}

impl Error for AstgBlifError
{
}

pub type AstgBlifResult<T> = Result<T, AstgBlifError>;

pub fn astg_to_blif(stg: &AstgGraph, latch_type: &str) -> AstgBlifResult<Option<AstgBlif>>
{
    validate_astg(stg, latch_type)?;

    let output_count = stg.noninput_signals().count();
    if output_count == 0
    {
        return Ok(None);
    }

    let signal_names = stg.signal_names();
    let mut fake_pi_names = Vec::with_capacity(output_count);
    let mut fake_po_names = Vec::with_capacity(output_count);
    let mut endpoints = Vec::with_capacity(stg.signals.len());

    for signal in &stg.signals
    {
        match signal.kind
        {
            SignalKind::Input =>
            {
                endpoints.push(SignalEndpoints
                {
                    signal_name: signal.name.clone(),
                    real_pi: Some(signal.name.clone()),
                    real_po: None,
                    fake_pi: None,
                    fake_po: None,
                });
            }
            SignalKind::Output | SignalKind::Internal =>
            {
                let fake_pi = make_fake_name(&signal_names, &signal.name, "_");
                let fake_po = make_fake_name(&signal_names, &signal.name, "_next");

                fake_pi_names.push(fake_pi.clone());
                fake_po_names.push(fake_po.clone());
                endpoints.push(SignalEndpoints
                {
                    signal_name: signal.name.clone(),
                    real_pi: None,
                    real_po: Some(signal.name.clone()),
                    fake_pi: Some(fake_pi),
                    fake_po: Some(fake_po),
                });
            }
            SignalKind::Dummy =>
            {
                endpoints.push(SignalEndpoints
                {
                    signal_name: signal.name.clone(),
                    real_pi: None,
                    real_po: None,
                    fake_pi: None,
                    fake_po: None,
                });
            }
        }
    }

    let mut output = String::new();

    output.push_str(".model ");
    output.push_str(&stg.name);
    output.push('\n');
    print_inputs(&mut output, ".inputs", stg, None);
    output.push('\n');
    print_real_outputs(&mut output, stg);

    let mut noninput_index = 0;
    for signal in &stg.signals
    {
        if !signal.is_noninput()
        {
            continue;
        }

        output.push_str(".latch ");
        output.push_str(&fake_po_names[noninput_index]);
        output.push(' ');
        output.push_str(&fake_pi_names[noninput_index]);
        output.push(' ');
        output.push_str(latch_type);
        output.push_str(" NIL ");
        output.push(
            if (signal.state_bit & stg.initial_state) != 0
            {
                '1'
            }
            else
            {
                '0'
            },
        );
        output.push('\n');
        noninput_index += 1;
    }

    noninput_index = 0;
    for signal in &stg.signals
    {
        if !signal.is_noninput()
        {
            continue;
        }

        print_inputs(&mut output, ".names", stg, Some(&fake_pi_names));
        output.push(' ');
        output.push_str(&fake_po_names[noninput_index]);
        output.push('\n');

        for state in &stg.states
        {
            if ((state.code ^ state.enabled) & signal.state_bit) != 0
            {
                print_cube(&mut output, stg, state.code, true);
            }
        }

        output.push_str(".names ");
        output.push_str(&fake_pi_names[noninput_index]);
        output.push(' ');
        output.push_str(&signal.name);
        output.push_str("\n1 1\n");
        noninput_index += 1;
    }

    output.push_str(".exdc\n");
    print_inputs(&mut output, ".inputs", stg, Some(&fake_pi_names));
    output.push_str("\n.outputs");
    noninput_index = 0;
    for signal in &stg.signals
    {
        if signal.is_noninput()
        {
            output.push(' ');
            output.push_str(&fake_po_names[noninput_index]);
            noninput_index += 1;
        }
    }
    output.push('\n');

    let dc_name = make_fake_name(&signal_names, "DC", "");
    print_inputs(&mut output, ".names", stg, Some(&fake_pi_names));
    output.push(' ');
    output.push_str(&dc_name);
    output.push('\n');
    for state in &stg.states
    {
        print_cube(&mut output, stg, state.code, false);
    }

    noninput_index = 0;
    for signal in &stg.signals
    {
        if signal.is_noninput()
        {
            output.push_str(".names ");
            output.push_str(&dc_name);
            output.push(' ');
            output.push_str(&fake_po_names[noninput_index]);
            output.push_str("\n1 1\n");
            noninput_index += 1;
        }
    }

    output.push_str(".end\n");

    Ok(Some(AstgBlif
    {
        text: output,
        endpoints,
    }))
}

pub fn astg_find_pi_or_po<'a>(
    blif: &'a AstgBlif,
    signal_name: &str,
    endpoint: AstgIoKind,
) -> Option<&'a str>
{
    blif.find_pi_or_po(signal_name, endpoint)
}

fn validate_astg(stg: &AstgGraph, latch_type: &str) -> AstgBlifResult<()>
{
    if stg.name.is_empty()
    {
        return Err(AstgBlifError::MissingModelName);
    }

    if latch_type.is_empty()
    {
        return Err(AstgBlifError::EmptyLatchType);
    }

    let mut seen_bits = BTreeSet::new();
    let known_bits = stg
        .signals
        .iter()
        .fold(0, |known_bits, signal| known_bits | signal.state_bit);

    for signal in &stg.signals
    {
        if signal.name.is_empty()
        {
            return Err(AstgBlifError::MissingSignalName);
        }

        if signal.state_bit != 0 && !seen_bits.insert(signal.state_bit)
        {
            return Err(AstgBlifError::DuplicateStateBit
            {
                signal_name: signal.name.clone(),
                state_bit: signal.state_bit,
            });
        }
    }

    for state in &stg.states
    {
        let unknown = state.enabled & !known_bits;
        if unknown != 0
        {
            let enabled_bit = unknown & unknown.wrapping_neg();

            return Err(AstgBlifError::UnknownEnabledBit
            {
                state_code: state.code,
                enabled_bit,
            });
        }
    }

    Ok(())
}

fn make_fake_name(signal_names: &BTreeSet<&str>, name: &str, suffix: &str) -> String
{
    let base = format!("{name}{suffix}");
    if !signal_names.contains(base.as_str())
    {
        return base;
    }

    let mut index = 1;
    loop
    {
        let candidate = format!("{base}{index}");
        if !signal_names.contains(candidate.as_str())
        {
            return candidate;
        }

        index += 1;
    }
}

fn print_inputs(output: &mut String, header: &str, stg: &AstgGraph, fake_pis: Option<&[String]>)
{
    let mut noninput_index = 0;

    output.push_str(header);
    for signal in &stg.signals
    {
        match signal.kind
        {
            SignalKind::Input =>
            {
                output.push(' ');
                output.push_str(&signal.name);
            }
            SignalKind::Output | SignalKind::Internal =>
            {
                if let Some(fake_pis) = fake_pis
                {
                    output.push(' ');
                    output.push_str(&fake_pis[noninput_index]);
                }

                noninput_index += 1;
            }
            SignalKind::Dummy =>
            {
            }
        }
    }
}

fn print_real_outputs(output: &mut String, stg: &AstgGraph)
{
    output.push_str(".outputs");
    for signal in &stg.signals
    {
        if signal.is_noninput()
        {
            output.push(' ');
            output.push_str(&signal.name);
        }
    }
    output.push('\n');
}

fn print_cube(output: &mut String, stg: &AstgGraph, code: StateCode, value: bool)
{
    for signal in &stg.signals
    {
        if matches!(
            signal.kind,
            SignalKind::Input | SignalKind::Output | SignalKind::Internal
        )
        {
            output.push(
                if (signal.state_bit & code) != 0
                {
                    '1'
                }
                else
                {
                    '0'
                },
            );
        }
    }

    output.push(' ');
    output.push(
        if value
        {
            '1'
        }
        else
        {
            '0'
        },
    );
    output.push('\n');
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn sample_graph() -> AstgGraph
    {
        AstgGraph::new(
            "demo",
            vec![
                Signal::new("a", 0b001, SignalKind::Input),
                Signal::new("b", 0b010, SignalKind::Output),
                Signal::new("c", 0b100, SignalKind::Internal),
            ],
            vec![
                AstgState::new(0b000, 0b010),
                AstgState::new(0b010, 0b100),
                AstgState::new(0b110, 0b010),
            ],
            0b010,
        )
    }

    #[test]
    fn astg_to_blif_writes_sequential_network()
    {
        let blif = astg_to_blif(&sample_graph(), "re").unwrap().unwrap();

        assert_eq!(
            blif.text,
            concat!(
                ".model demo\n",
                ".inputs a\n",
                ".outputs b c\n",
                ".latch b_next b_ re NIL 1\n",
                ".latch c_next c_ re NIL 0\n",
                ".names a b_ c_ b_next\n",
                "000 1\n",
                "010 1\n",
                ".names b_ b\n",
                "1 1\n",
                ".names a b_ c_ c_next\n",
                "010 1\n",
                "011 1\n",
                ".names c_ c\n",
                "1 1\n",
                ".exdc\n",
                ".inputs a b_ c_\n",
                ".outputs b_next c_next\n",
                ".names a b_ c_ DC\n",
                "000 0\n",
                "010 0\n",
                "011 0\n",
                ".names DC b_next\n",
                "1 1\n",
                ".names DC c_next\n",
                "1 1\n",
                ".end\n",
            )
        );
    }

    #[test]
    fn fake_names_avoid_existing_signal_names()
    {
        let stg = AstgGraph::new(
            "conflict",
            vec![
                Signal::new("x", 0b001, SignalKind::Output),
                Signal::new("x_", 0b010, SignalKind::Input),
                Signal::new("x_next", 0b100, SignalKind::Input),
            ],
            vec![AstgState::new(0, 0b001)],
            0,
        );

        let blif = astg_to_blif(&stg, "re").unwrap().unwrap();

        assert!(blif.text.contains(".latch x_next1 x_1 re NIL 0\n"));
        assert_eq!(blif.find_pi_or_po("x", AstgIoKind::FakePi), Some("x_1"));
        assert_eq!(
            astg_find_pi_or_po(&blif, "x", AstgIoKind::FakePo),
            Some("x_next1")
        );
    }

    #[test]
    fn endpoint_lookup_distinguishes_real_and_fake_interfaces()
    {
        let blif = astg_to_blif(&sample_graph(), "re").unwrap().unwrap();

        assert_eq!(blif.find_pi_or_po("a", AstgIoKind::RealPi), Some("a"));
        assert_eq!(blif.find_pi_or_po("a", AstgIoKind::RealPo), None);
        assert_eq!(blif.find_pi_or_po("b", AstgIoKind::RealPo), Some("b"));
        assert_eq!(blif.find_pi_or_po("b", AstgIoKind::FakePi), Some("b_"));
        assert_eq!(blif.find_pi_or_po("b", AstgIoKind::FakePo), Some("b_next"));
        assert_eq!(blif.find_pi_or_po("missing", AstgIoKind::RealPi), None);
    }

    #[test]
    fn input_only_graph_generates_no_network()
    {
        let stg = AstgGraph::new(
            "inputs",
            vec![Signal::new("a", 0b001, SignalKind::Input)],
            vec![AstgState::new(0, 0)],
            0,
        );

        assert_eq!(astg_to_blif(&stg, "re").unwrap(), None);
    }

    #[test]
    fn validation_rejects_unknown_enabled_bits()
    {
        let stg = AstgGraph::new(
            "bad",
            vec![Signal::new("a", 0b001, SignalKind::Output)],
            vec![AstgState::new(0, 0b010)],
            0,
        );

        assert_eq!(
            astg_to_blif(&stg, "re").unwrap_err(),
            AstgBlifError::UnknownEnabledBit
            {
                state_code: 0,
                enabled_bit: 0b010,
            }
        );
    }

    #[test]
    fn no_legacy_exports_or_dependency_metadata_are_present()
    {
        let source = include_str!("astg_blif.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
