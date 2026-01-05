use crate::machine::{Error, Side};
use machine_core::types::{
    projection::OptionGraph,
    proto_graph::NodeId,
    typescript_types::{Command, EventType, MachineLabel},
};
use petgraph::{visit::EdgeRef, Direction::Outgoing};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
enum DeterministicLabel {
    Command(Command),
    Event(EventType),
}

impl From<&MachineLabel> for DeterministicLabel {
    fn from(label: &MachineLabel) -> Self {
        match label {
            MachineLabel::Execute { cmd, .. } => DeterministicLabel::Command(cmd.clone()),
            MachineLabel::Input { event_type } => DeterministicLabel::Event(event_type.clone()),
        }
    }
}
fn state_name(graph: &OptionGraph, index: NodeId) -> String {
    match &graph[index] {
        None => "".to_string(),
        Some(s) => s.to_string(),
    }
}
/// error messages are designed assuming that `left` is the reference and `right` the tested
pub fn equivalent(left: &OptionGraph, li: NodeId, right: &OptionGraph, ri: NodeId) -> Vec<Error> {
    use Side::*;

    let _span = tracing::info_span!("equivalent").entered();

    let mut errors = Vec::new();

    // dfs traversal stack
    // must hold index pairs because node mappings might be m:n
    let mut stack = vec![(li, ri)];
    let mut visited = BTreeSet::new();

    while let Some((li, ri)) = stack.pop() {
        tracing::debug!(left = %state_name(left, li), ?li, right = %state_name(right, ri), ?ri, to_go = stack.len(), "loop");
        visited.insert((li, ri));
        // get all outgoing edge labels for the left side
        let mut l_out = BTreeMap::new();
        for edge in left.edges_directed(li, Outgoing) {
            l_out
                .entry(DeterministicLabel::from(edge.weight()))
                .and_modify(|_| errors.push(Error::NonDeterministic(Left, edge.id())))
                .or_insert(edge);
        }
        // get all outgoing edge labels for the right side
        let mut r_out = BTreeMap::new();
        for edge in right.edges_directed(ri, Outgoing) {
            r_out
                .entry(DeterministicLabel::from(edge.weight()))
                .and_modify(|_| errors.push(Error::NonDeterministic(Right, edge.id())))
                .or_insert(edge);
        }
        // keep note of stack so we can undo additions if !same
        let stack_len = stack.len();

        // compare both sets; iteration must be in order of weights (hence the BTreeMap above)
        let mut same = true;
        let mut l_edges = l_out.into_values().peekable();
        let mut r_edges = r_out.into_values().peekable();
        loop {
            let l = l_edges.peek();
            let r = r_edges.peek();
            match (l, r) {
                (None, None) => break,
                (None, Some(r_edge)) => {
                    tracing::debug!("left missing {} 1", r_edge.weight());
                    errors.push(Error::MissingTransition(Left, li, r_edge.id()));
                    same = false;
                    r_edges.next();
                }
                (Some(l_edge), None) => {
                    tracing::debug!("right missing {} 2", l_edge.weight());
                    errors.push(Error::MissingTransition(Right, ri, l_edge.id()));
                    same = false;
                    l_edges.next();
                }
                (Some(l_edge), Some(r_edge)) => match l_edge.weight().cmp(r_edge.weight()) {
                    Ordering::Less => {
                        tracing::debug!("right missing {}", l_edge.weight());
                        errors.push(Error::MissingTransition(Right, ri, l_edge.id()));
                        same = false;
                        l_edges.next();
                    }
                    Ordering::Equal => {
                        tracing::debug!("found match for {}", l_edge.weight());
                        let lt = l_edge.target();
                        let rt = r_edge.target();
                        if !visited.contains(&(lt, rt)) {
                            tracing::debug!(?lt, ?rt, "pushing targets");
                            stack.push((lt, rt));
                        }

                        l_edges.next();
                        r_edges.next();
                    }
                    Ordering::Greater => {
                        tracing::debug!("left missing {}", r_edge.weight());
                        errors.push(Error::MissingTransition(Left, li, r_edge.id()));
                        same = false;
                        r_edges.next();
                    }
                },
            }
        }
        if !same {
            // don’t bother visiting subsequent nodes if this one had discrepancies
            tracing::debug!("dumping {} stack elements", stack.len() - stack_len);
            stack.truncate(stack_len);
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use machine_core::types::{
        projection::Graph,
        typescript_types::{
            Command, DataResult, EventType, Granularity, InterfacingProtocols, MachineType, Role,
            StateName, Subscriptions, SubscriptionsWrapped, SwarmProtocolType, Transition,
        },
    };
    use tracing_subscriber::{fmt, fmt::format::FmtSpan, EnvFilter};
    fn setup_logger() {
        fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
            .try_init()
            .ok();
    }

    fn from_option_machine(graph: &OptionGraph) -> Graph {
        graph.map(
            |_, n| n.clone().unwrap().state_name().clone(),
            |_, x| x.clone(),
        )
    }
    fn to_option_machine(graph: &Graph) -> OptionGraph {
        graph.map(|_, n| Some(n.state_name().clone()), |_, x| x.clone())
    }

    fn get_proto1() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "1", "target": "2", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                    { "source": "2", "target": "0", "label": { "cmd": "deliver", "logType": ["part"], "role": "T" } },
                    { "source": "0", "target": "3", "label": { "cmd": "close", "logType": ["time"], "role": "D" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_proto2() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "1", "target": "2", "label": { "cmd": "deliver", "logType": ["part"], "role": "T" } },
                    { "source": "2", "target": "3", "label": { "cmd": "build", "logType": ["car"], "role": "F" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_proto3() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                    { "source": "1", "target": "2", "label": { "cmd": "test", "logType": ["report"], "role": "TR" } },
                    { "source": "2", "target": "3", "label": { "cmd": "accept", "logType": ["ok"], "role": "QCR" } },
                    { "source": "2", "target": "3", "label": { "cmd": "reject", "logType": ["notOk"], "role": "QCR" } }
                ]
            }"#,
        )
        .unwrap()
    }

    fn get_interfacing_swarms_1() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto1(), get_proto2()])
    }

    fn get_interfacing_swarms_1_reversed() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto2(), get_proto1()])
    }

    fn get_interfacing_swarms_2() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto1(), get_proto2(), get_proto3()])
    }

    fn get_interfacing_swarms_2_reversed() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto3(), get_proto2(), get_proto1()])
    }

    fn get_whf_transport() -> MachineType {
        serde_json::from_str::<MachineType>(
            r#"{
                "initial": "0",
                "transitions": [
                    {
                    "label": {
                        "tag": "Execute",
                        "cmd": "request",
                        "logType": [
                        "partID"
                        ]
                    },
                    "source": "0",
                    "target": "0"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "0",
                    "target": "5"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "partID"
                    },
                    "source": "0",
                    "target": "1"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "pos"
                    },
                    "source": "1",
                    "target": "2"
                    },
                    {
                    "label": {
                        "tag": "Execute",
                        "cmd": "deliver",
                        "logType": [
                        "part"
                        ]
                    },
                    "source": "2",
                    "target": "2"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "part"
                    },
                    "source": "2",
                    "target": "3"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "3",
                    "target": "4"
                    }
                ]
                }
            "#,
        )
        .unwrap()
    }

    fn get_whf_door() -> MachineType {
        serde_json::from_str::<MachineType>(
            r#"{
                "initial": "0",
                "transitions": [
                    {
                    "label": {
                        "tag": "Execute",
                        "cmd": "close",
                        "logType": [
                        "time"
                        ]
                    },
                    "source": "0",
                    "target": "0"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "0",
                    "target": "4"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "partID"
                    },
                    "source": "0",
                    "target": "1"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "part"
                    },
                    "source": "1",
                    "target": "2"
                    },
                    {
                    "label": {
                        "tag": "Execute",
                        "cmd": "close",
                        "logType": [
                        "time"
                        ]
                    },
                    "source": "2",
                    "target": "2"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "2",
                    "target": "3"
                    }
                ]
                }
            "#,
        )
        .unwrap()
    }

    fn get_whf_forklift() -> MachineType {
        serde_json::from_str::<MachineType>(
            r#"{
                "initial": "0",
                "transitions": [
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "0",
                    "target": "5"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "partID"
                    },
                    "source": "0",
                    "target": "1"
                    },
                    {
                    "label": {
                        "tag": "Execute",
                        "cmd": "get",
                        "logType": [
                        "pos"
                        ]
                    },
                    "source": "1",
                    "target": "1"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "pos"
                    },
                    "source": "1",
                    "target": "2"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "part"
                    },
                    "source": "2",
                    "target": "3"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "3",
                    "target": "4"
                    }
                ]
                }
            "#,
        )
        .unwrap()
    }

    fn get_whf_f() -> MachineType {
        serde_json::from_str::<MachineType>(
            r#"{
                "initial": "0",
                "transitions": [
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "0",
                    "target": "6"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "partID"
                    },
                    "source": "0",
                    "target": "1"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "part"
                    },
                    "source": "1",
                    "target": "2"
                    },
                    {
                    "label": {
                        "tag": "Execute",
                        "cmd": "build",
                        "logType": [
                        "car"
                        ]
                    },
                    "source": "2",
                    "target": "2"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "2",
                    "target": "3"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "car"
                    },
                    "source": "2",
                    "target": "4"
                    },
                    {
                    "label": {
                        "tag": "Execute",
                        "cmd": "build",
                        "logType": [
                        "car"
                        ]
                    },
                    "source": "3",
                    "target": "3"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "4",
                    "target": "5"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "car"
                    },
                    "source": "3",
                    "target": "5"
                    }
                ]
                }
            "#,
        )
        .unwrap()
    }

    use machine_core::types::typescript_types::State;

    #[test]
    fn test_equivalent_1() {
        setup_logger();

        let proto = serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["tireID"], "role": "C" } },
                    { "source": "1", "target": "2", "label": { "cmd": "retrieve", "logType": ["position"], "role": "W" } },
                    { "source": "2", "target": "3", "label": { "cmd": "receive", "logType": ["tire"], "role": "C" } },
                    { "source": "3", "target": "4", "label": { "cmd": "build", "logType": ["car"], "role": "F" } }
                ]
            }"#,
        )
        .unwrap();

        let subs = serde_json::from_str::<Subscriptions>(
            r#"{
            "C":["tireID","position","tire","car"],
            "W":["tireID","position","tire"],
            "F":["tireID","tire","car"]
        }"#,
        )
        .unwrap();

        let role = Role::new("F");
        let (proj, proj_initial, _) = match machine_core::project(
            InterfacingProtocols(vec![proto]),
            SubscriptionsWrapped(subs),
            role,
            false,
            false,
        ) {
            DataResult::ERROR { errors: _ } => panic!(),
            DataResult::OK { data } => crate::machine::from_json(data),
        };
        let expected_m = MachineType {
            initial: State::new("0"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("tireID"),
                    },
                    source: State::new("0"),
                    target: State::new("2"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("tire"),
                    },
                    source: State::new("2"),
                    target: State::new("3"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("build"),
                        log_type: vec![EventType::new("car")],
                    },
                    source: State::new("3"),
                    target: State::new("3"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("car"),
                    },
                    source: State::new("3"),
                    target: State::new("4"),
                },
            ],
        };
        let (expected, expected_initial, errors) = crate::machine::from_json(expected_m);
        assert!(errors.is_empty());
        assert!(expected_initial.is_some());
        // from equivalent(): "error messages are designed assuming that `left` is the reference and `right` the tested"
        assert!(equivalent(
            &expected,
            expected_initial.unwrap(),
            &proj,
            proj_initial.unwrap()
        )
        .is_empty());
    }

    #[test]
    fn test_equivalent_2() {
        setup_logger();
        let proto = get_proto1();
        let subs = match machine_core::exact_well_formed_sub(
            InterfacingProtocols(vec![proto.clone()]),
            SubscriptionsWrapped(BTreeMap::new()),
        ) {
            DataResult::ERROR { errors: _ } => panic!(),
            DataResult::OK { data } => data,
        };
        let role = Role::new("FL");
        let (left, left_initial, _) = match machine_core::project(
            InterfacingProtocols(vec![proto]),
            SubscriptionsWrapped(subs),
            role,
            false,
            false,
        ) {
            DataResult::ERROR { errors: _ } => panic!(),
            DataResult::OK { data } => crate::machine::from_json(data),
        };
        let right_m = MachineType {
            initial: State::new("0"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("0"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("get"),
                        log_type: vec![EventType::new("pos")],
                    },
                    source: State::new("1"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("pos"),
                    },
                    source: State::new("1"),
                    target: State::new("2"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("2"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("2"),
                    target: State::new("3"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0"),
                    target: State::new("3"),
                },
            ],
        };
        let (right, right_initial, errors) = crate::machine::from_json(right_m);
        let right = from_option_machine(&right);
        let right = to_option_machine(&right);

        assert!(errors.is_empty());

        let errors = equivalent(&left, left_initial.unwrap(), &right, right_initial.unwrap());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_equivalent_3() {
        setup_logger();
        let proto = get_proto2();
        let subs = match machine_core::exact_well_formed_sub(
            InterfacingProtocols(vec![proto.clone()]),
            SubscriptionsWrapped(BTreeMap::new()),
        ) {
            DataResult::ERROR { errors: _ } => panic!(),
            DataResult::OK { data } => data,
        };
        let role = Role::new("F");
        let (proj, proj_initial, _) = match machine_core::project(
            InterfacingProtocols(vec![proto]),
            SubscriptionsWrapped(subs),
            role,
            false,
            false,
        ) {
            DataResult::ERROR { errors: _ } => panic!(),
            DataResult::OK { data } => crate::machine::from_json(data),
        };
        let expected_m = MachineType {
            initial: State::new("1"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("part"),
                    },
                    source: State::new("1"),
                    target: State::new("2"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("build"),
                        log_type: vec![EventType::new("car")],
                    },
                    source: State::new("2"),
                    target: State::new("2"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("car"),
                    },
                    source: State::new("2"),
                    target: State::new("3"),
                },
            ],
        };
        let (expected, expected_initial, errors) = crate::machine::from_json(expected_m);

        assert!(errors.is_empty());
        assert!(expected_initial.is_some());
        // from equivalent(): "error messages are designed assuming that `left` is the reference and `right` the tested"
        assert!(equivalent(
            &expected,
            expected_initial.unwrap(),
            &proj,
            proj_initial.unwrap()
        )
        .is_empty());
    }

    #[test]
    fn test_equivalent_4() {
        setup_logger();
        let protos = get_interfacing_swarms_1();
        let subs = match machine_core::overapproximated_well_formed_sub(
            protos.clone(),
            SubscriptionsWrapped(BTreeMap::from([(
                Role::new("T"),
                BTreeSet::from([EventType::new("car")]),
            )])),
            Granularity::Coarse,
        ) {
            DataResult::ERROR { errors: _ } => panic!(),
            DataResult::OK { data } => data,
        };

        let role = Role::new("T");
        let (proj, proj_initial, _) =
            match machine_core::project(protos, SubscriptionsWrapped(subs), role, false, false) {
                DataResult::ERROR { errors: _ } => panic!(),
                DataResult::OK { data } => crate::machine::from_json(data),
            };
        let expected_m = MachineType {
            initial: State::new("0"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("request"),
                        log_type: vec![EventType::new("partID")],
                    },
                    source: State::new("0"),
                    target: State::new("0"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("0"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0"),
                    target: State::new("2"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("pos"),
                    },
                    source: State::new("1"),
                    target: State::new("3"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("deliver"),
                        log_type: vec![EventType::new("part")],
                    },
                    source: State::new("3"),
                    target: State::new("3"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("part"),
                    },
                    source: State::new("3"),
                    target: State::new("4"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("4"),
                    target: State::new("5"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("car"),
                    },
                    source: State::new("5"),
                    target: State::new("7"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("car"),
                    },
                    source: State::new("4"),
                    target: State::new("6"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("6"),
                    target: State::new("7"),
                },
            ],
        };
        let (expected, expected_initial, errors) = crate::machine::from_json(expected_m);

        assert!(errors.is_empty());
        assert!(expected_initial.is_some());
        // from equivalent(): "error messages are designed assuming that `left` is the reference and `right` the tested"
        assert!(equivalent(
            &expected,
            expected_initial.unwrap(),
            &proj,
            proj_initial.unwrap()
        )
        .is_empty());
    }

    #[test]
    fn test_equivalent_fail_1() {
        setup_logger();
        // warehouse example from coplaws slides
        let proto = get_proto1();
        let subs = match machine_core::exact_well_formed_sub(
            InterfacingProtocols(vec![proto.clone()]),
            SubscriptionsWrapped(BTreeMap::new()),
        ) {
            DataResult::ERROR { errors: _ } => panic!(),
            DataResult::OK { data } => data,
        };
        let role = Role::new("FL");
        let (left, left_initial, _) = match machine_core::project(
            InterfacingProtocols(vec![proto]),
            SubscriptionsWrapped(subs),
            role,
            false,
            false,
        ) {
            DataResult::ERROR { errors: _ } => panic!(),
            DataResult::OK { data } => crate::machine::from_json(data),
        };
        let right_m = MachineType {
            initial: State::new("0"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("0"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("get"),
                        log_type: vec![EventType::new("pos")],
                    },
                    source: State::new("1"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("pos"),
                    },
                    source: State::new("1"),
                    target: State::new("2"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("1"),
                    target: State::new("3"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("2"),
                    target: State::new("3"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0"),
                    target: State::new("3"),
                },
            ],
        };
        let (right, right_initial, errors) = crate::machine::from_json(right_m);
        let right = from_option_machine(&right);
        let right = to_option_machine(&right);

        assert!(errors.is_empty());

        let errors = equivalent(&left, left_initial.unwrap(), &right, right_initial.unwrap());
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_projection_fail_2() {
        setup_logger();
        // warehouse example from coplaws slides
        let proto = get_proto1();
        let subs = match machine_core::exact_well_formed_sub(
            InterfacingProtocols(vec![proto.clone()]),
            SubscriptionsWrapped(BTreeMap::new()),
        ) {
            DataResult::ERROR { errors: _ } => panic!(),
            DataResult::OK { data } => data,
        };
        let role = Role::new("FL");
        let (left, left_initial, _) = match machine_core::project(
            InterfacingProtocols(vec![proto]),
            SubscriptionsWrapped(subs),
            role,
            false,
            false,
        ) {
            DataResult::ERROR { errors: _ } => panic!(),
            DataResult::OK { data } => crate::machine::from_json(data),
        };
        let right_m = MachineType {
            initial: State::new("0"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("0"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("get"),
                        log_type: vec![EventType::new("pos")],
                    },
                    source: State::new("1"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("pos"),
                    },
                    source: State::new("1"),
                    target: State::new("2"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("2"),
                    target: State::new("2"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("2"),
                    target: State::new("3"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0"),
                    target: State::new("3"),
                },
            ],
        };
        let (right, right_initial, errors) = crate::machine::from_json(right_m);
        let right = from_option_machine(&right);
        let right = to_option_machine(&right);

        assert!(errors.is_empty());

        let errors = equivalent(&left, left_initial.unwrap(), &right, right_initial.unwrap());
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_projection_fail_3() {
        setup_logger();
        // warehouse example from coplaws slides
        let proto = get_proto1();
        let subs = match machine_core::exact_well_formed_sub(
            InterfacingProtocols(vec![proto.clone()]),
            SubscriptionsWrapped(BTreeMap::new()),
        ) {
            DataResult::ERROR { errors: _ } => panic!(),
            DataResult::OK { data } => data,
        };
        let role = Role::new("FL");
        let (left, left_initial, _) = match machine_core::project(
            InterfacingProtocols(vec![proto]),
            SubscriptionsWrapped(subs),
            role,
            false,
            false,
        ) {
            DataResult::ERROR { errors: _ } => panic!(),
            DataResult::OK { data } => crate::machine::from_json(data),
        };
        let right_m = MachineType {
            initial: State::new("0"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("0"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("get"),
                        log_type: vec![EventType::new("pos")],
                    },
                    source: State::new("1"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("pos"),
                    },
                    source: State::new("1"),
                    target: State::new("2"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("get"),
                        log_type: vec![EventType::new("pos")],
                    },
                    source: State::new("2"),
                    target: State::new("2"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("2"),
                    target: State::new("3"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0"),
                    target: State::new("3"),
                },
            ],
        };
        let (right, right_initial, errors) = crate::machine::from_json(right_m);
        let right = from_option_machine(&right);
        let right = to_option_machine(&right);

        assert!(errors.is_empty());

        let errors = equivalent(&left, left_initial.unwrap(), &right, right_initial.unwrap());
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_projection_fail_4() {
        setup_logger();
        // warehouse example from coplaws slides
        let proto = get_proto1();
        let subs = match machine_core::exact_well_formed_sub(
            InterfacingProtocols(vec![proto.clone()]),
            SubscriptionsWrapped(BTreeMap::new()),
        ) {
            DataResult::ERROR { errors: _ } => panic!(),
            DataResult::OK { data } => data,
        };
        let role = Role::new("FL");
        let (left, left_initial, _) = match machine_core::project(
            InterfacingProtocols(vec![proto]),
            SubscriptionsWrapped(subs),
            role,
            false,
            false,
        ) {
            DataResult::ERROR { errors: _ } => panic!(),
            DataResult::OK { data } => crate::machine::from_json(data),
        };
        let right_m = MachineType {
            initial: State::new("0"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("0"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("get"),
                        log_type: vec![EventType::new("pos")],
                    },
                    source: State::new("1"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("pos"),
                    },
                    source: State::new("1"),
                    target: State::new("2"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("2"),
                    target: State::new("3"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0"),
                    target: State::new("3"),
                },
            ],
        };
        let (right, right_initial, errors) = crate::machine::from_json(right_m);
        let right = from_option_machine(&right);
        let right = to_option_machine(&right);

        assert!(errors.is_empty());

        let errors = equivalent(&left, left_initial.unwrap(), &right, right_initial.unwrap());
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_combine_machines_equal_1() {
        setup_logger();
        // Example from coplaws slides. Use generated WWF subscriptions. Project over T.
        let role = Role::new("T");
        let subs1 = match machine_core::overapproximated_well_formed_sub(
            get_interfacing_swarms_1(),
            SubscriptionsWrapped(BTreeMap::new()),
            Granularity::TwoStep,
        ) {
            DataResult::OK { data } => data,
            DataResult::ERROR { errors } => {
                println!("{}", errors.join(", "));
                panic!()
            }
        };

        let (proj_combined1, proj_combined_initial1, _) = match machine_core::project(
            get_interfacing_swarms_1(),
            SubscriptionsWrapped(subs1.clone()),
            role.clone(),
            false,
            false,
        ) {
            DataResult::OK { data } => crate::machine::from_json(data),
            DataResult::ERROR { errors } => {
                println!("{}", errors.join(", "));
                panic!()
            }
        };

        let subs2 = match machine_core::overapproximated_well_formed_sub(
            get_interfacing_swarms_1_reversed(),
            SubscriptionsWrapped(BTreeMap::new()),
            Granularity::TwoStep,
        ) {
            DataResult::OK { data } => data,
            DataResult::ERROR { errors } => {
                println!("{}", errors.join(", "));
                panic!()
            }
        };

        let (proj_combined2, proj_combined_initial2, _) = match machine_core::project(
            get_interfacing_swarms_1_reversed(),
            SubscriptionsWrapped(subs2.clone()),
            role.clone(),
            true,
            false,
        ) {
            DataResult::OK { data } => crate::machine::from_json(data),
            DataResult::ERROR { errors } => {
                println!("{}", errors.join(", "));
                panic!()
            }
        };

        let (proj_expanded_proto, proj_expanded_proto_initial, _) = match machine_core::project(
            get_interfacing_swarms_1(),
            SubscriptionsWrapped(subs1.clone()),
            role.clone(),
            true,
            true,
        ) {
            DataResult::OK { data } => crate::machine::from_json(data),
            DataResult::ERROR { errors } => {
                println!("{}", errors.join(", "));
                panic!()
            }
        };
        // compose(a, b) should be equal to compose(b, a)
        assert_eq!(subs1, subs2);
        assert!(equivalent(
            &proj_combined1,
            proj_combined_initial1.unwrap(),
            &proj_combined2,
            proj_combined_initial2.unwrap()
        )
        .is_empty());

        assert!(equivalent(
            &proj_combined2,
            proj_combined_initial2.unwrap(),
            &proj_expanded_proto,
            proj_expanded_proto_initial.unwrap()
        )
        .is_empty());
    }

    #[test]
    fn test_combine_machines_equal_2() {
        setup_logger();
        // Fails when you use the exact subscriptions because that way not all involved roles subscribe to ALL interfaces. Ordering gets messed up.
        // The projected over the explicit composition may be correct, but the combined projections look weird and out of order.
        let subs1 = match machine_core::overapproximated_well_formed_sub(
            get_interfacing_swarms_2(),
            SubscriptionsWrapped(BTreeMap::new()),
            Granularity::TwoStep,
        ) {
            DataResult::OK { data } => data,
            DataResult::ERROR { errors } => {
                println!("{}", errors.join(", "));
                panic!()
            }
        };
        let subs2 = match machine_core::overapproximated_well_formed_sub(
            get_interfacing_swarms_2_reversed(),
            SubscriptionsWrapped(BTreeMap::new()),
            Granularity::TwoStep,
        ) {
            DataResult::OK { data } => data,
            DataResult::ERROR { errors } => {
                println!("{}", errors.join(", "));
                panic!()
            }
        };
        assert_eq!(subs1, subs2);
        let all_roles = vec![
            Role::new("T"),
            Role::new("FL"),
            Role::new("D"),
            Role::new("F"),
            Role::new("TR"),
            Role::new("QCR"),
        ];

        for role in all_roles {
            let (proj_combined1, proj_combined_initial1, _) = match machine_core::project(
                get_interfacing_swarms_2(),
                SubscriptionsWrapped(subs1.clone()),
                role.clone(),
                false,
                false,
            ) {
                DataResult::OK { data } => crate::machine::from_json(data),
                DataResult::ERROR { errors } => {
                    println!("{}", errors.join(", "));
                    panic!()
                }
            };
            let (proj_combined2, proj_combined_initial2, _) = match machine_core::project(
                get_interfacing_swarms_2_reversed(),
                SubscriptionsWrapped(subs2.clone()),
                role.clone(),
                true,
                false,
            ) {
                DataResult::OK { data } => crate::machine::from_json(data),
                DataResult::ERROR { errors } => {
                    println!("{}", errors.join(", "));
                    panic!()
                }
            };

            // compose(a, b) should be equal to compose(b, a)
            assert!(equivalent(
                &proj_combined1,
                proj_combined_initial1.unwrap(),
                &proj_combined2,
                proj_combined_initial2.unwrap()
            )
            .is_empty());
            let (proj_expanded_proto, proj_expanded_proto_initial, _) = match machine_core::project(
                get_interfacing_swarms_2(),
                SubscriptionsWrapped(subs1.clone()),
                role.clone(),
                true,
                true,
            ) {
                DataResult::OK { data } => crate::machine::from_json(data),
                DataResult::ERROR { errors } => {
                    println!("{}", errors.join(", "));
                    panic!()
                }
            };
            let errors = equivalent(
                &proj_combined2,
                proj_combined_initial2.unwrap(),
                &proj_expanded_proto,
                proj_expanded_proto_initial.unwrap(),
            );

            assert!(errors.is_empty());
        }
    }

    #[test]
    fn test_all_projs_whf() {
        setup_logger();
        let subs = match machine_core::overapproximated_well_formed_sub(
            get_interfacing_swarms_1(),
            SubscriptionsWrapped(BTreeMap::new()),
            Granularity::TwoStep,
        ) {
            DataResult::OK { data } => data,
            DataResult::ERROR { errors } => {
                println!("{}", errors.join(", "));
                panic!()
            }
        };
        let expected_projs = BTreeMap::from([
            (Role::new("T"), get_whf_transport()),
            (Role::new("FL"), get_whf_forklift()),
            (Role::new("D"), get_whf_door()),
            (Role::new("F"), get_whf_f()),
        ]);

        for role in expected_projs.keys() {
            let (proj_expanded_proto, proj_expanded_proto_initial, _) = match machine_core::project(
                get_interfacing_swarms_1(),
                SubscriptionsWrapped(subs.clone()),
                role.clone(),
                true,
                true,
            ) {
                DataResult::OK { data } => crate::machine::from_json(data),
                DataResult::ERROR { errors } => {
                    println!("{}", errors.join(", "));
                    panic!()
                }
            };
            let (proj_combined, proj_combined_initial, _) = match machine_core::project(
                get_interfacing_swarms_1(),
                SubscriptionsWrapped(subs.clone()),
                role.clone(),
                false,
                false,
            ) {
                DataResult::OK { data } => crate::machine::from_json(data),
                DataResult::ERROR { errors } => {
                    println!("{}", errors.join(", "));
                    panic!()
                }
            };
            assert!(equivalent(
                &proj_expanded_proto,
                proj_expanded_proto_initial.unwrap(),
                &proj_combined,
                proj_combined_initial.unwrap()
            )
            .is_empty());

            let (expected, expected_initial, _) =
                crate::machine::from_json(expected_projs.get(role).unwrap().clone());

            assert!(equivalent(
                &expected,
                expected_initial.unwrap(),
                &proj_combined,
                proj_combined_initial.unwrap()
            )
            .is_empty());
        }
    }
}
