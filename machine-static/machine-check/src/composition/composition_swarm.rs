use machine_types::errors::composition_errors::{Error, ErrorReport};
use machine_types::types::{
    typescript_types::{EventType, Role, Subscriptions, EventLabel, InterfacingProtocols,},
    proto_info::{ProtoStruct, ProtoInfo, UnordEventPair, unord_event_pair}
};
use machine_types::types::{proto_graph, proto_info};
use petgraph::{
    visit::{Dfs, EdgeRef, Walker},
    Direction::{Incoming, Outgoing},
};
use std::collections::BTreeSet;

// Retrieve a graph or return an error.
macro_rules! get_ith_or_error {
    ($proto_info:expr, $proto_pointer:expr) => {
        match $proto_info.get_ith_proto($proto_pointer) {
            Some(ProtoStruct {
                graph: g,
                initial: Some(i),
                errors: e,
                roles: _,
            }) => (g, i, e),
            Some(ProtoStruct {
                graph: _,
                initial: None,
                errors: e,
                roles: _,
            }) => return e,
            None => return vec![Error::InvalidArg],
        }
    };
}

// Well-formedness check
pub fn check(protos: InterfacingProtocols, subs: &Subscriptions) -> ErrorReport {
    let _span = tracing::info_span!("check").entered();
    let combined_proto_info = proto_info::swarms_to_proto_info(protos);
    if !combined_proto_info.no_errors() {
        return proto_info::proto_info_to_error_report(combined_proto_info);
    }

    // If we reach this point the protocols can interface and are all confusion free.
    // We construct a ProtoInfo with the composition as the only protocol and all the
    // information about branches etc. from combined_proto_info
    // and the succeeding_events field updated using the expanded composition.
    let composition = proto_info::explicit_composition_proto_info(combined_proto_info);
    let composition_checked = well_formed_proto_info(composition, subs);

    proto_info::proto_info_to_error_report(composition_checked)
}

// Perform wf checks on every protocol in a ProtoInfo.
// Does not check confusion-freeness.
fn well_formed_proto_info(proto_info: ProtoInfo, subs: &Subscriptions) -> ProtoInfo {
    let _span = tracing::info_span!("well_formed_proto_info").entered();
    let protocols: Vec<_> = proto_info
        .protocols
        .clone()
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            let errors = vec![p.errors, well_formed(&proto_info, i, subs)].concat();
            ProtoStruct { errors, ..p }
        })
        .collect();

    ProtoInfo {
        protocols,
        ..proto_info
    }
}

/*
 * Check well-formedness of protocol at index proto_pointer in proto_info w.r.t. subs.
 * A graph that was constructed with prepare_graph with no errors will have one event type per command.
 * Similarly, such a graph will be confusion free, which means we do not have to check for
 * command and log determinism like we do in swarm::well_formed.
 *
 * Does not check confusion freeness.
 */
fn well_formed(
    proto_info: &ProtoInfo,
    proto_pointer: usize,
    subs: &Subscriptions,
) -> Vec<Error> {
    let _span = tracing::info_span!("well_formed").entered();
    let mut errors = Vec::new();
    let empty = BTreeSet::new();
    let sub = |r: &Role| subs.get(r).unwrap_or(&empty);
    let (graph, initial, _) = get_ith_or_error!(proto_info, proto_pointer);

    // Visit all transitions in protocol and perform causal consistency and determinacy checks.
    for node in Dfs::new(&graph, initial).iter(&graph) {
        for edge in graph.edges_directed(node, Outgoing) {
            let event_type = edge.weight().get_event_type();

            // Causal consistency
            // Check if role subscribes to own emitted event.
            if !sub(&edge.weight().role).contains(&event_type) {
                errors.push(Error::SwarmError(
                    machine_types::errors::swarm_errors::Error::ActiveRoleNotSubscribed(edge.id()),
                ));
            }

            // Causal consistency
            // Check if roles with an enabled command in direct successor subscribe to event_type.
            // Active transitions_not_conc gets the transitions going out of edge.target()
            // and filters out the ones emitting events concurrent with event type of 'edge'.
            for successor in proto_graph::active_transitions_not_conc(
                edge.target(),
                &graph,
                &event_type,
                &proto_info.concurrent_events,
            ) {
                if !sub(&successor.role).contains(&event_type) {
                    errors.push(Error::SwarmError(
                        machine_types::errors::swarm_errors::Error::LaterActiveRoleNotSubscribed(
                            edge.id(),
                            successor.role,
                        ),
                    ));
                }
            }

            // Roles subscribing to event types emitted later in the protocol.
            let involved_roles = proto_info::roles_on_path(event_type.clone(), &proto_info, subs);

            // Determinacy.
            // Corresponds to branching rule of determinacy.
            // For some event type t, different protocols could have different sets event types branching with t.
            // Flattening here is okay because we check the event types that actually go out of the node.
            // Could you something go wrong here because of flattening?
            let branching_with_event_type: BTreeSet<_> = proto_info
                .branching_events
                .iter()
                .filter(|set| set.contains(&event_type))
                .flatten()
                .cloned()
                .collect();

            // The event types emitted at this node in the set of event types branching together with event_type.
            let branching_this_node: BTreeSet<_> = graph
                .edges_directed(node, Outgoing)
                .map(|e| e.weight().get_event_type())
                .filter(|t| branching_with_event_type.contains(t))
                .collect();

            // If only one event labeled as branching at this node, do not count it as an error if not subbed.
            // Could happen due loss of behavior.
            if branching_this_node.len() > 1 {
                // Find all, if any, roles that subscribe to event types emitted later in the protocol that do not subscribe to branches and accumulate errors.
                let involved_not_subbed = involved_roles
                    .iter()
                    .filter(|r| !branching_this_node.is_subset(&sub(&r)));
                let mut branching_errors: Vec<_> = involved_not_subbed
                    .map(|r| {
                        (
                            r,
                            branching_this_node
                                .difference(&sub(&r))
                                .cloned()
                                .collect::<Vec<EventType>>(),
                        )
                    })
                    .map(|(r, event_types)| {
                        Error::RoleNotSubscribedToBranch(event_types, edge.id(), node, r.clone())
                    })
                    .collect();
                errors.append(&mut branching_errors);
            }

            // Determinacy.
            // Corresponds to joining rule of determinacy.
            if proto_info.interfacing_events.contains(&event_type) {
                // Find pairs of concurrent event types that are both emitted immediately before event_type (i.e. not concurrent with event_type).
                // Inspect graph to find the immediately preceding -- exact analysis.
                let incoming_pairs_concurrent: Vec<UnordEventPair> =
                    proto_graph::event_pairs_from_node(node, &graph, Incoming)
                        .into_iter()
                        .filter(|pair| proto_info.concurrent_events.contains(pair))
                        .filter(|pair| {
                            pair.iter().all(|e| {
                                !proto_info
                                    .concurrent_events
                                    .contains(&unord_event_pair(e.clone(), event_type.clone()))
                            })
                        })
                        .collect();

                // Flatten events identified above and add event type. If no pairs join_set will be empty. Event type chained multiple times, but ok.
                let join_set: BTreeSet<EventType> = incoming_pairs_concurrent
                    .into_iter()
                    .flat_map(|pair| pair.into_iter().chain([event_type.clone()]))
                    .collect();

                // Find all, if any, roles that subscribe to event types emitted later in the protocol that do not subscribe to joins and prejoins and accumulate errors.
                let involved_not_subbed = involved_roles
                    .iter()
                    .filter(|r| !join_set.is_subset(sub(r)));
                let mut joining_errors: Vec<_> = involved_not_subbed
                    .map(|r| {
                        (
                            r,
                            join_set
                                .difference(&sub(r))
                                .cloned()
                                .collect::<Vec<EventType>>(),
                        )
                    })
                    .map(|(r, event_types)| {
                        Error::RoleNotSubscribedToJoin(event_types.clone(), edge.id(), r.clone())
                    })
                    .collect();
                errors.append(&mut joining_errors);
            }

            // Determinacy.
            // Corresponds to the looping rule of determinacy.
            if proto_info.infinitely_looping_events.contains(&event_type) {
                let t_and_after_t: BTreeSet<EventType> = [event_type.clone()]
                    .into_iter()
                    .chain(
                        proto_info
                            .succeeding_events
                        .get(&event_type)
                        .cloned()
                        .unwrap_or_else(|| BTreeSet::new()),
                )
                .collect();

                let involved_roles = proto_info::roles_on_path(event_type.clone(), &proto_info, subs);
                if !all_roles_sub_to_same(t_and_after_t, &involved_roles, subs) {
                    errors.push(Error::LoopingError(edge.id(), involved_roles.clone().into_iter().collect()));
                }
            }
        }
    }

    // We do not check looping errors since we only accept terminating protocols.
    errors
}

