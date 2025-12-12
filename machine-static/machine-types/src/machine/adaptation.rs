use std::collections::{BTreeMap, BTreeSet};

use crate::{composition, types::{projection::{Graph, OptionGraph}, proto_graph::NodeId, proto_info::{self, ProtoInfo, UnordEventPair}, typescript_types::{BranchMap, EventLabel, EventType, MachineLabel, ProjToMachineStates, ProjectionInfo, Role, State, StateName, Subscriptions}}};
use crate::machine::{projection, util};
use itertools::Itertools;
use petgraph::{
    visit::{EdgeRef, IntoNodeReferences},
    Direction::Outgoing,
};
// Possibly move to adaptation.
// Used for creating adapted machine.
// A composed state in an adapted machine contains some
// state from the original machine to be adapted.
// The field machine_states points to the state(s).
// A set of states because seems more general, maybe
// we need that in the future.
#[derive(Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Debug)]
struct AdaptationNode {
    state: State,
    machine_states: Option<BTreeSet<State>>,
}
type AdaptationGraph = petgraph::Graph<AdaptationNode, MachineLabel>;

pub fn projection_information(
    proto_info: &ProtoInfo,
    subs: &Subscriptions,
    role: Role,
    machine: (OptionGraph, NodeId),
    k: usize,
    minimize: bool,
) -> Option<ProjectionInfo> {
    let (proj, proj_initial) =
        match adapted_projection(&proto_info, subs, role, machine, k, minimize) {
            Some((proj, Some(proj_initial))) => (proj, proj_initial),
            _ => return None,
        };

    let proj_to_machine_states: ProjToMachineStates = proj
        .node_references()
        .map(|(_, n_ref)| {
            (
                n_ref.state.clone(),
                n_ref.machine_states.clone().unwrap().into_iter().collect(),
            )
        })
        .collect();

    let proj = from_adaptation_graph_to_option_graph(&proj);

    let branches = paths_from_event_types(&proj, &proto_info);
    let special_event_types = proto_info::get_branching_joining_proto_info(&proto_info);

    Some(ProjectionInfo {
        projection: util::from_option_to_machine(proj, proj_initial),
        branches,
        special_event_types,
        proj_to_machine_states,
    })
}

fn adapted_projection(
    proto_info: &ProtoInfo,
    subs: &Subscriptions,
    role: Role,
    machine: (OptionGraph, NodeId),
    k: usize,
    minimize: bool,
) -> Option<(AdaptationGraph, Option<NodeId>)> {
    let _span = tracing::info_span!("adapted_projection", %role).entered();
    if proto_info.protocols.is_empty() || k >= proto_info.protocols.len() {
        return None;
    }

    // project a protocol and turn the projection into an AdaptationGraph
    let mapper = |(proj, proj_initial, interface): (Graph, NodeId, BTreeSet<EventType>)| {
        let proj = proj.map(
            |_, n| AdaptationNode {
                state: n.clone(),
                machine_states: None,
            },
            |_, label| label.clone(),
        );
        (proj, proj_initial, interface)
    };

    let gen_node = |n1: &AdaptationNode, n2: &AdaptationNode| -> AdaptationNode {
        let name = format!("{} || {}", n1.state.state_name(), n2.state.state_name());
        match (n1.machine_states.clone(), n2.machine_states.clone()) {
            (None, None) => AdaptationNode {
                state: State::from(name),
                machine_states: None,
            },
            (Some(ms), None) => AdaptationNode {
                state: State::from(name),
                machine_states: Some(ms),
            },
            (None, Some(ms)) => AdaptationNode {
                state: State::from(name),
                machine_states: Some(ms),
            },
            (Some(ms1), Some(ms2)) => AdaptationNode {
                state: State::from(name),
                machine_states: Some(ms1.intersection(&ms2).cloned().collect()),
            },
        }
    };

    let projections: Vec<(AdaptationGraph, NodeId, BTreeSet<EventType>)> =
        projection::to_chained_projections(projection::to_chained_protos(proto_info), subs, role, minimize)
            .into_iter()
            .map(mapper)
            .collect();

    //AdaptationGraph{state: n.clone(), machine_state: Some(state.clone())}
    let (machine, machine_initial) = (from_option_graph_to_graph(&machine.0), machine.1);
    let machine = machine.map(
        |_, n| AdaptationNode {
            state: n.clone(),
            machine_states: Some(BTreeSet::from([n.clone()])),
        },
        |_, label| label.clone(),
    );
    let machine_proj_intersect = machine
        .edge_references()
        .map(|e_ref| e_ref.weight().get_event_type())
        .collect::<BTreeSet<EventType>>()
        .intersection(
            &projections[k]
                .0
                .edge_references()
                .map(|e_ref| e_ref.weight().get_event_type())
                .collect::<BTreeSet<EventType>>(),
        )
        .cloned()
        .collect();

    let ((machine_and_proj, machine_and_proj_initial), kth_interface) = (
        composition::compose(
            machine,
            machine_initial,
            projections[k].0.clone(),
            projections[k].1,
            machine_proj_intersect,
            gen_node,
        ),
        projections[k].2.clone(),
    );
    let machine_and_proj = machine_and_proj.map(
        |_, n| AdaptationNode {
            state: State::from(format!("({})", n.state.state_name().clone())),
            ..n.clone()
        },
        |_, label| label.clone(),
    );

    let projections = projections[..k]
        .iter()
        .cloned()
        .chain([(machine_and_proj, machine_and_proj_initial, kth_interface)])
        .chain(projections[k + 1..].iter().cloned())
        .collect();

    match projection::combine_projs(projections, gen_node) {
        Some((combined_projection, combined_initial)) => {
            Some((combined_projection, Some(combined_initial)))
        } // should we minimize here? not done to keep original shape of input machine as much as possible?
        None => None,
    }
}

