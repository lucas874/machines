use crate::errors::{Error, ErrorReport};
use crate::types::proto_graph;
use crate::types::proto_label::ProtoLabel;
use crate::types::typescript_types::{
    Command, EventLabel, InterfacingProtocols, Subscriptions, SwarmProtocolType,
};
use crate::types::{
    proto_graph::{Graph, NodeId},
    typescript_types::{EventType, Role, SwarmLabel},
};
use crate::{composability_check, composition};
use itertools::Itertools;
use petgraph::Directed;
use petgraph::algo;
use petgraph::visit::DfsPostOrder;
use petgraph::{
    Direction::{Incoming, Outgoing},
    visit::{Dfs, EdgeRef},
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub type RoleEventMap = BTreeMap<Role, BTreeSet<SwarmLabel>>;

pub type UnordEventPair = BTreeSet<EventType>;

pub fn unord_event_pair(a: EventType, b: EventType) -> UnordEventPair {
    BTreeSet::from([a, b])
}

#[derive(Debug, Clone)]
pub struct ProtoStruct {
    pub graph: Graph,
    pub initial: Option<NodeId>,
    pub errors: Vec<Error>,
    pub roles: BTreeSet<Role>,
}

impl ProtoStruct {
    pub fn new(
        graph: Graph,
        initial: Option<NodeId>,
        errors: Vec<Error>,
        roles: BTreeSet<Role>,
    ) -> Self {
        Self {
            graph,
            initial,
            errors,
            roles,
        }
    }

    pub fn get_triple(&self) -> (Graph, Option<NodeId>, Vec<Error>) {
        (
            self.graph.clone(),
            self.initial.clone(),
            self.errors.clone(),
        )
    }

    pub fn no_errors(&self) -> bool {
        self.errors.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct ProtoInfo {
    pub protocols: Vec<ProtoStruct>,
    pub role_event_map: RoleEventMap,
    pub concurrent_events: BTreeSet<UnordEventPair>, // Consider to make a more specific type. unordered pair.
    pub branching_events: Vec<BTreeSet<EventType>>,
    pub joining_events: BTreeMap<EventType, BTreeSet<EventType>>,
    pub immediately_pre: BTreeMap<EventType, BTreeSet<EventType>>,
    pub succeeding_events: BTreeMap<EventType, BTreeSet<EventType>>,
    pub interfacing_events: BTreeSet<EventType>,
    pub infinitely_looping_events: BTreeSet<EventType>, // Event types that do not lead to a terminal state.
    pub interface_errors: Vec<Error>,
}

impl ProtoInfo {
    pub fn new(
        protocols: Vec<ProtoStruct>,
        role_event_map: RoleEventMap,
        concurrent_events: BTreeSet<UnordEventPair>,
        branching_events: Vec<BTreeSet<EventType>>,
        joining_events: BTreeMap<EventType, BTreeSet<EventType>>,
        immediately_pre: BTreeMap<EventType, BTreeSet<EventType>>,
        succeeding_events: BTreeMap<EventType, BTreeSet<EventType>>,
        interfacing_events: BTreeSet<EventType>,
        infinitely_looping_events: BTreeSet<EventType>,
        interface_errors: Vec<Error>,
    ) -> Self {
        Self {
            protocols,
            role_event_map,
            concurrent_events,
            branching_events,
            joining_events,
            immediately_pre,
            succeeding_events,
            interfacing_events,
            infinitely_looping_events,
            interface_errors,
        }
    }

    pub fn new_only_proto(protocols: Vec<ProtoStruct>) -> Self {
        Self {
            protocols,
            role_event_map: BTreeMap::new(),
            concurrent_events: BTreeSet::new(),
            branching_events: Vec::new(),
            joining_events: BTreeMap::new(),
            immediately_pre: BTreeMap::new(),
            succeeding_events: BTreeMap::new(),
            interfacing_events: BTreeSet::new(),
            infinitely_looping_events: BTreeSet::new(),
            interface_errors: Vec::new(),
        }
    }

    pub fn get_ith_proto(&self, i: usize) -> Option<ProtoStruct> {
        self.protocols.get(i).cloned()
    }

    pub fn no_errors(&self) -> bool {
        self.protocols.iter().all(|p| p.no_errors()) && self.interface_errors.is_empty()
    }

    // Return all values from a ProtoInfo.role_event_map field as a set of triples:
    // (Command, EventType, Role)
    fn get_labels(&self) -> BTreeSet<(Command, EventType, Role)> {
        self.role_event_map
            .values()
            .flat_map(|role_info| {
                role_info
                    .iter()
                    .map(|sl| (sl.cmd.clone(), sl.get_event_type(), sl.role.clone()))
            })
            .collect()
    }

    // If we accumulate errors this map should really map to a set of (Command, Role)...
    pub fn event_type_map(&self) -> BTreeMap<EventType, (Command, Role)> {
        self.get_labels()
            .into_iter()
            .map(|(c, t, r)| (t, (c, r)))
            .collect()
    }

    // If we accumulate errors this map should really map to a set of (EvenType, Role)...
    pub fn command_map(&self) -> BTreeMap<Command, (EventType, Role)> {
        self.get_labels()
            .into_iter()
            .map(|(c, t, r)| (c, (t, r)))
            .collect()
    }

    // Consumes the proto info instance to create an error report.
    pub fn to_error_report(self) -> ErrorReport {
        ErrorReport(
            self.protocols
                .into_iter()
                .map(|p| (p.graph, p.errors))
                .chain([(Graph::new(), self.interface_errors)]) // NO!!! Why not?
                .collect(),
        )
    }

    pub fn get_succeeding(&self, event_type: &EventType) -> BTreeSet<EventType> {
        self.succeeding_events
            .get(event_type)
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_preceding(&self, event_type: &EventType) -> BTreeSet<EventType> {
        self.immediately_pre
            .get(event_type)
            .cloned()
            .unwrap_or_default()
    }
}

// Overapproximate concurrent events.
// Anything from different protocols that are not interfacing events is considered concurrent.
// Pre: interface has been checked.
fn get_concurrent_events(
    proto_info1: &ProtoInfo,
    proto_info2: &ProtoInfo,
    interfacing_event_types: &BTreeSet<EventType>,
) -> BTreeSet<UnordEventPair> {
    let _span = tracing::info_span!("get_concurrent_events").entered();
    let concurrent_events_union: BTreeSet<UnordEventPair> = proto_info1
        .concurrent_events
        .union(&proto_info2.concurrent_events)
        .cloned()
        .collect();
    let events_proto1: BTreeSet<EventType> = proto_info1
        .get_event_types()
        .difference(interfacing_event_types)
        .cloned()
        .collect();
    let events_proto2: BTreeSet<EventType> = proto_info2
        .get_event_types()
        .difference(interfacing_event_types)
        .cloned()
        .collect();
    let cartesian_product = events_proto1
        .into_iter()
        .cartesian_product(&events_proto2)
        .map(|(a, b)| unord_event_pair(a, b.clone()))
        .collect();

    concurrent_events_union
        .union(&cartesian_product)
        .cloned()
        .collect()
}

pub fn get_updating_event_types(
    proto_info: &ProtoInfo,
    subscriptions: &Subscriptions,
) -> BTreeSet<EventType> {
    proto_info
        .branching_events
        .clone()
        .into_iter()
        .flatten()
        .chain(
            proto_info
                .joining_events
                .keys()
                .cloned()
                .collect::<BTreeSet<EventType>>(),
        )
        .chain(infinitely_looping_event_types_in_sub(
            proto_info,
            subscriptions,
        ))
        .collect()
}

// Identify those infinitely looping event types G-t-> present in subscriptions of roles(t, G, subscriptions)
// If multiple event types in the same loop satisfy the condition for being a looping event type in subscription
// then return all of them instead of picking one which would be enough. Design choice.
// Instead we could take the smallest event type (according to some order) from each loop? If not covered by branching?
fn infinitely_looping_event_types_in_sub(
    proto_info: &ProtoInfo,
    subscriptions: &Subscriptions,
) -> BTreeSet<EventType> {
    // infinitely_looping.filter(|t| if all roles in roles_on_path subscribe to t then true otherwise false)
    let _span = tracing::info_span!("infinitely_looping_event_types_in_sub").entered();
    // all distinct loops
    let loops: BTreeSet<BTreeSet<EventType>> = proto_info
        .infinitely_looping_events
        .iter()
        .map(|t| {
            proto_info
                .get_succeeding(t)
                .into_iter()
                .chain([t.clone()])
                .collect::<BTreeSet<EventType>>()
        })
        .collect();

    // event types in those loops that all involved roles subscribe to
    let loops: BTreeSet<BTreeSet<EventType>> = loops
        .into_iter()
        .map(|a_loop| {
            a_loop
                .into_iter()
                .filter(|t| {
                    roles_on_path(t.clone(), proto_info, subscriptions)
                        .iter()
                        .all(|r| {
                            subscriptions
                                .get(r)
                                .is_some_and(|event_types_r| event_types_r.contains(t))
                        })
                })
                .collect()
        })
        .collect();

    // first: 'Returns a reference to the first element in the set, if any. This element is always the minimum of all elements in the set.'
    loops
        .into_iter()
        .map(|event_types| event_types.first().cloned())
        .filter_map(|option_event_type| option_event_type)
        .collect()
}

// Set of interfacing roles between two protocols
#[inline]
fn get_interfacing_roles(proto_info1: &ProtoInfo, proto_info2: &ProtoInfo) -> BTreeSet<Role> {
    proto_info1
        .protocols
        .iter()
        .flat_map(|protostruct| protostruct.roles.clone())
        .collect::<BTreeSet<Role>>()
        .intersection(
            &proto_info2
                .protocols
                .iter()
                .flat_map(|protostruct| protostruct.roles.clone())
                .collect::<BTreeSet<Role>>(),
        )
        .cloned()
        .collect()
}

// The interfacing roles are those roles that appear in proto_info1 and in proto_info2
// The interfacing event types are those emitted by the interfacing role in either proto_info1 or proto_info2.
// Assumes that proto_info1 and proto_info2 interface correctly.
#[inline]
fn get_interfacing_event_types(
    proto_info1: &ProtoInfo,
    proto_info2: &ProtoInfo,
) -> BTreeSet<EventType> {
    get_interfacing_roles(proto_info1, proto_info2)
        .iter()
        .flat_map(|r| {
            proto_info1
                .role_event_map
                .get(r)
                .unwrap()
                .union(&proto_info2.role_event_map.get(r).unwrap())
        })
        .map(|swarm_label| swarm_label.get_event_type())
        .collect()
}

// Construct map from joining event types to concurrent events preceding joining event types.
#[inline]
fn joining_event_types_map(proto_info: &ProtoInfo) -> BTreeMap<EventType, BTreeSet<EventType>> {
    let pre_joins = |e: &EventType| -> BTreeSet<EventType> {
        let pre = proto_info.get_preceding(e);
        let product = pre.clone().into_iter().cartesian_product(&pre);
        product
            .filter(|(e1, e2)| {
                *e1 != **e2 // necessary? Not the case if in set of concurrent?
                    && proto_info
                        .concurrent_events
                        .contains(&unord_event_pair(e1.clone(), (*e2).clone()))
            })
            .map(|(e1, e2)| [e1, e2.clone()])
            .flatten()
            .collect()
    };
    // Get those interfacing event types with immediately preceding conucurrent event types and turn it into a map
    proto_info
        .interfacing_events
        .iter()
        .map(|e| (e.clone(), pre_joins(e)))
        .filter(|(_, pre)| !pre.is_empty())
        .collect()
}

pub fn flatten_joining_map(
    joining_event_types: &BTreeMap<EventType, BTreeSet<EventType>>,
) -> BTreeSet<EventType> {
    joining_event_types
        .iter()
        .flat_map(|(join, pre)| pre.clone().into_iter().chain([join.clone()]))
        .collect()
}

// Combine fields of two proto infos.
// Do not compute transitive closure of happens after and do not compute joining event types.
fn combine_two_proto_infos(proto_info1: ProtoInfo, proto_info2: ProtoInfo) -> ProtoInfo {
    let _span = tracing::info_span!("combine_proto_infos").entered();
    let interface_errors = composability_check::check_interface(&proto_info1, &proto_info2);
    let interfacing_event_types = get_interfacing_event_types(&proto_info1, &proto_info2);
    let protocols = vec![proto_info1.protocols.clone(), proto_info2.protocols.clone()].concat();
    let role_event_map = combine_maps(
        proto_info1.role_event_map.clone(),
        proto_info2.role_event_map.clone(),
        None,
    );
    // get concurrent event types based on current set of interfacing event types.
    let concurrent_events =
        get_concurrent_events(&proto_info1, &proto_info2, &interfacing_event_types);
    let branching_events: Vec<BTreeSet<EventType>> = proto_info1
        .branching_events
        .into_iter()
        .chain(proto_info2.branching_events.into_iter())
        .collect();
    let immediately_pre = combine_maps(
        proto_info1.immediately_pre.clone(),
        proto_info2.immediately_pre.clone(),
        None,
    );
    let happens_after = combine_maps(
        proto_info1.succeeding_events,
        proto_info2.succeeding_events,
        None,
    );

    let interfacing_event_types = [
        proto_info1.interfacing_events,
        proto_info2.interfacing_events,
        interfacing_event_types,
    ]
    .into_iter()
    .flatten()
    .collect();

    let infinitely_looping_events = proto_info1
        .infinitely_looping_events
        .into_iter()
        .chain(proto_info2.infinitely_looping_events.into_iter())
        .collect();

    ProtoInfo::new(
        protocols,
        role_event_map,
        concurrent_events,
        branching_events,
        BTreeMap::new(),
        immediately_pre,
        happens_after,
        interfacing_event_types,
        infinitely_looping_events,
        [
            proto_info1.interface_errors,
            proto_info2.interface_errors,
            interface_errors,
        ]
        .concat(),
    )
}

pub fn combine_proto_infos(protos: Vec<ProtoInfo>) -> ProtoInfo {
    let _span = tracing::info_span!("combine_proto_infos_fold").entered();
    if protos.is_empty() {
        return ProtoInfo::new_only_proto(vec![]);
    }

    let mut combined = protos[1..]
        .to_vec()
        .into_iter()
        .fold(protos[0].clone(), |acc, p| combine_two_proto_infos(acc, p));

    combined.joining_events = joining_event_types_map(&combined);
    combined
}

// Construct a ProtoInfo containing all protocols, all branching events, joining events etc.
// Then add any errors arising from confusion freeness to the proto info and return it.
// Does not compute transitive closure of combined succeeding_events, simply takes union of component succeeding_events fields.
pub fn swarms_to_proto_info(protos: InterfacingProtocols) -> ProtoInfo {
    let _span = tracing::info_span!("swarms_to_proto_info").entered();
    let combined_proto_info = combine_proto_infos(prepare_proto_infos(protos));
    composability_check::confusion_free_proto_info(combined_proto_info)
}

pub fn prepare_proto_infos(protos: InterfacingProtocols) -> Vec<ProtoInfo> {
    let _span = tracing::info_span!("prepare_proto_infos").entered();
    protos
        .0
        .iter()
        .map(|p| prepare_proto_info(p.clone()))
        .collect()
}

// Precondition: proto does not contain concurrency.
pub(crate) fn prepare_proto_info(proto: SwarmProtocolType) -> ProtoInfo {
    let _span = tracing::info_span!("prepare_proto_info").entered();
    let mut role_event_map: RoleEventMap = BTreeMap::new();
    let mut branching_events = Vec::new();
    let mut immediately_pre_map: BTreeMap<EventType, BTreeSet<EventType>> = BTreeMap::new();
    let (graph, initial, errors) = proto_graph::swarm_to_graph(&proto);
    if initial.is_none() || !errors.is_empty() {
        return ProtoInfo::new_only_proto(vec![ProtoStruct::new(
            graph,
            initial,
            errors,
            BTreeSet::new(),
        )]);
    }

    let mut walk = Dfs::new(&graph, initial.unwrap());

    // Add to set of branching and joining.
    // Graph contains no concurrency, so:
    //      Branching event types are all outgoing event types if more than one and if more than one distinct target.
    //      Immediately preceding to each edge are all incoming event types
    while let Some(node_id) = walk.next(&graph) {
        let outgoing_labels: Vec<_> = graph
            .edges_directed(node_id, Outgoing)
            .map(|edge| edge.weight())
            .collect();
        let incoming_event_types: BTreeSet<EventType> = graph
            .edges_directed(node_id, Incoming)
            .map(|edge| edge.weight().get_event_type())
            .collect();

        if outgoing_labels.len() > 1 && direct_successors(&graph, node_id).len() > 1 {
            branching_events.push(
                outgoing_labels
                    .iter()
                    .map(|edge| edge.get_event_type())
                    .collect(),
            );
        }

        for label in outgoing_labels {
            role_event_map
                .entry(label.role.clone())
                .and_modify(|role_info| {
                    role_info.insert(label.clone());
                })
                .or_insert_with(|| BTreeSet::from([label.clone()]));

            immediately_pre_map
                .entry(label.get_event_type())
                .and_modify(|events| {
                    events.append(&mut incoming_event_types.clone());
                })
                .or_insert_with(|| incoming_event_types.clone());
        }
    }

    // Consider changing after_not_concurrent to not take concurrent events as argument. now that we do not consider swarms with concurrency here.
    let happens_after = after_not_concurrent(&graph, initial.unwrap(), &BTreeSet::new());

    // Nodes that can not reach a terminal node.
    let infinitely_looping_events =
        proto_graph::infinitely_looping_event_types(&graph, &happens_after);

    ProtoInfo::new(
        vec![ProtoStruct::new(
            graph,
            initial,
            errors,
            role_event_map.keys().cloned().collect(),
        )],
        role_event_map,
        BTreeSet::new(),
        branching_events,
        BTreeMap::new(),
        immediately_pre_map,
        happens_after,
        BTreeSet::new(),
        infinitely_looping_events,
        vec![],
    )
}

// Set of direct successor nodes from node (those reachable in one step).
fn direct_successors(graph: &Graph, node: NodeId) -> BTreeSet<NodeId> {
    graph
        .edges_directed(node, Outgoing)
        .map(|e| e.target())
        .collect()
}

// Compute a map mapping event types to the set of event types that follow it.
// I.e. for each event type t all those event types t' that can be emitted after t.
fn after_not_concurrent(
    graph: &Graph,
    initial: NodeId,
    concurrent_events: &BTreeSet<BTreeSet<EventType>>,
) -> BTreeMap<EventType, BTreeSet<EventType>> {
    let _span = tracing::info_span!("after_not_concurrent").entered();
    let mut succ_map: BTreeMap<EventType, BTreeSet<EventType>> = BTreeMap::new();
    let mut is_stable = after_not_concurrent_step(graph, initial, concurrent_events, &mut succ_map);

    while !is_stable {
        is_stable = after_not_concurrent_step(graph, initial, concurrent_events, &mut succ_map);
    }

    succ_map
}

// For each event type t we get a set ('active_in_successor') of event types
// that only contains event types that are immediately after t and not concurrent with t.
// We then add each event type t' in active_in_successor and all the event types t''
// that we already know are after t' to the set of event types succeeding t.
fn after_not_concurrent_step(
    graph: &Graph,
    initial: NodeId,
    concurrent_events: &BTreeSet<BTreeSet<EventType>>,
    succ_map: &mut BTreeMap<EventType, BTreeSet<EventType>>,
) -> bool {
    if graph.node_count() == 0 || initial == NodeId::end() {
        return true;
    }
    let mut is_stable = true;
    let mut walk = DfsPostOrder::new(&graph, initial);
    while let Some(node) = walk.next(&graph) {
        for edge in graph.edges_directed(node, Outgoing) {
            let event_type = edge.weight().get_event_type();
            let active_in_successor = proto_graph::active_transitions_not_conc(
                edge.target(),
                graph,
                &event_type,
                concurrent_events,
            )
            .into_iter()
            .map(|label| label.get_event_type());

            let mut succ_events: BTreeSet<EventType> = active_in_successor
                .clone()
                .into_iter()
                .flat_map(|e| {
                    let events = succ_map.get(&e).cloned().unwrap_or_default();
                    events.clone()
                })
                .chain(active_in_successor.into_iter())
                .collect();

            if !succ_map.contains_key(&event_type)
                || !succ_events
                    .iter()
                    .all(|e| succ_map[&event_type].contains(e))
            {
                succ_map
                    .entry(event_type)
                    .and_modify(|events| {
                        events.append(&mut succ_events);
                    })
                    .or_insert(succ_events);
                is_stable = false;
            }
        }
    }
    is_stable
}

pub fn transitive_closure_succeeding(
    succ_map: BTreeMap<EventType, BTreeSet<EventType>>,
) -> BTreeMap<EventType, BTreeSet<EventType>> {
    let _span = tracing::info_span!("transitive_closure_succeeding").entered();
    let mut graph: petgraph::Graph<EventType, (), Directed> = petgraph::Graph::new();
    let mut node_map = BTreeMap::new();
    for (event, succeeding) in &succ_map {
        if !node_map.contains_key(event) {
            node_map.insert(event.clone(), graph.add_node(event.clone()));
        }
        for succ in succeeding {
            if !node_map.contains_key(succ) {
                node_map.insert(succ.clone(), graph.add_node(succ.clone()));
            }
            graph.add_edge(node_map[event], node_map[succ], ());
        }
    }

    let reflexive_transitive_closure = algo::floyd_warshall(&graph, |_| 1);
    let transitive_closure: Vec<_> = reflexive_transitive_closure
        .unwrap_or_else(|_| HashMap::new())
        .into_iter()
        .filter(|(_, v)| *v != i32::MAX && *v != 0)
        .map(|(related_pair, _)| related_pair)
        .collect();

    let mut succ_map_new: BTreeMap<EventType, BTreeSet<EventType>> = BTreeMap::new();
    for (i1, i2) in transitive_closure {
        succ_map_new
            .entry(graph[i1].clone())
            .and_modify(|succeeding_events| {
                succeeding_events.insert(graph[i2].clone());
            })
            .or_insert_with(|| BTreeSet::from([graph[i2].clone()]));
    }

    // do this because of loops. everything reachable from itself in result from floyd_warshall(), but we filter these out. add them again if loops.
    combine_maps(succ_map, succ_map_new, None)
}

// The involved roles on a path are those roles that subscribe to one or
// more of the event types emitted in a transition reachable from the transition
// represented by its emitted event 'event_type'.
pub fn roles_on_path(
    event_type: EventType,
    proto_info: &ProtoInfo,
    subs: &Subscriptions,
) -> BTreeSet<Role> {
    let succeeding_events: BTreeSet<EventType> = proto_info
        .get_succeeding(&event_type)
        .into_iter()
        .chain([event_type])
        .collect();
    subs.iter()
        .filter(|(_, events)| events.intersection(&succeeding_events).count() != 0)
        .map(|(r, _)| r.clone())
        .collect()
}

pub fn explicit_composition_proto_info(proto_info: ProtoInfo) -> ProtoInfo {
    let _span = tracing::info_span!("explicit_composition_proto_info").entered();
    let (composed, composed_initial) = explicit_composition(&proto_info);
    let succeeding_events =
        after_not_concurrent(&composed, composed_initial, &proto_info.concurrent_events);
    let infinitely_looping_events =
        proto_graph::infinitely_looping_event_types(&composed, &succeeding_events);
    ProtoInfo {
        protocols: vec![ProtoStruct::new(
            composed,
            Some(composed_initial),
            vec![],
            BTreeSet::new(),
        )],
        succeeding_events,
        infinitely_looping_events,
        ..proto_info
    }
}

// precondition: the protocols can interface on the given interfaces
fn explicit_composition(proto_info: &ProtoInfo) -> (Graph, NodeId) {
    let _span = tracing::info_span!("explicit_composition").entered();
    if proto_info.protocols.is_empty() {
        return (Graph::new(), NodeId::end());
    }

    let (g, i, _) = proto_info.protocols[0].get_triple();
    let g_roles = g.get_roles();
    let folder = |(acc_g, acc_i, acc_roles): (Graph, NodeId, BTreeSet<Role>),
                  p: ProtoStruct|
     -> (Graph, NodeId, BTreeSet<Role>) {
        let empty = BTreeSet::new();
        let interface = acc_roles
            .intersection(&p.graph.get_roles())
            .cloned()
            .flat_map(|role| {
                proto_info
                    .role_event_map
                    .get(&role)
                    .unwrap_or(&empty)
                    .iter()
                    .map(|label| label.get_event_type())
            })
            .collect();
        let acc_roles = acc_roles.into_iter().chain(p.graph.get_roles()).collect();
        let (graph, initial) = composition::compose(
            acc_g,
            acc_i,
            p.graph,
            p.initial.unwrap(),
            interface,
            composition::gen_state_name,
        );
        (graph, initial, acc_roles)
    };
    let (graph, initial, _) = proto_info.protocols[1..]
        .to_vec()
        .into_iter()
        .fold((g, i.unwrap(), g_roles), folder);
    (graph, initial)
}

// Construct a graph that is the 'expanded' composition of protos.
pub fn compose_protocols(protos: InterfacingProtocols) -> Result<(Graph, NodeId), ErrorReport> {
    let _span = tracing::info_span!("compose_protocols").entered();
    let combined_proto_info = swarms_to_proto_info(protos);
    if !combined_proto_info.no_errors() {
        return Err(combined_proto_info.to_error_report());
    }

    let p = explicit_composition_proto_info(combined_proto_info)
        .get_ith_proto(0)
        .unwrap();
    Ok((p.graph, p.initial.unwrap()))
}

// combine maps with sets as values
fn combine_maps<K: Ord + Clone, V: Ord + Clone>(
    map1: BTreeMap<K, BTreeSet<V>>,
    map2: BTreeMap<K, BTreeSet<V>>,
    extra: Option<BTreeSet<V>>,
) -> BTreeMap<K, BTreeSet<V>> {
    let all_keys: BTreeSet<K> = map1.keys().chain(map2.keys()).cloned().collect();
    let extra = extra.unwrap_or(BTreeSet::new());
    let extend_for_key = |k: &K| -> (K, BTreeSet<V>) {
        (
            k.clone(),
            map1.get(k)
                .unwrap_or(&BTreeSet::new())
                .union(map2.get(k).unwrap_or(&BTreeSet::new()))
                .chain(&extra)
                .cloned()
                .collect(),
        )
    };

    all_keys.iter().map(extend_for_key).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;

    #[test]
    fn test_after_not_concurrent() {
        let proto1: SwarmProtocolType =
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        { "source": "0", "target": "1", "label": { "cmd": "i1", "logType": ["i1"], "role": "IR" } },
                        { "source": "1", "target": "2", "label": { "cmd": "a", "logType": ["a"], "role": "R1" } },
                        { "source": "2", "target": "3", "label": { "cmd": "b", "logType": ["b"], "role": "R1" } },
                        { "source": "3", "target": "4", "label": { "cmd": "i2", "logType": ["i2"], "role": "IR" } }
                    ]
                }"#,
            )
            .unwrap();

        let proto2: SwarmProtocolType =
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        { "source": "0", "target": "1", "label": { "cmd": "i1", "logType": ["i1"], "role": "IR" } },
                        { "source": "1", "target": "2", "label": { "cmd": "c", "logType": ["c"], "role": "R2" } },
                        { "source": "2", "target": "3", "label": { "cmd": "d", "logType": ["d"], "role": "R2" } },
                        { "source": "3", "target": "4", "label": { "cmd": "i2", "logType": ["i2"], "role": "IR" } }
                    ]
                }"#,
            )
            .unwrap();

        let interfacing_swarms = InterfacingProtocols(vec![proto1, proto2]);

        let expected_after = BTreeMap::from([
            (
                EventType::new("i1"),
                BTreeSet::from([
                    EventType::new("a"),
                    EventType::new("b"),
                    EventType::new("c"),
                    EventType::new("d"),
                    EventType::new("i2"),
                ]),
            ),
            (
                EventType::new("a"),
                BTreeSet::from([EventType::new("b"), EventType::new("i2")]),
            ),
            (EventType::new("b"), BTreeSet::from([EventType::new("i2")])),
            (
                EventType::new("c"),
                BTreeSet::from([EventType::new("d"), EventType::new("i2")]),
            ),
            (EventType::new("d"), BTreeSet::from([EventType::new("i2")])),
            (EventType::new("i2"), BTreeSet::from([])),
        ]);

        let expected_concurrent = BTreeSet::from([
            unord_event_pair(EventType::new("a"), EventType::new("c")),
            unord_event_pair(EventType::new("a"), EventType::new("d")),
            unord_event_pair(EventType::new("b"), EventType::new("c")),
            unord_event_pair(EventType::new("b"), EventType::new("d")),
        ]);

        let combined_proto_info =
            combine_proto_infos(prepare_proto_infos(interfacing_swarms.clone()));

        assert_eq!(expected_after, combined_proto_info.succeeding_events);
        assert_eq!(expected_concurrent, combined_proto_info.concurrent_events);

        let (composition, composition_initial) =
            compose_protocols(interfacing_swarms.clone()).unwrap();

        let after_map = after_not_concurrent(
            &composition,
            composition_initial,
            &combined_proto_info.concurrent_events,
        );
        assert_eq!(expected_after, after_map);
    }

    #[test]
    fn test_interface() {
        let proto1: SwarmProtocolType =
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        { "source": "0", "target": "1", "label": { "cmd": "i1", "logType": ["i1"], "role": "IR1" } },
                        { "source": "1", "target": "2", "label": { "cmd": "a", "logType": ["a"], "role": "R1" } }
                    ]
                }"#,
            )
            .unwrap();

        let proto2: SwarmProtocolType =
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        { "source": "0", "target": "1", "label": { "cmd": "i1", "logType": ["i1"], "role": "IR1" } },
                        { "source": "1", "target": "2", "label": { "cmd": "i2", "logType": ["i2"], "role": "IR2" } }
                    ]
                }"#,
            )
            .unwrap();

        let proto3: SwarmProtocolType =
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        { "source": "0", "target": "1", "label": { "cmd": "i2", "logType": ["i2"], "role": "IR2" } },
                        { "source": "1", "target": "2", "label": { "cmd": "c", "logType": ["i1"], "role": "R3" } }
                    ]
                }"#,
            )
            .unwrap();

        let interfacing_swarms = InterfacingProtocols(vec![proto1, proto2, proto3]);

        let combined_proto_info =
            combine_proto_infos(prepare_proto_infos(interfacing_swarms.clone()));

        // The IR1 not used as an interface refers to the composition of (p || proto3) where p = (proto1 || proto2)
        let expected_errors = vec!["Event type i1 appears as i1@IR1<i1> and as c@R3<i1>"];
        let mut errors = combined_proto_info.to_error_report().to_strings();
        errors.sort();
        assert_eq!(expected_errors, errors);

        let proto1: SwarmProtocolType =
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        { "source": "0", "target": "1", "label": { "cmd": "i1", "logType": ["i1"], "role": "IR1" } },
                        { "source": "1", "target": "2", "label": { "cmd": "a", "logType": ["a"], "role": "R1" } }
                    ]
                }"#,
            )
            .unwrap();

        let proto2: SwarmProtocolType =
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        { "source": "0", "target": "1", "label": { "cmd": "i1", "logType": ["i1"], "role": "IR1" } },
                        { "source": "1", "target": "2", "label": { "cmd": "i2", "logType": ["i2"], "role": "IR1" } },
                        { "source": "2", "target": "3", "label": { "cmd": "i3", "logType": ["i3"], "role": "IR2" } },
                        { "source": "3", "target": "4", "label": { "cmd": "i4", "logType": ["i4"], "role": "IR2" } }
                    ]
                }"#,
            )
            .unwrap();

        let proto3: SwarmProtocolType =
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        { "source": "0", "target": "1", "label": { "cmd": "i3", "logType": ["i3"], "role": "IR2" } },
                        { "source": "1", "target": "2", "label": { "cmd": "i5", "logType": ["i4"], "role": "IR2" } }
                    ]
                }"#,
            )
            .unwrap();

        let interfacing_swarms = InterfacingProtocols(vec![proto1, proto2, proto3]);

        let combined_proto_info =
            combine_proto_infos(prepare_proto_infos(interfacing_swarms.clone()));

        // The IR1 not used as an interface refers to the composition of (p || proto3) where p = (proto1 || proto2)
        let expected_errors = vec!["Event type i4 appears as i4@IR2<i4> and as i5@IR2<i4>"];
        let mut errors = combined_proto_info.to_error_report().to_strings();
        errors.sort();
        assert_eq!(expected_errors, errors);
    }

    #[test]
    fn test_joining_event_types() {
        // e_r0
        // e_ir
        let preceding_events = |range: std::ops::Range<usize>| -> BTreeSet<EventType> {
            range
                .into_iter()
                .map(|u| EventType::new(&format!("e_r{}", u)))
                .collect()
        };
        test_utils::setup_logger();
        for i in 1..6 {
            let index = i as usize;
            let proto_info = swarms_to_proto_info(InterfacingProtocols(
                test_utils::get_interfacing_swarms_pat_4().0[..index].to_vec(),
            ));
            if i == 1 {
                assert_eq!(proto_info.joining_events, BTreeMap::new());
            } else {
                assert_eq!(
                    proto_info.joining_events,
                    BTreeMap::from([(EventType::new("e_ir"), preceding_events(0..i))])
                );
            }
        }
    }

    #[test]
    fn test_empty_set_of_protocols() {
        let error_report = ProtoInfo::new_only_proto(vec![]).to_error_report();
        assert!(error_report.is_empty());
    }

    mod loop_tests {
        use super::*;
        use crate::subscription::{exact, overapproximation};
        use crate::types::typescript_types::Granularity;

        macro_rules! check_looping_event_types {
            ($protocol:expr, $expected_infinitely_looping_in_sub:expr) => {
                let interfacing_protocols = InterfacingProtocols(vec![$protocol.clone()]);
                let exact_subscriptions =
                    exact::exact_well_formed_sub(interfacing_protocols.clone(), &BTreeMap::new())
                        .unwrap();
                let overapproximated_subscriptions = overapproximation::overapprox_well_formed_sub(
                    interfacing_protocols.clone(),
                    &BTreeMap::new(),
                    Granularity::TwoStep,
                )
                .unwrap();
                let proto_info = prepare_proto_info($protocol);
                let infinitely_looping_in_exact =
                    infinitely_looping_event_types_in_sub(&proto_info, &exact_subscriptions);
                let infinitely_looping_in_approx = infinitely_looping_event_types_in_sub(
                    &proto_info,
                    &overapproximated_subscriptions,
                );

                assert_eq!(
                    infinitely_looping_in_exact,
                    $expected_infinitely_looping_in_sub
                );
                assert_eq!(
                    infinitely_looping_in_approx,
                    $expected_infinitely_looping_in_sub
                );
            };
        }

        // This module contains tests for relating to looping event types.
        #[test]
        fn looping_1() {
            test_utils::setup_logger();
            // Check states that can not reach terminal state an infinitely looping event types
            let proto_info =
                swarms_to_proto_info(InterfacingProtocols(
                    vec![test_utils::get_looping_proto_1()],
                ));
            assert!(proto_info.no_errors());
            assert_eq!(
                proto_info
                    .infinitely_looping_events
                    .clone()
                    .into_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>(),
                vec!["c", "d", "e"]
            );
        }

        #[test]
        fn looping_2() {
            test_utils::setup_logger();
            // Check states that can not reach terminal state an infinitely looping event types
            let proto_info =
                swarms_to_proto_info(InterfacingProtocols(
                    vec![test_utils::get_looping_proto_2()],
                ));
            assert!(proto_info.no_errors());
            assert_eq!(
                proto_info
                    .infinitely_looping_events
                    .clone()
                    .into_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>(),
                vec!["c", "d", "e"]
            );
        }

        #[test]
        fn looping_3() {
            test_utils::setup_logger();
            // Check states that can not reach terminal state an infinitely looping event types
            let proto_info =
                swarms_to_proto_info(InterfacingProtocols(
                    vec![test_utils::get_looping_proto_3()],
                ));
            assert!(proto_info.no_errors());
            assert_eq!(
                proto_info
                    .infinitely_looping_events
                    .clone()
                    .into_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>(),
                vec!["c", "d", "e", "f", "g", "h", "i"]
            );
        }

        #[test]
        fn looping_4() {
            test_utils::setup_logger();

            // Check states that can not reach terminal state an infinitely looping event types
            let proto_info =
                swarms_to_proto_info(InterfacingProtocols(
                    vec![test_utils::get_looping_proto_4()],
                ));
            assert!(proto_info.no_errors());
            assert_eq!(
                proto_info
                    .infinitely_looping_events
                    .clone()
                    .into_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>(),
                vec!["c", "d", "e", "f", "g", "h"]
            );
        }

        #[test]
        fn looping_5() {
            test_utils::setup_logger();
            // Check states that can not reach terminal state an infinitely looping event types
            let proto_info =
                swarms_to_proto_info(InterfacingProtocols(
                    vec![test_utils::get_looping_proto_5()],
                ));
            assert!(proto_info.no_errors());
            assert_eq!(
                proto_info
                    .infinitely_looping_events
                    .clone()
                    .into_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>(),
                vec!["a", "b", "c", "d", "e", "f", "g", "h"]
            );
        }

        #[test]
        fn looping_6() {
            test_utils::setup_logger();
            // Check states that can not reach terminal state an infinitely looping event types
            let proto_info =
                swarms_to_proto_info(InterfacingProtocols(
                    vec![test_utils::get_looping_proto_6()],
                ));
            assert!(proto_info.no_errors());
            assert_eq!(
                proto_info
                    .infinitely_looping_events
                    .clone()
                    .into_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>(),
                vec!["a", "b", "c", "d", "e", "f"]
            );
        }

        // Test that looping event types in subscriptions are correctly identified.
        #[test]
        fn identify_looping_1() {
            test_utils::setup_logger();
            let proto = test_utils::get_looping_proto_1();
            // c and d are part of the same loop and both satisfy the looping condition.
            // We pick c as THE looping event type for the loop in the subscription.
            let expected_infinitely_looping_in_sub = BTreeSet::from([EventType::new("c")]);
            check_looping_event_types!(proto, expected_infinitely_looping_in_sub);
        }

        #[test]
        fn identify_looping_2() {
            test_utils::setup_logger();
            let proto = test_utils::get_looping_proto_2();
            let expected_infinitely_looping_in_sub = BTreeSet::from([EventType::new("c")]);
            check_looping_event_types!(proto, expected_infinitely_looping_in_sub);
        }

        #[test]
        fn identify_looping_3() {
            test_utils::setup_logger();
            let proto = test_utils::get_looping_proto_3();
            let expected_infinitely_looping_in_sub =
                BTreeSet::from([EventType::new("c"), EventType::new("f")]);
            check_looping_event_types!(proto, expected_infinitely_looping_in_sub);
        }

        #[test]
        fn identify_looping_4() {
            test_utils::setup_logger();
            let proto = test_utils::get_looping_proto_4();
            let expected_infinitely_looping_in_sub = BTreeSet::from([EventType::new("c")]);
            check_looping_event_types!(proto, expected_infinitely_looping_in_sub);
        }

        #[test]
        fn identify_looping_5() {
            test_utils::setup_logger();
            let proto = test_utils::get_looping_proto_5();
            // a and e are part of the same loop and both satisfy the condition for being a looping event type.
            // They are added because they are branching, before looping step.
            // We pick a as the looping event type in the subscription for that loop.
            // Although it does not really matter here --> both a and e are in the set of updating event types because they are branching.
            let expected_infinitely_looping_in_sub = BTreeSet::from([EventType::new("a")]);
            check_looping_event_types!(proto, expected_infinitely_looping_in_sub);
        }

        #[test]
        fn identify_looping_6() {
            test_utils::setup_logger();
            let proto = test_utils::get_looping_proto_6();
            // a and e are part of the same loop and both satisfy the condition for being a looping event type.
            // They are added because they are branching, before looping step.
            // We pick a as the looping event type in the subscription for that loop.
            // Although it does not really matter here --> both a and e are in the set of updating event types because they are branching.
            let expected_infinitely_looping_in_sub = BTreeSet::from([EventType::new("b")]);
            check_looping_event_types!(proto, expected_infinitely_looping_in_sub);
        }
    }
}