// True if there exists an event type in event_types such that all roles in involved_roles subscribe to it.
// Consider importing the one from machine-types.
fn all_roles_sub_to_same(
    event_types: BTreeSet<EventType>,
    involved_roles: &BTreeSet<Role>,
    subs: &Subscriptions,
) -> bool {
    let _span = tracing::info_span!("all_roles_sub_to_same").entered();
    let empty = BTreeSet::new();
    event_types
        .into_iter()
        .any(|t_|  involved_roles
            .iter()
            .all(|r| subs.get(r).unwrap_or(&empty).contains(&t_)))
}

/* pub fn from_json(proto: SwarmProtocolType) -> (Graph, Option<NodeId>, Vec<String>) {
    let _span = tracing::info_span!("from_json").entered();
    let proto_info = prepare_proto_info(proto);
    let (g, i, e) = match proto_info.get_ith_proto(0) {
        Some(ProtoStruct {
            graph: g,
            initial: i,
            errors: e,
            roles: _,
        }) => (g, i, e),
        _ => return (Graph::new(), None, vec![]),
    };
    let e = e.map(Error::convert(&g));
    (g, i, e)
} */

#[cfg(test)]
mod tests {
    use machine_types::errors::composition_errors;
    use machine_types::types::typescript_types::SwarmProtocolType;

    use super::*;
    use tracing_subscriber::{fmt, fmt::format::FmtSpan, EnvFilter};
    fn setup_logger() {
        fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
            .try_init()
            .ok();
    }