fn paths_from_event_types(proj: &OptionGraph, proto_info: &ProtoInfo) -> BranchMap {
    let _span = tracing::info_span!("paths_from_event_types").entered();
    let mut m: BTreeMap<EventType, BTreeSet<EventType>> = BTreeMap::new();
    let special_events = proto_info::get_branching_joining_proto_info(proto_info);

    // The reason for making set of concurrent events smaller is?
    let after_pairs: BTreeSet<UnordEventPair> =
        proto_info::transitive_closure_succeeding(proto_info.succeeding_events.clone())
            .into_iter()
            .map(|(e, es)| {
                [e].into_iter()
                    .cartesian_product(&es)
                    .map(|(e1, e2)| proto_info::unord_event_pair(e1, e2.clone()))
                    .collect::<BTreeSet<UnordEventPair>>()
            })
            .flatten()
            .collect();
    let concurrent_events: BTreeSet<UnordEventPair> = proto_info
        .concurrent_events
        .difference(&after_pairs)
        .cloned()
        .collect();

    for node in proj.node_indices() {
        for edge in proj.edges_directed(node, Outgoing) {
            match edge.weight() {
                MachineLabel::Execute { .. } => continue,
                MachineLabel::Input { .. } => {
                    let mut paths_this_edge = visit_successors_stop_on_branch(
                        proj,
                        edge.target(),
                        &edge.weight().get_event_type(),
                        &special_events,
                        &concurrent_events,
                    );
                    m.entry(edge.weight().get_event_type())
                        .and_modify(|s| s.append(&mut paths_this_edge))
                        .or_insert_with(|| paths_this_edge);
                }
            }
        }
    }

    m.into_iter()
        .map(|(t, after_t)| (t, after_t.into_iter().collect()))
        .collect()
}

fn visit_successors_stop_on_branch(
    proj: &OptionGraph,
    machine_state: NodeId,
    et: &EventType,
    special_events: &BTreeSet<EventType>,
    concurrent_events: &BTreeSet<UnordEventPair>,
) -> BTreeSet<EventType> {
    let _span = tracing::info_span!("visit_successors_stop_on_branch").entered();
    let mut visited = BTreeSet::new();
    let mut to_visit = Vec::from([machine_state]);
    let mut event_types = BTreeSet::new();
    //event_types.insert(et.clone());
    while let Some(node) = to_visit.pop() {
        visited.insert(node);
        for e in proj.edges_directed(node, Outgoing) {
            if !concurrent_events
                .contains(&proto_info::unord_event_pair(e.weight().get_event_type(), et.clone()))
            {
                event_types.insert(e.weight().get_event_type());
            }
            if !special_events.contains(&e.weight().get_event_type())
                && !visited.contains(&e.target())
            {
                to_visit.push(e.target());
            }
        }
    }
    event_types
}

fn from_option_graph_to_graph(graph: &OptionGraph) -> Graph {
    graph.map(
        |_, n| n.clone().unwrap_or_else(|| State::new("")),
        |_, x| x.clone(),
    )
}

fn from_adaptation_graph_to_option_graph(graph: &AdaptationGraph) -> OptionGraph {
    graph.map(|_, n| Some(n.state.state_name().clone()), |_, x| x.clone())
}