    // Example from coplaws slides
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
    fn get_subs1() -> Subscriptions {
        serde_json::from_str::<Subscriptions>(
            r#"{
                "T": ["partID", "part", "pos", "time"],
                "FL": ["partID", "pos", "time"],
                "D": ["partID", "part", "time"]
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
    fn get_subs2() -> Subscriptions {
        serde_json::from_str::<Subscriptions>(
            r#"{
                "T": ["partID", "part"],
                "F": ["part", "car"]
            }"#,
        )
        .unwrap()
    }
    fn get_proto3() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "observe", "logType": ["report1"], "role": "TR" } },
                    { "source": "1", "target": "2", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                    { "source": "2", "target": "3", "label": { "cmd": "test", "logType": ["report2"], "role": "TR" } },
                    { "source": "3", "target": "4", "label": { "cmd": "accept", "logType": ["ok"], "role": "QCR" } },
                    { "source": "3", "target": "4", "label": { "cmd": "reject", "logType": ["notOk"], "role": "QCR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_subs3() -> Subscriptions {
        serde_json::from_str::<Subscriptions>(
            r#"{
                "F": ["car", "report1"],
                "TR": ["car", "report1", "report2"],
                "QCR": ["report2", "ok", "notOk"]
            }"#,
        )
        .unwrap()
    }
    fn get_proto31() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "observe1", "logType": ["report1"], "role": "QCR" } },
                    { "source": "1", "target": "2", "label": { "cmd": "observe2", "logType": ["report2"], "role": "QCR" } },
                    { "source": "2", "target": "3", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                    { "source": "3", "target": "4", "label": { "cmd": "assess", "logType": ["report3"], "role": "QCR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_proto32() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "observe", "logType": ["observing"], "role": "QCR" } },
                    { "source": "1", "target": "2", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                    { "source": "2", "target": "3", "label": { "cmd": "test", "logType": ["report"], "role": "QCR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_proto_4() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_ir_0", "logType": ["e_ir_0"], "role": "IR" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_ir_1", "logType": ["e_ir_1"], "role": "IR" } },
                    { "source": "2", "target": "1", "label": { "cmd": "c_r0_0", "logType": ["e_r0_0"], "role": "R0" } },
                    { "source": "1", "target": "3", "label": { "cmd": "c_r0_1", "logType": ["e_r0_1"], "role": "R0" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_proto_5() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_ir_0", "logType": ["e_ir_0"], "role": "IR" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_r1_0", "logType": ["e_r1_0"], "role": "R1" } },
                    { "source": "2", "target": "3", "label": { "cmd": "c_ir_1", "logType": ["e_ir_1"], "role": "IR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_subs_composition_1() -> Subscriptions {
        serde_json::from_str::<Subscriptions>(
            r#"{
                "T": ["partID", "part", "pos", "time"],
                "FL": ["partID", "pos", "time"],
                "D": ["partID", "part", "time"],
                "F": ["partID", "part", "car", "time"]
            }"#,
        )
        .unwrap()
    }

    // two event types in close, request appears multiple times, get emits no events
    fn get_malformed_proto1() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "1", "target": "2", "label": { "cmd": "get", "logType": [], "role": "FL" } },
                    { "source": "2", "target": "0", "label": { "cmd": "request", "logType": ["part"], "role": "T" } },
                    { "source": "0", "target": "0", "label": { "cmd": "close", "logType": ["time", "time2"], "role": "D" } }
                ]
            }"#,
        )
        .unwrap()
    }

    // initial state state unreachable
    fn get_malformed_proto2() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "1", "target": "2", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                    { "source": "2", "target": "3", "label": { "cmd": "deliver", "logType": ["partID"], "role": "T" } }
                ]
            }"#,
        )
        .unwrap()
    }

    // all states not reachable
    fn get_malformed_proto3() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "2", "target": "3", "label": { "cmd": "deliver", "logType": ["part"], "role": "T" } },
                    { "source": "4", "target": "5", "label": { "cmd": "build", "logType": ["car"], "role": "F" } }
                ]
            }"#,
        )
        .unwrap()
    }

    // pos event type associated with multiple commands and nondeterminism at 0.
    // No terminal state can be reached from any state -- OK according to confusion freeness
    fn get_confusionful_proto1() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "0", "target": "0", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "1", "target": "2", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                    { "source": "2", "target": "0", "label": { "cmd": "request", "logType": ["pos"], "role": "T" } },
                    { "source": "0", "target": "0", "label": { "cmd": "close", "logType": ["time"], "role": "D" } }
                ]
            }"#,
        )
        .unwrap()
    }
    // No terminal state can be reached from any state -- OK according to confusion freeness
    fn get_some_nonterminating_proto() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "a", "logType": ["a"], "role": "a" } },
                    { "source": "0", "target": "2", "label": { "cmd": "c", "logType": ["c"], "role": "c" } },
                    { "source": "2", "target": "3", "label": { "cmd": "b", "logType": ["b"], "role": "b" } },
                    { "source": "1", "target": "4", "label": { "cmd": "d", "logType": ["d"], "role": "d" } },
                    { "source": "4", "target": "5", "label": { "cmd": "e", "logType": ["e"], "role": "e" } },
                    { "source": "5", "target": "1", "label": { "cmd": "f", "logType": ["f"], "role": "f" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_fail_1_component_1() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"
            {
                "initial": "456",
                "transitions": [
                    {
                    "label": {
                        "cmd": "R453_cmd_0",
                        "logType": [
                        "R453_e_0"
                        ],
                        "role": "R453"
                    },
                    "source": "456",
                    "target": "457"
                    },
                    {
                    "label": {
                        "cmd": "R454_cmd_0",
                        "logType": [
                        "R454_e_0"
                        ],
                        "role": "R454"
                    },
                    "source": "457",
                    "target": "458"
                    }
                ]
                }
            "#,
        )
        .unwrap()
    }

    fn get_fail_1_component_2() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"
            {
                "initial": "459",
                "transitions": [
                    {
                    "label": {
                        "cmd": "R455_cmd_0",
                        "logType": [
                        "R455_e_0"
                        ],
                        "role": "R455"
                    },
                    "source": "459",
                    "target": "460"
                    },
                    {
                    "label": {
                        "cmd": "R455_cmd_1",
                        "logType": [
                        "R455_e_1"
                        ],
                        "role": "R455"
                    },
                    "source": "460",
                    "target": "459"
                    },
                    {
                    "label": {
                        "cmd": "R454_cmd_0",
                        "logType": [
                        "R454_e_0"
                        ],
                        "role": "R454"
                    },
                    "source": "459",
                    "target": "461"
                    }
                ]
            }
            "#,
        )
        .unwrap()
    }

    fn pattern_4_proto_0() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_r0", "logType": ["e_r0"], "role": "R0" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_ir", "logType": ["e_ir"], "role": "IR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn pattern_4_proto_1() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_r1", "logType": ["e_r1"], "role": "R1" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_ir", "logType": ["e_ir"], "role": "IR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn pattern_4_proto_2() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_r2", "logType": ["e_r2"], "role": "R2" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_ir", "logType": ["e_ir"], "role": "IR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn pattern_4_proto_3() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_r3", "logType": ["e_r3"], "role": "R3" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_ir", "logType": ["e_ir"], "role": "IR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn pattern_4_proto_4() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_r4", "logType": ["e_r4"], "role": "R4" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_ir", "logType": ["e_ir"], "role": "IR" } }
                ]
            }"#,
        )
        .unwrap()
    }

    fn ref_pat_proto_0() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_ir0_0", "logType": ["e_ir0_0"], "role": "IR0" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_ir0_1", "logType": ["e_ir0_1"], "role": "IR0" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn ref_pat_proto_1() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_ir0_0", "logType": ["e_ir0_0"], "role": "IR0" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_ir1_0", "logType": ["e_ir1_0"], "role": "IR1" } },
                    { "source": "2", "target": "3", "label": { "cmd": "c_ir1_1", "logType": ["e_ir1_1"], "role": "IR1" } },
                    { "source": "3", "target": "4", "label": { "cmd": "c_rb", "logType": ["e_rb"], "role": "RB" } },
                    { "source": "4", "target": "5", "label": { "cmd": "c_ir0_1", "logType": ["e_ir0_1"], "role": "IR0" } },
                    { "source": "1", "target": "6", "label": { "cmd": "c_ra", "logType": ["e_ra"], "role": "RA" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn ref_pat_proto_2() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_ir1_0", "logType": ["e_ir1_0"], "role": "IR1" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_rc", "logType": ["e_rc"], "role": "RC" } },
                    { "source": "2", "target": "3", "label": { "cmd": "c_ir1_1", "logType": ["e_ir1_1"], "role": "IR1" } }
                ]
            }"#,
        )
        .unwrap()
    }

    fn get_interfacing_swarms_5() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto_4(), get_proto_5()])
    }

    fn get_ref_pat_protos() -> InterfacingProtocols {
        InterfacingProtocols(vec![
            ref_pat_proto_0(),
            ref_pat_proto_1(),
            ref_pat_proto_2(),
        ])
    }

    fn get_interfacing_swarms_1() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto1(), get_proto2()])
    }

    fn get_interfacing_swarms_2() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto1(), get_proto2(), get_proto3()])
    }

    fn get_interfacing_swarms_3() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto1(), get_proto2(), get_proto31()])
    }

    fn get_interfacing_swarms_4() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto1(), get_proto2(), get_proto32()])
    }

    fn get_interfacing_swarms_pat_4() -> InterfacingProtocols {
        InterfacingProtocols(vec![
            pattern_4_proto_0(),
            pattern_4_proto_1(),
            pattern_4_proto_2(),
            pattern_4_proto_3(),
            pattern_4_proto_4(),
        ])
    }
    fn get_fail_1_swarms() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_fail_1_component_1(), get_fail_1_component_2()])
    }

    // QCR subscribes to car and part because report1 is concurrent with part and they lead to a joining event car/event is joining bc of this.
    fn get_subs_composition_2() -> Subscriptions {
        serde_json::from_str::<Subscriptions>(
            r#"{
                "T": ["partID", "part", "pos", "time"],
                "FL": ["partID", "pos", "time"],
                "D": ["partID", "part", "time"],
                "F": ["partID", "part", "car", "time", "report1"],
                "TR": ["partID", "report1", "report2", "car", "time", "part"],
                "QCR": ["partID", "part", "report1", "report2", "car", "time", "ok", "notOk"]
            }"#,
        )
        .unwrap()
    }

    // true if subs1 is a subset of subs2
    fn is_sub_subscription(subs1: Subscriptions, subs2: Subscriptions) -> bool {
        if !subs1
            .keys()
            .cloned()
            .collect::<BTreeSet<Role>>()
            .is_subset(&subs2.keys().cloned().collect::<BTreeSet<Role>>())
        {
            return false;
        }

        for role in subs1.keys() {
            if !subs1[role].is_subset(&subs2[role]) {
                return false;
            }
        }

        true
    }


    mod well_formedness_check_tests {
        use std::collections::BTreeMap;

        use super::*;
        use machine_types::subscription::exact;
        // Tests relating to well-formedness checking.
        #[test]
        fn test_wf_ok() {
            setup_logger();
            let proto1: InterfacingProtocols = InterfacingProtocols(vec![get_proto1()]);
            let result1 = exact::exact_well_formed_sub(proto1.clone(), &BTreeMap::new());
            assert!(result1.is_ok());
            let subs1 = result1.unwrap();
            let error_report = check(proto1, &subs1);
            assert!(error_report.is_empty());
            assert_eq!(get_subs1(), subs1);

            let proto2: InterfacingProtocols = InterfacingProtocols(vec![get_proto2()]);
            let result2 = exact::exact_well_formed_sub(proto2.clone(), &BTreeMap::new());
            assert!(result2.is_ok());
            let subs2 = result2.unwrap();
            let error_report = check(proto2, &subs2);
            assert!(error_report.is_empty());
            assert_eq!(get_subs2(), subs2);

            let proto3: InterfacingProtocols = InterfacingProtocols(vec![get_proto3()]);
            let result3 = exact::exact_well_formed_sub(proto3.clone(), &BTreeMap::new());
            assert!(result3.is_ok());
            let subs3 = result3.unwrap();
            let error_report = check(proto3, &subs3);
            assert!(error_report.is_empty());
            assert_eq!(get_subs3(), subs3);

            let composition1: InterfacingProtocols = get_interfacing_swarms_1();
            let result_composition1 =
                exact::exact_well_formed_sub(composition1.clone(), &BTreeMap::new());
            assert!(result_composition1.is_ok());
            let subs_composition = result_composition1.unwrap();
            let error_report = check(composition1, &subs_composition);
            assert!(error_report.is_empty());
            assert_eq!(get_subs_composition_1(), subs_composition);

            let composition2: InterfacingProtocols = get_interfacing_swarms_2();
            let result_composition2 =
                exact::exact_well_formed_sub(composition2.clone(), &BTreeMap::new());
            assert!(result_composition2.is_ok());
            let subs_composition = result_composition2.unwrap();
            let error_report = check(composition2, &subs_composition);
            assert!(error_report.is_empty());
            assert_eq!(get_subs_composition_2(), subs_composition);
        }

        #[test]
        fn test_wf_fail() {
            setup_logger();
            let input: InterfacingProtocols = InterfacingProtocols(vec![get_proto1()]);
            let subs = BTreeMap::from([
                (Role::new("T"), BTreeSet::from([EventType::new("pos")])),
                (Role::new("D"), BTreeSet::from([EventType::new("pos")])),
                (Role::new("FL"), BTreeSet::from([EventType::new("partID")])),
            ]);
            let error_report = check(input, &subs);
            let mut errors = composition_errors::error_report_to_strings(error_report);
            errors.sort();
            let mut expected_errors = vec![
                "active role does not subscribe to any of its emitted event types in transition (0)--[close@D<time>]-->(3)",
                "active role does not subscribe to any of its emitted event types in transition (0)--[request@T<partID>]-->(1)",
                "active role does not subscribe to any of its emitted event types in transition (2)--[deliver@T<part>]-->(0)",
                "active role does not subscribe to any of its emitted event types in transition (1)--[get@FL<pos>]-->(2)",
                "role T does not subscribe to event types partID, time in branching transitions at state 0, but is involved after transition (0)--[request@T<partID>]-->(1)",
                "role D does not subscribe to event types partID, time in branching transitions at state 0, but is involved after transition (0)--[request@T<partID>]-->(1)",
                "role FL does not subscribe to event types time in branching transitions at state 0, but is involved after transition (0)--[request@T<partID>]-->(1)",
                "subsequently active role D does not subscribe to events in transition (2)--[deliver@T<part>]-->(0)",
                "subsequently active role T does not subscribe to events in transition (2)--[deliver@T<part>]-->(0)",
            ];

            expected_errors.sort();
            assert_eq!(errors, expected_errors);

            let input: InterfacingProtocols = InterfacingProtocols(vec![get_proto2()]);
            let error_report = check(input, &get_subs3());
            let mut errors = composition_errors::error_report_to_strings(error_report);
            errors.sort();
            let mut expected_errors = vec![
                "active role does not subscribe to any of its emitted event types in transition (0)--[request@T<partID>]-->(1)",
                "subsequently active role T does not subscribe to events in transition (0)--[request@T<partID>]-->(1)",
                "active role does not subscribe to any of its emitted event types in transition (1)--[deliver@T<part>]-->(2)",
                "subsequently active role F does not subscribe to events in transition (1)--[deliver@T<part>]-->(2)"
            ];

            expected_errors.sort();
            assert_eq!(errors, expected_errors);

            let input: InterfacingProtocols = InterfacingProtocols(vec![get_proto3()]);

            let error_report = check(input, &get_subs1());
            let mut errors = composition_errors::error_report_to_strings(error_report);
            errors.sort();
            let mut expected_errors = vec![
                "active role does not subscribe to any of its emitted event types in transition (0)--[observe@TR<report1>]-->(1)",
                "active role does not subscribe to any of its emitted event types in transition (1)--[build@F<car>]-->(2)",
                "active role does not subscribe to any of its emitted event types in transition (2)--[test@TR<report2>]-->(3)",
                "active role does not subscribe to any of its emitted event types in transition (3)--[accept@QCR<ok>]-->(4)",
                "active role does not subscribe to any of its emitted event types in transition (3)--[reject@QCR<notOk>]-->(4)",
                "subsequently active role F does not subscribe to events in transition (0)--[observe@TR<report1>]-->(1)",
                "subsequently active role QCR does not subscribe to events in transition (2)--[test@TR<report2>]-->(3)",
                "subsequently active role QCR does not subscribe to events in transition (2)--[test@TR<report2>]-->(3)",
                "subsequently active role TR does not subscribe to events in transition (1)--[build@F<car>]-->(2)"
            ];

            expected_errors.sort();
            assert_eq!(errors, expected_errors);
        }

        #[test]
        fn test_compose_non_wf_swarms() {
            setup_logger();
            let input = get_interfacing_swarms_1();
            let subs = BTreeMap::from([
                (Role::new("T"), BTreeSet::from([EventType::new("part")])),
                (Role::new("D"), BTreeSet::from([EventType::new("part")])),
                (Role::new("FL"), BTreeSet::from([EventType::new("part")])),
                (Role::new("F"), BTreeSet::from([EventType::new("part")])),
            ]);
            let error_report = check(input, &subs);
            let mut errors = composition_errors::error_report_to_strings(error_report);
            errors.sort();
            let mut expected_errors = vec![
                "active role does not subscribe to any of its emitted event types in transition (0 || 0)--[request@T<partID>]-->(1 || 1)",
                "active role does not subscribe to any of its emitted event types in transition (0 || 0)--[close@D<time>]-->(3 || 0)",
                "active role does not subscribe to any of its emitted event types in transition (1 || 1)--[get@FL<pos>]-->(2 || 1)",
                "active role does not subscribe to any of its emitted event types in transition (0 || 2)--[build@F<car>]-->(0 || 3)",
                "active role does not subscribe to any of its emitted event types in transition (0 || 3)--[close@D<time>]-->(3 || 3)",
                "active role does not subscribe to any of its emitted event types in transition (0 || 2)--[close@D<time>]-->(3 || 2)",
                "active role does not subscribe to any of its emitted event types in transition (3 || 2)--[build@F<car>]-->(3 || 3)",
                "role D does not subscribe to event types partID, time in branching transitions at state 0 || 0, but is involved after transition (0 || 0)--[request@T<partID>]-->(1 || 1)",
                "role T does not subscribe to event types partID, time in branching transitions at state 0 || 0, but is involved after transition (0 || 0)--[request@T<partID>]-->(1 || 1)",
                "role FL does not subscribe to event types partID, time in branching transitions at state 0 || 0, but is involved after transition (0 || 0)--[request@T<partID>]-->(1 || 1)",
                "role F does not subscribe to event types partID, time in branching transitions at state 0 || 0, but is involved after transition (0 || 0)--[request@T<partID>]-->(1 || 1)",
                "subsequently active role FL does not subscribe to events in transition (0 || 0)--[request@T<partID>]-->(1 || 1)",
                "subsequently active role T does not subscribe to events in transition (1 || 1)--[get@FL<pos>]-->(2 || 1)",
            ];
            expected_errors.sort();
            assert_eq!(errors, expected_errors);
        }

        #[test]
        fn test_fail1() {
            setup_logger();
            let result = exact::exact_well_formed_sub(get_fail_1_swarms(), &BTreeMap::new());
            assert!(result.is_ok());
            let subs1 = result.unwrap();
            let error_report = check(get_fail_1_swarms(), &subs1);
            assert!(error_report.is_empty());

            let error_report = check(get_fail_1_swarms(), &BTreeMap::new());
            let mut errors = composition_errors::error_report_to_strings(error_report);
            errors.sort();
            let mut expected_errors = vec![
                "active role does not subscribe to any of its emitted event types in transition (456 || 459)--[R455_cmd_0@R455<R455_e_0>]-->(456 || 460)",
                "subsequently active role R455 does not subscribe to events in transition (456 || 459)--[R455_cmd_0@R455<R455_e_0>]-->(456 || 460)",
                "active role does not subscribe to any of its emitted event types in transition (456 || 459)--[R453_cmd_0@R453<R453_e_0>]-->(457 || 459)",
                "subsequently active role R454 does not subscribe to events in transition (456 || 459)--[R453_cmd_0@R453<R453_e_0>]-->(457 || 459)",
                "active role does not subscribe to any of its emitted event types in transition (457 || 459)--[R454_cmd_0@R454<R454_e_0>]-->(458 || 461)",
                "active role does not subscribe to any of its emitted event types in transition (457 || 459)--[R455_cmd_0@R455<R455_e_0>]-->(457 || 460)",
                "subsequently active role R455 does not subscribe to events in transition (457 || 459)--[R455_cmd_0@R455<R455_e_0>]-->(457 || 460)",
                "active role does not subscribe to any of its emitted event types in transition (457 || 460)--[R455_cmd_1@R455<R455_e_1>]-->(457 || 459)",
                "subsequently active role R454 does not subscribe to events in transition (457 || 460)--[R455_cmd_1@R455<R455_e_1>]-->(457 || 459)",
                "subsequently active role R455 does not subscribe to events in transition (457 || 460)--[R455_cmd_1@R455<R455_e_1>]-->(457 || 459)",
                "active role does not subscribe to any of its emitted event types in transition (456 || 460)--[R455_cmd_1@R455<R455_e_1>]-->(456 || 459)",
                "subsequently active role R455 does not subscribe to events in transition (456 || 460)--[R455_cmd_1@R455<R455_e_1>]-->(456 || 459)",
                "active role does not subscribe to any of its emitted event types in transition (456 || 460)--[R453_cmd_0@R453<R453_e_0>]-->(457 || 460)"
            ];
            expected_errors.sort();
            assert_eq!(errors, expected_errors);
        }

        #[test]
        fn test_join_errors() {
            setup_logger();
            let composition: InterfacingProtocols = get_interfacing_swarms_2();
            let result_composition = exact::exact_well_formed_sub(composition.clone(), &BTreeMap::new());
            assert!(result_composition.is_ok());
            let mut subs_composition = result_composition.unwrap();
            subs_composition.entry(Role::new("QCR")).and_modify(|s| {
                *s = BTreeSet::from([
                    EventType::new("report2"),
                    EventType::new("ok"),
                    EventType::new("notOk"),
                    EventType::new("partID"),
                    EventType::new("time"),
                ])
            });
            subs_composition.entry(Role::new("F")).and_modify(|s| {
                s.remove(&EventType::new("report1"));
            });
            let error_report = check(composition.clone(), &subs_composition);
            let mut errors = composition_errors::error_report_to_strings(error_report);
            let mut expected_errors = vec![
                "role F does not subscribe to event types report1 leading to or in joining event in transition (0 || 2 || 1)--[build@F<car>]-->(0 || 3 || 2)",
                "subsequently active role F does not subscribe to events in transition (0 || 2 || 0)--[observe@TR<report1>]-->(0 || 2 || 1)",
                "subsequently active role F does not subscribe to events in transition (3 || 2 || 0)--[observe@TR<report1>]-->(3 || 2 || 1)",
                "role QCR does not subscribe to event types car, part, report1 leading to or in joining event in transition (0 || 2 || 1)--[build@F<car>]-->(0 || 3 || 2)"];
            errors.sort();
            expected_errors.sort();
            assert_eq!(errors, expected_errors);
        }

        #[test]
        fn inference_example_1() {
            fn subs() -> Subscriptions {
                serde_json::from_str::<Subscriptions>(
                    r#"{
                    "R1": ["a1", "a2", "b1"],
                    "R2": ["b1", "b2", "a1"],
                    "R3": ["c1", "c2"]
                }"#,
                )
                .unwrap()
            }
            fn proto1() -> SwarmProtocolType {
                serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        { "source": "0", "target": "1", "label": { "cmd": "cmd_a1", "logType": ["a1"], "role": "R1" } },
                        { "source": "1", "target": "3", "label": { "cmd": "cmd_a2", "logType": ["a2"], "role": "R1" } },
                        { "source": "0", "target": "2", "label": { "cmd": "cmd_b1", "logType": ["b1"], "role": "R2" } },
                        { "source": "2", "target": "4", "label": { "cmd": "cmd_b2", "logType": ["b2"], "role": "R2" } }
                    ]
                }"#,
            )
            .unwrap()
            }
            fn proto2() -> SwarmProtocolType {
                serde_json::from_str::<SwarmProtocolType>(
                    r#"{
                        "initial": "0",
                        "transitions": [
                            { "source": "0", "target": "1", "label": { "cmd": "cmd_c1", "logType": ["c1"], "role": "R3" } },
                            { "source": "1", "target": "2", "label": { "cmd": "cmd_c2", "logType": ["c2"], "role": "R3" } }
                        ]
                    }"#,
                )
                .unwrap()
            }
            fn as_interfacing_protocols() -> InterfacingProtocols {
                InterfacingProtocols(vec![proto1(), proto2()])
            }

            assert!(check(as_interfacing_protocols(), &subs()).is_empty());
            let smalles_sub = exact::exact_well_formed_sub(as_interfacing_protocols(), &BTreeMap::new());
            assert!(smalles_sub.is_ok());
            assert_eq!(smalles_sub.unwrap(), subs());
        }

        #[test]
        fn inference_example_2() {
            fn subs() -> Subscriptions {
                serde_json::from_str::<Subscriptions>(
                    r#"{
                    "R1": ["a1", "a2", "b1"],
                    "R2": ["b1", "b2", "a1"],
                    "R3": ["c1", "c2"],
                    "IR": ["i1", "a1", "a2", "b1", "c2"]
                }"#,
                )
                .unwrap()
            }
            fn proto1() -> SwarmProtocolType {
                serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        { "source": "0", "target": "1", "label": { "cmd": "cmd_a1", "logType": ["a1"], "role": "R1" } },
                        { "source": "1", "target": "3", "label": { "cmd": "cmd_a2", "logType": ["a2"], "role": "R1" } },
                        { "source": "3", "target": "5", "label": { "cmd": "cmd_i1", "logType": ["i1"], "role": "IR" } },
                        { "source": "0", "target": "2", "label": { "cmd": "cmd_b1", "logType": ["b1"], "role": "R2" } },
                        { "source": "2", "target": "4", "label": { "cmd": "cmd_b2", "logType": ["b2"], "role": "R2" } }
                    ]
                }"#,
            )
            .unwrap()
            }
            fn proto2() -> SwarmProtocolType {
                serde_json::from_str::<SwarmProtocolType>(
                    r#"{
                        "initial": "0",
                        "transitions": [
                            { "source": "0", "target": "1", "label": { "cmd": "cmd_c1", "logType": ["c1"], "role": "R3" } },
                            { "source": "1", "target": "2", "label": { "cmd": "cmd_c2", "logType": ["c2"], "role": "R3" } },
                            { "source": "2", "target": "3", "label": { "cmd": "cmd_i1", "logType": ["i1"], "role": "IR" } }
                        ]
                    }"#,
                )
                .unwrap()
            }
            fn as_interfacing_protocols() -> InterfacingProtocols {
                InterfacingProtocols(vec![proto1(), proto2()])
            }
            assert!(check(as_interfacing_protocols(), &subs()).is_empty());
            let smalles_sub = exact::exact_well_formed_sub(as_interfacing_protocols(), &BTreeMap::new());
            assert!(smalles_sub.is_ok());
            assert_eq!(smalles_sub.unwrap(), subs());
        }

        #[test]
        fn inference_example_3() {
            fn subs() -> Subscriptions {
                serde_json::from_str::<Subscriptions>(
                    r#"{
                    "R1": ["i1", "a1", "a2", "b1"],
                    "R2": ["i1", "b1", "b2", "a1"],
                    "R3": ["i1", "c1", "c2"],
                    "IR": ["i1"]
                }"#,
                )
                .unwrap()
            }
            fn proto1() -> SwarmProtocolType {
                serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        { "source": "0", "target": "1", "label": { "cmd": "cmd_i1", "logType": ["i1"], "role": "IR" } },
                        { "source": "1", "target": "2", "label": { "cmd": "cmd_a1", "logType": ["a1"], "role": "R1" } },
                        { "source": "2", "target": "4", "label": { "cmd": "cmd_a2", "logType": ["a2"], "role": "R1" } },
                        { "source": "1", "target": "3", "label": { "cmd": "cmd_b1", "logType": ["b1"], "role": "R2" } },
                        { "source": "3", "target": "5", "label": { "cmd": "cmd_b2", "logType": ["b2"], "role": "R2" } }
                    ]
                }"#,
            )
            .unwrap()
            }
            fn proto2() -> SwarmProtocolType {
                serde_json::from_str::<SwarmProtocolType>(
                    r#"{
                        "initial": "0",
                        "transitions": [
                            { "source": "0", "target": "1", "label": { "cmd": "cmd_i1", "logType": ["i1"], "role": "IR" } },
                            { "source": "1", "target": "2", "label": { "cmd": "cmd_c1", "logType": ["c1"], "role": "R3" } },
                            { "source": "2", "target": "3", "label": { "cmd": "cmd_c2", "logType": ["c2"], "role": "R3" } }
                        ]
                    }"#,
                )
                .unwrap()
            }
            fn as_interfacing_protocols() -> InterfacingProtocols {
                InterfacingProtocols(vec![proto1(), proto2()])
            }

            assert!(check(as_interfacing_protocols(), &subs()).is_empty());
            let smalles_sub = exact::exact_well_formed_sub(as_interfacing_protocols(), &BTreeMap::new());
            assert!(smalles_sub.is_ok());
            assert_eq!(smalles_sub.unwrap(), subs());
        }

        #[test]
        fn inference_example_4() {
            fn subs() -> Subscriptions {
                serde_json::from_str::<Subscriptions>(
                    r#"{
                    "R1": ["c1", "a1"],
                    "R2": ["a1", "b1"],
                    "R3": ["a1", "b1", "c1"]
                }"#,
                )
                .unwrap()
            }
            fn proto1() -> SwarmProtocolType {
                serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        { "source": "0", "target": "1", "label": { "cmd": "cmd_i1", "logType": ["i1"], "role": "IR" } },
                        { "source": "1", "target": "2", "label": { "cmd": "cmd_i2", "logType": ["i2"], "role": "IR" } },
                        { "source": "0", "target": "3", "label": { "cmd": "cmd_a1", "logType": ["a1"], "role": "R1" } },
                        { "source": "3", "target": "4", "label": { "cmd": "cmd_b1", "logType": ["b1"], "role": "R2" } },
                        { "source": "4", "target": "0", "label": { "cmd": "cmd_c1", "logType": ["c1"], "role": "R3" } }
                    ]
                }"#,
            )
            .unwrap()
            }
            fn proto2() -> SwarmProtocolType {
                serde_json::from_str::<SwarmProtocolType>(
                    r#"{
                        "initial": "0",
                        "transitions": [
                            { "source": "0", "target": "1", "label": { "cmd": "cmd_i2", "logType": ["i2"], "role": "IR" } },
                            { "source": "1", "target": "2", "label": { "cmd": "cmd_i1", "logType": ["i1"], "role": "IR" } }
                        ]
                    }"#,
                )
                .unwrap()
            }
            fn as_interfacing_protocols() -> InterfacingProtocols {
                InterfacingProtocols(vec![proto1(), proto2()])
            }

            assert!(check(as_interfacing_protocols(), &subs()).is_empty());
            let smalles_sub = exact::exact_well_formed_sub(as_interfacing_protocols(), &BTreeMap::new());
            assert!(smalles_sub.is_ok());
            assert_eq!(smalles_sub.unwrap(), subs());
        }

        #[test]
        fn inference_example_5() {
            fn subs() -> Subscriptions {
                serde_json::from_str::<Subscriptions>(
                    r#"{
                    "R1": ["a1", "c1"],
                    "R2": ["a1", "b1", "c1"]
                }"#,
                )
                .unwrap()
            }
            fn proto1() -> SwarmProtocolType {
                serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        { "source": "0", "target": "1", "label": { "cmd": "cmd_a1", "logType": ["a1"], "role": "R1" } },
                        { "source": "1", "target": "2", "label": { "cmd": "cmd_b1", "logType": ["b1"], "role": "R2" } },
                        { "source": "0", "target": "3", "label": { "cmd": "cmd_c1", "logType": ["c1"], "role": "R1" } },
                        { "source": "0", "target": "4", "label": { "cmd": "cmd_i1", "logType": ["i1"], "role": "IR" } },
                        { "source": "4", "target": "5", "label": { "cmd": "cmd_i2", "logType": ["i2"], "role": "IR" } }
                    ]
                }"#,
            )
            .unwrap()
            }
            fn proto2() -> SwarmProtocolType {
                serde_json::from_str::<SwarmProtocolType>(
                    r#"{
                        "initial": "0",
                        "transitions": [
                            { "source": "0", "target": "1", "label": { "cmd": "cmd_i2", "logType": ["i2"], "role": "IR" } },
                            { "source": "1", "target": "2", "label": { "cmd": "cmd_i1", "logType": ["i1"], "role": "IR" } }
                        ]
                    }"#,
                )
                .unwrap()
            }
            fn as_interfacing_protocols() -> InterfacingProtocols {
                InterfacingProtocols(vec![proto1(), proto2()])
            }

            assert!(check(as_interfacing_protocols(), &subs()).is_empty());
            let smalles_sub = exact::exact_well_formed_sub(as_interfacing_protocols(), &BTreeMap::new());
            assert!(smalles_sub.is_ok());
            assert_eq!(smalles_sub.unwrap(), subs());
        }
    }

    // This module contains tests for relating to looping event types.
    mod loop_tests {
        use std::collections::BTreeMap;

        use super::*;
        use machine_types::types::typescript_types::Granularity;
        use machine_types::subscription::{exact, overapproximation};

        #[test]
        fn looping_1() {
            setup_logger();
            fn proto1() -> SwarmProtocolType {
                serde_json::from_str::<SwarmProtocolType>(
                    r#"{
                        "initial": "0",
                        "transitions": [
                            { "source": "0", "target": "1", "label": { "cmd": "cmd_a", "logType": ["a"], "role": "R1" } },
                            { "source": "0", "target": "2", "label": { "cmd": "cmd_b", "logType": ["b"], "role": "R2" } },
                            { "source": "2", "target": "3", "label": { "cmd": "cmd_c", "logType": ["c"], "role": "R1" } },
                            { "source": "3", "target": "4", "label": { "cmd": "cmd_d", "logType": ["d"], "role": "R2" } },
                            { "source": "4", "target": "2", "label": { "cmd": "cmd_e", "logType": ["e"], "role": "R1" } }
                        ]
                    }"#,
                )
                .unwrap()
            }
            // Check exact well-formed subscriptions
            let sub =
                exact::exact_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new())
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());

            // Check overapprox well-formed subscriptions
            let sub =
                overapproximation::overapprox_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new(), Granularity::TwoStep)
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());

            let sub =
                overapproximation::overapprox_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new(), Granularity::Fine)
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());
        }

        #[test]
        fn looping_2() {
            setup_logger();
            fn proto1() -> SwarmProtocolType {
                serde_json::from_str::<SwarmProtocolType>(
                    r#"{
                        "initial": "0",
                        "transitions": [
                            { "source": "0", "target": "1", "label": { "cmd": "cmd_a", "logType": ["a"], "role": "R1" } },
                            { "source": "0", "target": "2", "label": { "cmd": "cmd_b", "logType": ["b"], "role": "R2" } },
                            { "source": "2", "target": "3", "label": { "cmd": "cmd_c", "logType": ["c"], "role": "R3" } },
                            { "source": "3", "target": "4", "label": { "cmd": "cmd_d", "logType": ["d"], "role": "R4" } },
                            { "source": "4", "target": "2", "label": { "cmd": "cmd_e", "logType": ["e"], "role": "R5" } }
                        ]
                    }"#,
                )
                .unwrap()
            }

            // Check exact well-formed subscriptions
            let sub =
                exact::exact_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new())
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());

            // Check overapprox well-formed subscriptions
            let sub =
                overapproximation::overapprox_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new(), Granularity::TwoStep)
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());

            let sub =
                overapproximation::overapprox_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new(), Granularity::Fine)
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());
        }

        #[test]
        fn looping_3() {
            setup_logger();
            fn proto1() -> SwarmProtocolType {
                serde_json::from_str::<SwarmProtocolType>(
                    r#"{
                        "initial": "0",
                        "transitions": [
                            { "source": "0", "target": "1", "label": { "cmd": "cmd_a", "logType": ["a"], "role": "R1" } },
                            { "source": "0", "target": "2", "label": { "cmd": "cmd_b", "logType": ["b"], "role": "R2" } },
                            { "source": "2", "target": "3", "label": { "cmd": "cmd_c", "logType": ["c"], "role": "R3" } },
                            { "source": "3", "target": "4", "label": { "cmd": "cmd_d", "logType": ["d"], "role": "R4" } },
                            { "source": "4", "target": "2", "label": { "cmd": "cmd_e", "logType": ["e"], "role": "R5" } },
                            { "source": "1", "target": "5", "label": { "cmd": "cmd_f", "logType": ["f"], "role": "R5" } },
                            { "source": "5", "target": "6", "label": { "cmd": "cmd_g", "logType": ["g"], "role": "R6" } },
                            { "source": "6", "target": "7", "label": { "cmd": "cmd_h", "logType": ["h"], "role": "R6" } },
                            { "source": "7", "target": "1", "label": { "cmd": "cmd_i", "logType": ["i"], "role": "R7" } }
                        ]
                    }"#,
                )
                .unwrap()
            }

            // Check exact well-formed subscriptions
            let sub =
                exact::exact_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new())
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());

            // Check overapprox well-formed subscriptions
            let sub =
                overapproximation::overapprox_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new(), Granularity::TwoStep)
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());

            let sub =
                overapproximation::overapprox_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new(), Granularity::Fine)
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());
        }

        #[test]
        fn looping_4() {
            setup_logger();
            fn proto1() -> SwarmProtocolType {
                serde_json::from_str::<SwarmProtocolType>(
                    r#"{
                        "initial": "0",
                        "transitions": [
                            { "source": "0", "target": "1", "label": { "cmd": "cmd_a", "logType": ["a"], "role": "R1" } },
                            { "source": "1", "target": "2", "label": { "cmd": "cmd_b", "logType": ["b"], "role": "R2" } },
                            { "source": "2", "target": "3", "label": { "cmd": "cmd_c", "logType": ["c"], "role": "R3" } },
                            { "source": "3", "target": "4", "label": { "cmd": "cmd_d", "logType": ["d"], "role": "R4" } },
                            { "source": "4", "target": "5", "label": { "cmd": "cmd_e", "logType": ["e"], "role": "R5" } },
                            { "source": "5", "target": "6", "label": { "cmd": "cmd_f", "logType": ["f"], "role": "R6" } },
                            { "source": "6", "target": "7", "label": { "cmd": "cmd_g", "logType": ["g"], "role": "R7" } },
                            { "source": "7", "target": "2", "label": { "cmd": "cmd_h", "logType": ["h"], "role": "R8" } }
                        ]
                    }"#,
                )
                .unwrap()
            }

            // Check exact well-formed subscriptions
            let sub =
                exact::exact_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new())
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());

            // Check overapprox well-formed subscriptions
            let sub =
                overapproximation::overapprox_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new(), Granularity::TwoStep)
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());

            let sub =
                overapproximation::overapprox_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new(), Granularity::Fine)
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());
        }

        #[test]
        fn looping_5() {
            setup_logger();
            fn proto1() -> SwarmProtocolType {
                serde_json::from_str::<SwarmProtocolType>(
                    r#"{
                        "initial": "0",
                        "transitions": [
                            { "source": "0", "target": "1", "label": { "cmd": "cmd_a", "logType": ["a"], "role": "R1" } },
                            { "source": "1", "target": "2", "label": { "cmd": "cmd_b", "logType": ["b"], "role": "R2" } },
                            { "source": "2", "target": "3", "label": { "cmd": "cmd_c", "logType": ["c"], "role": "R3" } },
                            { "source": "3", "target": "0", "label": { "cmd": "cmd_d", "logType": ["d"], "role": "R4" } },
                            { "source": "0", "target": "4", "label": { "cmd": "cmd_e", "logType": ["e"], "role": "R5" } },
                            { "source": "4", "target": "5", "label": { "cmd": "cmd_f", "logType": ["f"], "role": "R6" } },
                            { "source": "5", "target": "6", "label": { "cmd": "cmd_g", "logType": ["g"], "role": "R7" } },
                            { "source": "6", "target": "0", "label": { "cmd": "cmd_h", "logType": ["h"], "role": "R8" } }
                        ]
                    }"#,
                )
                .unwrap()
            }

            // Check exact well-formed subscriptions
            let sub =
                exact::exact_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new())
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());

            // Check overapprox well-formed subscriptions
            let sub =
                overapproximation::overapprox_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new(), Granularity::TwoStep)
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());

            let sub =
                overapproximation::overapprox_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new(), Granularity::Fine)
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());
        }

        #[test]
        fn looping_6() {
            setup_logger();
            fn proto1() -> SwarmProtocolType {
                serde_json::from_str::<SwarmProtocolType>(
                    r#"{
                        "initial": "0",
                        "transitions": [
                            { "source": "0", "target": "1", "label": { "cmd": "cmd_a", "logType": ["a"], "role": "R1" } },
                            { "source": "1", "target": "0", "label": { "cmd": "cmd_b", "logType": ["b"], "role": "R2" } },
                            { "source": "1", "target": "2", "label": { "cmd": "cmd_c", "logType": ["c"], "role": "R3" } },
                            { "source": "2", "target": "3", "label": { "cmd": "cmd_d", "logType": ["d"], "role": "R4" } },
                            { "source": "3", "target": "4", "label": { "cmd": "cmd_e", "logType": ["e"], "role": "R5" } },
                            { "source": "4", "target": "0", "label": { "cmd": "cmd_f", "logType": ["f"], "role": "R6" } }
                        ]
                    }"#,
                )
                .unwrap()
            }

            // Check exact well-formed subscriptions
            let sub =
                exact::exact_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new())
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());

            // Check overapprox well-formed subscriptions
            let sub =
                overapproximation::overapprox_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new(), Granularity::TwoStep)
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());

            let sub =
                overapproximation::overapprox_well_formed_sub(InterfacingProtocols(vec![proto1()]), &BTreeMap::new(), Granularity::Fine)
                    .unwrap();
            assert!(check(InterfacingProtocols(vec![proto1()]), &sub).is_empty());
        }
    }
}
