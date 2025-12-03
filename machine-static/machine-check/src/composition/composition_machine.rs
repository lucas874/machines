use super::{
    composition_types::{
        get_branching_joining_proto_info, unord_event_pair, BranchMap, EventLabel,
        ProjToMachineStates, ProjectionInfo, ProtoInfo, ProtoStruct, UnordEventPair,
    },
    types::{Command, StateName, Transition},
    EventType, MachineLabel, MachineType, NodeId, Role, State, Subscriptions, SwarmLabel,
};
use crate::{
    composition::composition_swarm::transitive_closure_succeeding,
    machine::{Error, Side},
};
use itertools::Itertools;
use petgraph::{
    graph::EdgeReference,
    visit::{EdgeFiltered, EdgeRef, IntoEdgeReferences, IntoEdgesDirected, IntoNodeReferences},
    Direction::{Incoming, Outgoing},
};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};
// types more or less copied from machine.rs.
type Graph = petgraph::Graph<State, MachineLabel>;
type OptionGraph = petgraph::Graph<Option<State>, MachineLabel>;
type ERef<'a> = <&'a super::Graph as IntoEdgeReferences>::EdgeRef;

impl From<String> for State {
    fn from(value: String) -> State {
        State::new(&value)
    }
}

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

// Vec of triples of the form:
//      (protocol_graph, initial_node, interfacing event types with vec[i-1])
// Protocols linked together in a 'chain' by interfacing event types
type ChainedProtos = Vec<(
    petgraph::Graph<State, SwarmLabel>,
    NodeId,
    BTreeSet<EventType>,
)>;

// Same as type above, but for projections
type ChainedProjections = Vec<(Graph, NodeId, BTreeSet<EventType>)>;

// Similar to machine::project, except that transitions with event types
// not subscribed to by role are skipped.
pub fn project(
    swarm: &super::Graph,
    initial: NodeId,
    subs: &Subscriptions,
    role: Role,
    minimize: bool,
) -> (Graph, NodeId) {
    let _span = tracing::info_span!("project", %role).entered();
    let mut machine = Graph::new();
    let sub = BTreeSet::new();
    let sub = subs.get(&role).unwrap_or(&sub);
    // need to keep track of corresponding machine node for each swarm node. maps nodes in protocol to nodes in projection
    let mut m_nodes: Vec<NodeId> = vec![NodeId::end(); swarm.node_count()];

    let interested = |edge: ERef| sub.contains(&edge.weight().get_event_type());
    let filtered = EdgeFiltered(swarm, interested);

    // find all nodes that should be in the projection
    let nodes_in_proj: Vec<NodeId> = swarm
        .node_references()
        .filter(|(ni, _)| *ni == initial || filtered.edges_directed(*ni, Incoming).count() > 0)
        .map(|(ni, _)| ni)
        .collect();

    // add the nodes identified above
    for node in nodes_in_proj.iter() {
        m_nodes[node.index()] = machine.add_node(swarm[*node].state_name().clone());
    }

    let find_interesting_edges = |node: NodeId| -> Vec<EdgeReference<'_, SwarmLabel>> {
        let mut stack: Vec<NodeId> = vec![node];
        let mut visited: BTreeSet<NodeId> = BTreeSet::from([node]);
        let mut interesting_edges: Vec<EdgeReference<'_, SwarmLabel>> = vec![];

        while let Some(n) = stack.pop() {
            for edge in swarm.edges_directed(n, Outgoing) {
                if sub.contains(&edge.weight().get_event_type()) {
                    interesting_edges.push(edge);
                } else {
                    if !visited.contains(&edge.target()) {
                        stack.push(edge.target());
                        visited.insert(edge.target());
                    }
                }
            }
        }

        interesting_edges
    };

    for node in nodes_in_proj {
        let interesting_edges: Vec<_> = find_interesting_edges(node);
        for edge in interesting_edges {
            if edge.weight().role == role {
                let execute_label = MachineLabel::Execute {
                    cmd: edge.weight().cmd.clone(),
                    log_type: vec![edge.weight().get_event_type()],
                };
                machine.add_edge(m_nodes[node.index()], m_nodes[node.index()], execute_label);
            }
            let input_label = MachineLabel::Input {
                event_type: edge.weight().get_event_type(),
            };
            machine.add_edge(
                m_nodes[node.index()],
                m_nodes[edge.target().index()],
                input_label,
            );
        }
    }
    //(machine, m_nodes[initial.index()])
    if minimize {
        let (dfa, dfa_initial) = nfa_to_dfa(machine, m_nodes[initial.index()]); // make deterministic. slight deviation from projection operation formally.
        minimal_machine(&dfa, dfa_initial) // when minimizing we get a machine that is a little different but equivalent to the one prescribed by the projection operator formally
    } else {
        (machine, m_nodes[initial.index()])
    }
}

// Map the protocols of a proto_info to a ChainedProtos
fn to_chained_protos(proto_info: &ProtoInfo) -> ChainedProtos {
    let folder = |(acc, roles_prev): (ChainedProtos, BTreeSet<Role>),
                  proto: ProtoStruct|
     -> (ChainedProtos, BTreeSet<Role>) {
        let interfacing_event_types = roles_prev
            .intersection(&proto.roles)
            .flat_map(|role| {
                proto_info
                    .role_event_map
                    .get(role)
                    .unwrap()
                    .iter()
                    .map(|swarm_label| swarm_label.get_event_type())
            })
            .collect();
        let acc = acc
            .into_iter()
            .chain([(proto.graph, proto.initial.unwrap(), interfacing_event_types)])
            .collect();

        (acc,proto.roles.union(&roles_prev).cloned().collect())
    };
    let (chained_protos, _) = proto_info
        .protocols
        .clone()
        .into_iter()
        .fold((vec![], BTreeSet::new()), folder);
    chained_protos
}

// Map a ChainedProtos to a ChainedProjections
fn to_chained_projections(
    chained_protos: ChainedProtos,
    subs: &Subscriptions,
    role: Role,
    minimize: bool,
) -> ChainedProjections {
    let mapper = |(graph, initial, interface)| -> (Graph, NodeId, BTreeSet<EventType>) {
        let (projection, projection_initial) =
            project(&graph, initial, subs, role.clone(), minimize);
        (projection, projection_initial, interface)
    };

    chained_protos.into_iter().map(mapper).collect()
}

// precondition: the protocols interfaces on the supplied interfaces.
// precondition: the composition of the protocols in swarms is wwf w.r.t. subs.
pub fn project_combine(
    proto_info: &ProtoInfo,
    subs: &Subscriptions,
    role: Role,
    minimize: bool,
) -> (OptionGraph, Option<NodeId>) {
    let _span = tracing::info_span!("project_combine", %role).entered();

    let projections = to_chained_projections(to_chained_protos(proto_info), subs, role, minimize);

    match combine_projs(projections, gen_state_name) {
        Some((combined_projection, combined_initial)) =>
        //let (combined_projection, combined_initial) = minimal_machine(&combined_projection, combined_initial);
        // option because used in equivalent. Consider changing.
        {
            (
                to_option_machine(&combined_projection),
                Some(combined_initial),
            )
        }
        None => (OptionGraph::new(), Some(NodeId::end())),
    }
}

fn combine_projs<N: Clone, E: Clone + EventLabel>(
    projections: Vec<(petgraph::Graph<N, E>, NodeId, BTreeSet<EventType>)>,
    gen_node: fn(&N, &N) -> N,
) -> Option<(petgraph::Graph<N, E>, NodeId)> {
    let _span = tracing::info_span!("combine_projs").entered();
    if projections.is_empty() {
        return None;
    }
    let (acc_machine, acc_initial, _) = projections[0].clone();
    let (combined_projection, combined_initial) = projections[1..].to_vec().into_iter().fold(
        (acc_machine, acc_initial),
        |(acc, acc_i), (m, i, interface)| compose(acc, acc_i, m, i, interface, gen_node),
    );
    Some((combined_projection, combined_initial))
}

// nfa to dfa using subset construction. Hopcroft, Motwani and Ullman section 2.3.5
fn nfa_to_dfa(nfa: Graph, i: NodeId) -> (Graph, NodeId) {
    let _span = tracing::info_span!("nfa_to_dfa").entered();
    let mut dfa = Graph::new();
    // maps vectors of NodeIds from the nfa to a NodeId in the new dfa
    let mut dfa_nodes: BTreeMap<BTreeSet<NodeId>, NodeId> = BTreeMap::new();

    // push to and pop from in loop until empty and NFA has been turned into a dfa
    let mut stack: Vec<BTreeSet<NodeId>> = Vec::new();

    // [0, 1, 2] becomes Some(State("{0, 1, 2}"))
    let state_name = |nodes: &BTreeSet<NodeId>| -> State {
        let name = format!("{{ {} }}", nodes.iter().map(|n| nfa[*n].clone()).join(", "));
        State::new(&name)
    };

    // get all outgoing edges of the sources. turn into a map from machine labels to vectors of target states.
    let outgoing_map = |srcs: &BTreeSet<NodeId>| -> BTreeMap<MachineLabel, BTreeSet<NodeId>> {
        srcs.iter()
            .flat_map(|src| {
                nfa.edges_directed(*src, Outgoing)
                    .map(|e| (e.weight().clone(), e.target()))
            })
            .collect::<BTreeSet<(MachineLabel, NodeId)>>()
            .into_iter()
            .fold(BTreeMap::new(), |mut m, (edge_label, target)| {
                m.entry(edge_label)
                    .and_modify(|v: &mut BTreeSet<NodeId>| {
                        v.insert(target);
                    })
                    .or_insert_with(|| BTreeSet::from([target]));
                m
            })
    };

    // add initial state to dfa
    dfa_nodes.insert(
        BTreeSet::from([i]),
        dfa.add_node(state_name(&BTreeSet::from([i]))),
    );
    // add initial state to stack
    stack.push(BTreeSet::from([i]));

    while let Some(states) = stack.pop() {
        let map = outgoing_map(&states);
        for edge in map.keys() {
            if !dfa_nodes.contains_key(&map[edge]) {
                stack.push(map[edge].clone());
            }
            let target: NodeId = *dfa_nodes
                .entry(map[edge].clone())
                .or_insert_with(|| dfa.add_node(state_name(&map[edge])));
            let src: NodeId = *dfa_nodes.get(&states).unwrap();
            dfa.add_edge(src, target, edge.clone());
        }
    }

    (dfa, dfa_nodes[&BTreeSet::from([i])])
}

fn minimal_machine(graph: &Graph, i: NodeId) -> (Graph, NodeId) {
    let _span = tracing::info_span!("minimal_machine").entered();
    let partition = partition_refinement(graph);
    let mut minimal = Graph::new();
    let mut node_to_partition = BTreeMap::new();
    let mut partition_to_minimal_graph_node = BTreeMap::new();
    let mut edges = BTreeSet::new();
    let state_name = |nodes: &BTreeSet<NodeId>| -> State {
        let name = format!(
            "{{ {} }}",
            nodes.iter().map(|n| graph[*n].clone()).join(", ")
        );
        State::new(&name)
    };

    for n in graph.node_indices() {
        node_to_partition.insert(
            n,
            partition.iter().find(|block| block.contains(&n)).unwrap(),
        );
    }

    for block in &partition {
        partition_to_minimal_graph_node.insert(block, minimal.add_node(state_name(block)));
    }
    for node in graph.node_indices() {
        for edge in graph.edges_directed(node, Outgoing) {
            let source = partition_to_minimal_graph_node[node_to_partition[&node]];
            let target = partition_to_minimal_graph_node[node_to_partition[&edge.target()]];
            if !edges.contains(&(source, edge.weight().clone(), target)) {
                minimal.add_edge(source, target, edge.weight().clone());
                edges.insert((source, edge.weight().clone(), target));
            }
        }
    }
    let initial = partition_to_minimal_graph_node[node_to_partition[&i]];
    (minimal, initial)
}

fn partition_refinement(graph: &Graph) -> BTreeSet<BTreeSet<NodeId>> {
    let _span = tracing::info_span!("partition_refinement").entered();
    let mut partition_old = BTreeSet::new();
    let tmp: (BTreeSet<_>, BTreeSet<_>) = graph
        .node_indices()
        .partition(|n| graph.edges_directed(*n, Outgoing).count() == 0);
    let mut partition: BTreeSet<BTreeSet<NodeId>> = BTreeSet::from([tmp.0, tmp.1]);

    let pre_labels = |block: &BTreeSet<NodeId>| -> BTreeSet<MachineLabel> {
        block
            .iter()
            .flat_map(|n| {
                graph
                    .edges_directed(*n, Incoming)
                    .map(|e| e.weight().clone())
            })
            .collect()
    };

    while partition.len() != partition_old.len() {
        partition_old = partition.clone();
        for superblock in &partition_old {
            for label in pre_labels(superblock) {
                partition = refine_partition(graph, partition, superblock, &label);
            }
        }
    }

    partition
}

fn refine_partition(
    graph: &Graph,
    partition: BTreeSet<BTreeSet<NodeId>>,
    superblock: &BTreeSet<NodeId>,
    label: &MachineLabel,
) -> BTreeSet<BTreeSet<NodeId>> {
    partition
        .iter()
        .flat_map(|block| refine_block(graph, block, superblock, label))
        .collect()
}

fn refine_block(
    graph: &Graph,
    block: &BTreeSet<NodeId>,
    superblock: &BTreeSet<NodeId>,
    label: &MachineLabel,
) -> BTreeSet<BTreeSet<NodeId>> {
    let predicate = |node: &NodeId| -> bool {
        graph
            .edges_directed(*node, Outgoing)
            .any(|e| *e.weight() == *label && superblock.contains(&e.target()))
    };

    let tmp: (BTreeSet<_>, BTreeSet<_>) = block.iter().partition(|n| predicate(n));

    BTreeSet::from([tmp.0, tmp.1])
        .into_iter()
        .filter(|s| !s.is_empty())
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
                .contains(&unord_event_pair(e.weight().get_event_type(), et.clone()))
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

pub fn paths_from_event_types(proj: &OptionGraph, proto_info: &ProtoInfo) -> BranchMap {
    let _span = tracing::info_span!("paths_from_event_types").entered();
    let mut m: BTreeMap<EventType, BTreeSet<EventType>> = BTreeMap::new();
    let special_events = get_branching_joining_proto_info(proto_info);

    // The reason for making set of concurrent events smaller is?
    let after_pairs: BTreeSet<UnordEventPair> =
        transitive_closure_succeeding(proto_info.succeeding_events.clone())
            .into_iter()
            .map(|(e, es)| {
                [e].into_iter()
                    .cartesian_product(&es)
                    .map(|(e1, e2)| unord_event_pair(e1, e2.clone()))
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

// precondition: both machines are projected from wwf protocols?
// precondition: m1 and m2 subscribe to all events in interface? Sort of works without but not really?
// takes type parameters to make it work for machines and protocols.
pub(in crate::composition) fn compose<N, E: EventLabel>(
    m1: petgraph::Graph<N, E>,
    i1: NodeId,
    m2: petgraph::Graph<N, E>,
    i2: NodeId,
    interface: BTreeSet<EventType>,
    gen_node: fn(&N, &N) -> N,
) -> (petgraph::Graph<N, E>, NodeId) {
    let _span = tracing::info_span!("compose").entered();
    let mut machine = petgraph::Graph::<N, E>::new();
    let mut node_map: BTreeMap<(NodeId, NodeId), NodeId> = BTreeMap::new();

    let weight_target_mapper = |e: EdgeReference<'_, E>| (e.weight().clone(), e.target());

    let outgoing_map = |m: &petgraph::Graph<N, E>, src: NodeId| -> BTreeMap<E, NodeId> {
        m.edges_directed(src, Outgoing)
            .map(weight_target_mapper)
            .collect()
    };

    // take the outgoing edges of a node an split into two vectors: one for the edges involving interfacing events and one for the edges that do not
    let partitioned = |m: &petgraph::Graph<N, E>, node: NodeId| -> (Vec<E>, Vec<E>) {
        m.edges_directed(node, Outgoing)
            .map(|e| e.weight().clone())
            .partition(|e| interface.contains(&e.get_event_type()))
    };

    let outgoing_to_visit = |m1: &petgraph::Graph<N, E>,
                             s1: NodeId,
                             m2: &petgraph::Graph<N, E>,
                             s2: NodeId|
     -> Vec<E> {
        let (interfacing1, non_interfacing1) = partitioned(m1, s1);
        let (interfacing2, non_interfacing2) = partitioned(m2, s2);

        let interfacing_in_both: Vec<E> = interfacing1
            .iter()
            .cloned()
            .collect::<BTreeSet<E>>()
            .intersection(&interfacing2.iter().cloned().collect::<BTreeSet<E>>())
            .cloned()
            .collect();
        vec![non_interfacing1, non_interfacing2, interfacing_in_both]
            .into_iter()
            .flatten()
            .collect()
    };

    let combined_initial = machine.add_node(gen_node(&m1[i1], &m2[i2]));
    node_map.insert((i1, i2), combined_initial);
    let mut worklist = vec![(combined_initial, (i1, i2))];

    while let Some((src, (old_src1, old_src2))) = worklist.pop() {
        let map1 = outgoing_map(&m1, old_src1);
        let map2 = outgoing_map(&m2, old_src2);
        let outgoing_edges = outgoing_to_visit(&m1, old_src1, &m2, old_src2);

        // add all outgoing edges from src node. only visit edges that are not interfacing or interfacing and both outgoing of old_src1 and old_src2
        // if a edge leads to a node that does not exist yet, create the node.
        for e in outgoing_edges {
            let (dst1, dst2) = match (map1.get(&e), map2.get(&e)) {
                (Some(e1), Some(e2)) => (*e1, *e2),
                (Some(e1), None) => (*e1, old_src2),
                (None, Some(e2)) => (old_src1, *e2),
                _ => unimplemented!(),
            };
            if node_map.contains_key(&(dst1, dst2)) {
                let dst = node_map.get(&(dst1, dst2)).unwrap();
                machine.add_edge(src, *dst, e);
            } else {
                let new_dst = machine.add_node(gen_node(&m1[dst1], &m2[dst2]));
                machine.add_edge(src, new_dst, e);
                node_map.insert((dst1, dst2), new_dst);
                worklist.push((new_dst, (dst1, dst2)));
            }
        }
    }

    (machine, combined_initial)
}

pub fn gen_state_name<N: StateName + From<String>>(n1: &N, n2: &N) -> N {
    let name = format!("{} || {}", n1.state_name(), n2.state_name());
    N::from(name)
}

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
        to_chained_projections(to_chained_protos(proto_info), subs, role, minimize)
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
        compose(
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

    match combine_projs(projections, gen_node) {
        Some((combined_projection, combined_initial)) => {
            Some((combined_projection, Some(combined_initial)))
        } // should we minimize here? not done to keep original shape of input machine as much as possible?
        None => None,
    }
}

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
    let special_event_types = get_branching_joining_proto_info(&proto_info);

    Some(ProjectionInfo {
        projection: from_option_to_machine(proj, proj_initial),
        branches,
        special_event_types,
        proj_to_machine_states,
    })
}

pub(in crate::composition) fn to_option_machine(graph: &Graph) -> OptionGraph {
    graph.map(|_, n| Some(n.state_name().clone()), |_, x| x.clone())
}

pub(in crate::composition) fn from_option_graph_to_graph(graph: &OptionGraph) -> Graph {
    graph.map(
        |_, n| n.clone().unwrap_or_else(|| State::new("")),
        |_, x| x.clone(),
    )
}

fn from_adaptation_graph_to_option_graph(graph: &AdaptationGraph) -> OptionGraph {
    graph.map(|_, n| Some(n.state.state_name().clone()), |_, x| x.clone())
}

//from_adaption_to_machine
pub fn to_json_machine(graph: Graph, initial: NodeId) -> MachineType {
    let _span = tracing::info_span!("to_json_machine").entered();
    let machine_label_mapper = |m: &Graph, eref: EdgeReference<'_, MachineLabel>| {
        let label = eref.weight().clone();
        let source = m[eref.source()].clone();
        let target = m[eref.target()].clone();
        Transition {
            label,
            source,
            target,
        }
    };

    let transitions: Vec<_> = graph
        .edge_references()
        .map(|e| machine_label_mapper(&graph, e))
        .collect();

    MachineType {
        initial: graph[initial].clone(),
        transitions,
    }
}

pub fn from_option_to_machine(
    graph: petgraph::Graph<Option<State>, MachineLabel>,
    initial: NodeId,
) -> MachineType {
    let _span = tracing::info_span!("from_option_to_machine").entered();
    let machine_label_mapper =
        |m: &petgraph::Graph<Option<State>, MachineLabel>,
         eref: EdgeReference<'_, MachineLabel>| {
            let label = eref.weight().clone();
            let source = m[eref.source()].clone().unwrap_or(State::from(""));
            let target = m[eref.target()].clone().unwrap_or(State::from(""));
            Transition {
                label,
                source,
                target,
            }
        };

    let transitions: Vec<_> = graph
        .edge_references()
        .map(|e| machine_label_mapper(&graph, e))
        .collect();

    MachineType {
        initial: graph[initial].clone().unwrap_or(State::from("")),
        transitions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        composition::{
            composition_swarm::{
                compose_protocols, exact_well_formed_sub, from_json,
                overapprox_well_formed_sub, swarms_to_proto_info,
            },
            composition_types::{Granularity, InterfacingProtocols},
        },
        machine::{self},
        types::{Command, EventType, Role, Transition},
        MachineType, Subscriptions, SwarmProtocolType,
    };
    use tracing_subscriber::{fmt, fmt::format::FmtSpan, EnvFilter};

    fn setup_logger() {
        fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
            .try_init()
            .ok();
    }

    pub(in crate::composition) fn from_option_machine(graph: &OptionGraph) -> Graph {
        graph.map(
            |_, n| n.clone().unwrap().state_name().clone(),
            |_, x| x.clone(),
        )
    }
    fn from_adaptation_graph_to_graph(graph: &AdaptationGraph) -> Graph {
        graph.map(|_, n| n.state.state_name().clone(), |_, x| x.clone())
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
    fn get_proto_4() -> SwarmProtocolType {
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

    fn get_interfacing_swarms_3() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto1(), get_proto2(), get_proto_4()])
    }

    fn get_interfacing_swarms_warehouse() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto1()])
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

    mod projection_tests {
        use super::*;
            #[test]
        fn test_projection_1() {
            setup_logger();
            // From Combining Swarm Protocols, example 5.
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
            // contains superfluous subscriptions, but to match example in article
            let subs = serde_json::from_str::<Subscriptions>(
                r#"{
                "C":["tireID","position","tire","car"],
                "W":["tireID","position","tire"],
                "F":["tireID","tire","car"]
            }"#,
            )
            .unwrap();

            let role = Role::new("F");
            let (g, i, _) = from_json(proto);
            let (proj, proj_initial) = project(&g, i.unwrap(), &subs, role, false);
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
                &to_option_machine(&proj),
                proj_initial
            )
            .is_empty());
        }

        #[test]
        fn test_projection_2() {
            setup_logger();
            // warehouse example from coplaws slides
            let proto = get_proto1();
            let result_subs =
                exact_well_formed_sub(InterfacingProtocols(vec![proto.clone()]), &BTreeMap::new());
            assert!(result_subs.is_ok());
            let subs = result_subs.unwrap();
            let role = Role::new("FL");
            let (g, i, _) = from_json(proto);
            let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone(), false);
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

            let errors = equivalent(
                &to_option_machine(&left),
                left_initial,
                &right,
                right_initial.unwrap(),
            );
            assert!(errors.is_empty());
        }

        #[test]
        fn test_projection_3() {
            setup_logger();
            // car factory from coplaws example
            let proto = get_proto2();
            let result_subs =
                exact_well_formed_sub(InterfacingProtocols(vec![proto.clone()]), &BTreeMap::new());
            assert!(result_subs.is_ok());
            let subs = result_subs.unwrap();
            let role = Role::new("F");
            let (g, i, _) = from_json(proto);
            let (proj, proj_initial) = project(&g, i.unwrap(), &subs, role, false);
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
                &to_option_machine(&proj),
                proj_initial
            )
            .is_empty());
        }

        #[test]
        fn test_projection_4() {
            setup_logger();
            // car factory from coplaws example
            let protos = get_interfacing_swarms_1();
            let result_subs = overapprox_well_formed_sub(
                protos.clone(),
                &BTreeMap::from([(Role::new("T"), BTreeSet::from([EventType::new("car")]))]),
                Granularity::Coarse,
            );
            assert!(result_subs.is_ok());
            let subs = result_subs.unwrap();

            let role = Role::new("T");
            let (g, i) = compose_protocols(protos).unwrap();
            let (proj, proj_initial) = project(&g, i, &subs, role, false);
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
                &to_option_machine(&proj),
                proj_initial
            )
            .is_empty());
        }

        #[test]
        fn test_projection_fail_1() {
            setup_logger();
            // warehouse example from coplaws slides
            let proto = get_proto1();
            let result_subs =
                exact_well_formed_sub(InterfacingProtocols(vec![proto.clone()]), &BTreeMap::new());
            assert!(result_subs.is_ok());
            let subs = result_subs.unwrap();
            let role = Role::new("FL");
            let (g, i, _) = from_json(proto);
            let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone(), false);
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

            let errors = equivalent(
                &to_option_machine(&left),
                left_initial,
                &right,
                right_initial.unwrap(),
            );
            assert!(!errors.is_empty());
        }
        #[test]
        fn test_projection_fail_2() {
            setup_logger();
            // warehouse example from coplaws slides
            let proto = get_proto1();
            let result_subs =
                exact_well_formed_sub(InterfacingProtocols(vec![proto.clone()]), &BTreeMap::new());
            assert!(result_subs.is_ok());
            let subs = result_subs.unwrap();
            let role = Role::new("FL");
            let (g, i, _) = from_json(proto);
            let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone(), false);
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

            let errors = equivalent(
                &to_option_machine(&left),
                left_initial,
                &right,
                right_initial.unwrap(),
            );
            assert!(!errors.is_empty());
        }
        #[test]
        fn test_projection_fail_3() {
            setup_logger();
            // warehouse example from coplaws slides
            let proto = get_proto1();
            let result_subs =
                exact_well_formed_sub(InterfacingProtocols(vec![proto.clone()]), &BTreeMap::new());
            assert!(result_subs.is_ok());
            let subs = result_subs.unwrap();
            let role = Role::new("FL");
            let (g, i, _) = from_json(proto);
            let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone(), false);
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

            let errors = equivalent(
                &to_option_machine(&left),
                left_initial,
                &right,
                right_initial.unwrap(),
            );
            assert!(!errors.is_empty());
        }
        #[test]
        fn test_projection_fail_4() {
            setup_logger();
            // warehouse example from coplaws slides
            let proto = get_proto1();
            let result_subs =
                exact_well_formed_sub(InterfacingProtocols(vec![proto.clone()]), &BTreeMap::new());
            assert!(result_subs.is_ok());
            let subs = result_subs.unwrap();
            let role = Role::new("FL");
            let (g, i, _) = from_json(proto);
            let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone(), false);
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

            let errors = equivalent(
                &to_option_machine(&left),
                left_initial,
                &right,
                right_initial.unwrap(),
            );
            assert!(!errors.is_empty());
        }
    }

    mod machine_composition_tests {
        use super::*;

        #[test]
        fn test_combine_machines_1() {
            setup_logger();
            // Example from coplaws slides. Use generated WWF subscriptions. Project over T.
            let role = Role::new("T");
            let subs1 = crate::composition::composition_swarm::overapprox_well_formed_sub(
                get_interfacing_swarms_1(),
                &BTreeMap::new(),
                Granularity::Coarse,
            );
            assert!(subs1.is_ok());
            let subs1 = subs1.unwrap();
            let proto_info = swarms_to_proto_info(get_interfacing_swarms_1());
            assert!(proto_info.no_errors());

            let (proj_combined1, proj_combined_initial1) =
                project_combine(&proto_info, &subs1, role.clone(), false);

            let subs2 = crate::composition::composition_swarm::overapprox_well_formed_sub(
                get_interfacing_swarms_1_reversed(),
                &BTreeMap::new(),
                Granularity::Coarse,
            );
            assert!(subs2.is_ok());
            let subs2 = subs2.unwrap();
            let proto_info = swarms_to_proto_info(get_interfacing_swarms_1_reversed());
            assert!(proto_info.no_errors());

            let (proj_combined2, proj_combined_initial2) =
                project_combine(&proto_info, &subs2, role.clone(), false);

            // compose(a, b) should be equal to compose(b, a)
            assert_eq!(subs1, subs2);
            assert!(equivalent(
                &proj_combined1,
                proj_combined_initial1.unwrap(),
                &proj_combined2,
                proj_combined_initial2.unwrap()
            )
            .is_empty());

            let composition = compose_protocols(get_interfacing_swarms_1());
            assert!(composition.is_ok());
            let (composed_graph, composed_initial) = composition.unwrap();
            let (proj, proj_initial) = project(
                &composed_graph,
                composed_initial,
                &subs1,
                role.clone(),
                true,
            );

            assert!(equivalent(
                &proj_combined2,
                proj_combined_initial2.unwrap(),
                &to_option_machine(&proj),
                proj_initial
            )
            .is_empty());
        }

        #[test]
        fn test_combine_machines_2() {
            setup_logger();
            // fails when you use the exact subscriptions because that way not all roles subscribe to ALL interfaces? Ordering gets messed up.
            // the projected over the explicit composition may be correct, but the combined projections look weird and out of order.
            let composition = compose_protocols(get_interfacing_swarms_2());
            assert!(composition.is_ok());
            let (composed_graph, composed_initial) = composition.unwrap();
            let subs = crate::composition::composition_swarm::overapprox_well_formed_sub(
                get_interfacing_swarms_2(),
                &BTreeMap::new(),
                Granularity::Coarse,
            );
            assert!(subs.is_ok());
            let subs = subs.unwrap();
            let all_roles = vec![
                Role::new("T"),
                Role::new("FL"),
                Role::new("D"),
                Role::new("F"),
                Role::new("TR"),
                Role::new("QCR"),
            ];

            for role in all_roles {
                let subs1 = crate::composition::composition_swarm::overapprox_well_formed_sub(
                    get_interfacing_swarms_2(),
                    &BTreeMap::new(),
                    Granularity::Coarse,
                );
                assert!(subs1.is_ok());
                let subs1 = subs1.unwrap();
                let proto_info = swarms_to_proto_info(get_interfacing_swarms_2());
                assert!(proto_info.no_errors());

                let (proj_combined1, proj_combined_initial1) =
                    project_combine(&proto_info, &subs1, role.clone(), false);

                let subs2 = crate::composition::composition_swarm::overapprox_well_formed_sub(
                    get_interfacing_swarms_2_reversed(),
                    &BTreeMap::new(),
                    Granularity::Coarse,
                );
                assert!(subs2.is_ok());
                let subs2 = subs2.unwrap();
                let proto_info = swarms_to_proto_info(get_interfacing_swarms_2_reversed());
                assert!(proto_info.no_errors());

                let (proj_combined2, proj_combined_initial2) =
                    project_combine(&proto_info, &subs2, role.clone(), false);

                // compose(a, b) should be equal to compose(b, a)
                assert_eq!(subs1, subs2);
                assert!(equivalent(
                    &proj_combined1,
                    proj_combined_initial1.unwrap(),
                    &proj_combined2,
                    proj_combined_initial2.unwrap()
                )
                .is_empty());
                assert_eq!(subs2, subs);

                let (proj, proj_initial) =
                    project(&composed_graph, composed_initial, &subs, role.clone(), true);
                let errors = equivalent(
                    &proj_combined2,
                    proj_combined_initial2.unwrap(),
                    &to_option_machine(&proj),
                    proj_initial,
                );

                assert!(errors.is_empty());
            }
        }

        #[test]
        fn test_all_projs_whf() {
            setup_logger();
            let composition = compose_protocols(get_interfacing_swarms_1());
            assert!(composition.is_ok());
            let (composed_graph, composed_initial) = composition.unwrap();
            let subs = crate::composition::composition_swarm::overapprox_well_formed_sub(
                get_interfacing_swarms_1(),
                &BTreeMap::new(),
                Granularity::TwoStep
            );
            assert!(subs.is_ok());
            let subs = subs.unwrap();

            let all_roles = vec![
                Role::new("T"),
                Role::new("FL"),
                Role::new("D"),
                Role::new("F"),
            ];

            let expected_projs = BTreeMap::from([
                (Role::new("T"), get_whf_transport()),
                (Role::new("FL"), get_whf_forklift()),
                (Role::new("D"), get_whf_door()),
                (Role::new("F"), get_whf_f()),
            ]);

            for role in all_roles {
                let (expand_proj, expand_proj_initial) = project(
                    &composed_graph,
                    composed_initial,
                    &subs,
                    role.clone(),
                    true,
                );
                let (combined_proj, combined_proj_initial) =
                    project_combine(&swarms_to_proto_info(get_interfacing_swarms_1()), &subs, role.clone(), true);

                assert!(equivalent(
                    &to_option_machine(&expand_proj),
                    expand_proj_initial,
                    &combined_proj,
                    combined_proj_initial.unwrap())
                    .is_empty()
                );

                let (expected, expected_initial, _) = crate::machine::from_json(expected_projs.get(&role).unwrap().clone());

                assert!(equivalent(
                    &expected,
                    expected_initial.unwrap(),
                    &combined_proj,
                    combined_proj_initial.unwrap())
                    .is_empty());
            }
        }

        #[test]
        fn test_compose_zero() {
            let left = MachineType {
                initial: State::new("left_0"),
                transitions: vec![
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("a"),
                        },
                        source: State::new("left_0"),
                        target: State::new("left_1"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("cmd_a"),
                            log_type: vec![EventType::new("a")],
                        },
                        source: State::new("left_0"),
                        target: State::new("left_0"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("b"),
                        },
                        source: State::new("left_1"),
                        target: State::new("left_2"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("cmd_b"),
                            log_type: vec![EventType::new("b")],
                        },
                        source: State::new("left_1"),
                        target: State::new("left_1"),
                    },
                ],
            };
            let right = MachineType {
                initial: State::new("right_0"),
                transitions: vec![
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("b"),
                        },
                        source: State::new("right_0"),
                        target: State::new("right_1"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("cmd_b"),
                            log_type: vec![EventType::new("b")],
                        },
                        source: State::new("right_0"),
                        target: State::new("right_0"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("a"),
                        },
                        source: State::new("right_1"),
                        target: State::new("right_2"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("cmd_a"),
                            log_type: vec![EventType::new("a")],
                        },
                        source: State::new("right_1"),
                        target: State::new("right_1"),
                    },
                ],
            };
            let (left, left_initial, _) = crate::machine::from_json(left);
            let left = from_option_graph_to_graph(&left);
            let (right, right_initial, _) = crate::machine::from_json(right);
            let right = from_option_graph_to_graph(&right);
            let interface = BTreeSet::from([EventType::new("a"), EventType::new("b")]);
            let (combined, combined_initial) = compose(
                right,
                right_initial.unwrap(),
                left,
                left_initial.unwrap(),
                interface,
                gen_state_name,
            );
            let combined = to_json_machine(combined, combined_initial);

            let expected = MachineType {
                initial: State::new("right_0 || left_0"),
                transitions: vec![],
            };

            assert_eq!(combined, expected);
        }
    }

    // TODO:
    // Move tests related to adaptation and adaptation info to a module. Make one more (one that currently just prints).
    // Add a test somewhere that uses WH || F || QC

    #[test]
    fn test_example_from_text_machine() {
        setup_logger();
        let role = Role::new("F");
        let subs = crate::composition::composition_swarm::overapprox_well_formed_sub(
            get_interfacing_swarms_3(),
            &BTreeMap::new(),
            Granularity::Medium,
        );
        assert!(subs.is_ok());
        let subs = subs.unwrap();
        let proto_info = swarms_to_proto_info(get_interfacing_swarms_3());
        assert!(proto_info.no_errors());
        let (proj, proj_initial) = project_combine(&proto_info, &subs, role.clone(), false);
        println!(
            "projection of {}: {}",
            role.to_string(),
            serde_json::to_string_pretty(&from_option_to_machine(proj, proj_initial.unwrap()))
                .unwrap()
        );
    }

    #[test]
    #[ignore]
    fn combine_with_self() {
        setup_logger();

        let proto = get_proto1();
        let result_subs = overapprox_well_formed_sub(
            InterfacingProtocols(vec![proto.clone(), get_proto2()]),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(result_subs.is_ok());
        let subs = result_subs.unwrap();
        println!("subs: {}", serde_json::to_string_pretty(&subs).unwrap());
        let role = Role::new("FL");
        let (g, i, _) = from_json(proto);
        let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone(), false);
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
        let right_option = to_option_machine(&right);

        println!(
            "left {:?}: {}",
            role.clone(),
            serde_json::to_string_pretty(&to_json_machine(left.clone(), left_initial)).unwrap()
        );
        println!(
            "right {:?}: {}",
            role,
            serde_json::to_string_pretty(&from_option_to_machine(
                right_option.clone(),
                right_initial.unwrap()
            ))
            .unwrap()
        );
        assert!(errors.is_empty());

        /* let errors = equivalent(
            &to_option_machine(&left),
            left_initial,
            &right_option,
            right_initial.unwrap());
        assert!(errors.is_empty());
        let errors: Vec<String> = errors.into_iter().map(crate::machine::Error::convert(&to_option_machine(&left), &right_option)).collect(); */
        println!("{:?}", errors);
        let interface = BTreeSet::from([
            EventType::new("partID"),
            EventType::new("pos"),
            EventType::new("time"),
        ]);
        // right left swapped here on purpose
        let (combined, combined_initial) = compose(
            right,
            right_initial.unwrap(),
            left,
            left_initial,
            interface,
            gen_state_name,
        );
        println!(
            "combined {:?}: {}",
            role.clone(),
            serde_json::to_string_pretty(&to_json_machine(combined.clone(), combined_initial))
                .unwrap()
        );
    }

    #[test]
    #[ignore]
    fn test_adapted_projection_fl() {
        setup_logger();

        let fl_m = MachineType {
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
        let (fl_m_graph, fl_m_graph_initial, _) = crate::machine::from_json(fl_m);

        let role = Role::new("FL");
        let swarms = get_interfacing_swarms_1();
        let subs1 = crate::composition::composition_swarm::overapprox_well_formed_sub(
            swarms.clone(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(subs1.is_ok());
        let subs1 = subs1.unwrap();
        println!("subs: {}", serde_json::to_string_pretty(&subs1).unwrap());
        let proto_info = swarms_to_proto_info(swarms.clone());
        assert!(proto_info.no_errors());

        let adapted = adapted_projection(
            &proto_info,
            &subs1,
            role.clone(),
            (fl_m_graph.clone(), fl_m_graph_initial.unwrap()),
            0,
            true,
        );
        let (adapted_proj, adapted_proj_initial) = adapted.unwrap();
        println!(
            "left {:?}: {}",
            role.clone(),
            serde_json::to_string_pretty(&from_option_to_machine(
                fl_m_graph.clone(),
                fl_m_graph_initial.unwrap()
            ))
            .unwrap()
        );
        println!(
            "right {:?}: {}",
            role,
            serde_json::to_string_pretty(&to_json_machine(
                from_adaptation_graph_to_graph(&adapted_proj.clone()),
                adapted_proj_initial.unwrap()
            ))
            .unwrap()
        );

        let role = Role::new("FL");
        let swarms = get_interfacing_swarms_3();
        let subs2 = crate::composition::composition_swarm::overapprox_well_formed_sub(
            swarms.clone(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(subs2.is_ok());
        let subs2 = subs2.unwrap();
        println!("subs: {}", serde_json::to_string_pretty(&subs2).unwrap());
        let proto_info = swarms_to_proto_info(swarms.clone());
        assert!(proto_info.no_errors());

        //let (adapted_proj, adapted_proj_initial) = adapted_projection(&proto_info.protocols, &subs2, role.clone(), (fl_m_graph.clone(), fl_m_graph_initial.unwrap()), 0);
        let adapted = adapted_projection(
            &proto_info,
            &subs2,
            role.clone(),
            (fl_m_graph.clone(), fl_m_graph_initial.unwrap()),
            0,
            true,
        );
        let (adapted_proj, adapted_proj_initial) = adapted.unwrap();
        println!(
            "left {:?}: {}",
            role.clone(),
            serde_json::to_string_pretty(&from_option_to_machine(
                fl_m_graph.clone(),
                fl_m_graph_initial.unwrap()
            ))
            .unwrap()
        );
        println!(
            "right {:?}: {}",
            role,
            serde_json::to_string_pretty(&to_json_machine(
                from_adaptation_graph_to_graph(&adapted_proj.clone()),
                adapted_proj_initial.unwrap()
            ))
            .unwrap()
        );
    }

    #[test]
    #[ignore]
    fn test_adapted_projection_r() {
        setup_logger();

        let f_m = MachineType {
            initial: State::new("0"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("part"),
                    },
                    source: State::new("0"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("build"),
                        log_type: vec![EventType::new("car")],
                    },
                    source: State::new("1"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("car"),
                    },
                    source: State::new("1"),
                    target: State::new("2"),
                },
            ],
        };
        let (f_m_graph, f_m_graph_initial, _) = crate::machine::from_json(f_m);

        let role = Role::new("F");
        let swarms = get_interfacing_swarms_1();
        let subs1 = crate::composition::composition_swarm::overapprox_well_formed_sub(
            swarms.clone(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(subs1.is_ok());
        let subs1 = subs1.unwrap();
        println!("subs: {}", serde_json::to_string_pretty(&subs1).unwrap());
        let proto_info = swarms_to_proto_info(swarms.clone());
        assert!(proto_info.no_errors());

        let adapted = adapted_projection(
            &proto_info,
            &subs1,
            role.clone(),
            (f_m_graph.clone(), f_m_graph_initial.unwrap()),
            1,
            true,
        );
        let (adapted_proj, adapted_proj_initial) = adapted.unwrap();
        println!(
            "left {:?}: {}",
            role.clone(),
            serde_json::to_string_pretty(&from_option_to_machine(
                f_m_graph.clone(),
                f_m_graph_initial.unwrap()
            ))
            .unwrap()
        );
        println!(
            "right {:?}: {}",
            role,
            serde_json::to_string_pretty(&to_json_machine(
                from_adaptation_graph_to_graph(&adapted_proj.clone()),
                adapted_proj_initial.unwrap()
            ))
            .unwrap()
        );

        let role = Role::new("F");
        let swarms = get_interfacing_swarms_3();
        let subs2 = crate::composition::composition_swarm::overapprox_well_formed_sub(
            swarms.clone(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(subs2.is_ok());
        let subs2 = subs2.unwrap();
        println!("subs: {}", serde_json::to_string_pretty(&subs2).unwrap());
        let proto_info = swarms_to_proto_info(swarms.clone());
        assert!(proto_info.no_errors());

        let adapted = adapted_projection(
            &proto_info,
            &subs2,
            role.clone(),
            (f_m_graph.clone(), f_m_graph_initial.unwrap()),
            1,
            true,
        );
        let (adapted_proj, adapted_proj_initial) = adapted.unwrap();
        println!(
            "left {:?}: {}",
            role.clone(),
            serde_json::to_string_pretty(&from_option_to_machine(
                f_m_graph.clone(),
                f_m_graph_initial.unwrap()
            ))
            .unwrap()
        );
        println!(
            "right {:?}: {}",
            role,
            serde_json::to_string_pretty(&to_json_machine(
                from_adaptation_graph_to_graph(&adapted_proj.clone()),
                adapted_proj_initial.unwrap()
            ))
            .unwrap()
        );
    }

    #[test]
    #[ignore]
    fn test_det_proj_1() {
        setup_logger();
        let composition = compose_protocols(get_interfacing_swarms_1());
        assert!(composition.is_ok());
        let (composed_graph, composed_initial) = composition.unwrap();
        let subs = crate::composition::composition_swarm::exact_well_formed_sub(
            get_interfacing_swarms_1(),
            &BTreeMap::new(),
        );
        assert!(subs.is_ok());
        let subs = subs.unwrap();
        println!(
            "subscription: {}",
            serde_json::to_string_pretty(&subs).unwrap()
        );
        //let all_roles = vec![Role::new("T"), Role::new("FL"), Role::new("D"), Role::new("F")];
        let (proj, proj_initial) = project(
            &composed_graph,
            composed_initial,
            &subs,
            Role::new("D"),
            false,
        );
        println!(
            "{}: {}",
            Role::new("D").to_string(),
            serde_json::to_string_pretty(&to_json_machine(proj, proj_initial)).unwrap()
        );
        /* for role in all_roles {
            let (proj, proj_initial) = project(&composed_graph, composed_initial, &subs, role.clone());
            println!("{}: {}", role.clone().to_string(), serde_json::to_string_pretty(&to_json_machine(proj, proj_initial)).unwrap());
        } */
    }

    #[test]
    #[ignore]
    fn test_det_proj_2() {
        setup_logger();
        let composition = compose_protocols(get_interfacing_swarms_3());
        assert!(composition.is_ok());
        let (composed_graph, composed_initial) = composition.unwrap();
        let subs = crate::composition::composition_swarm::exact_well_formed_sub(
            get_interfacing_swarms_3(),
            &BTreeMap::new(),
        );
        assert!(subs.is_ok());
        let subs = subs.unwrap();
        //println!("subscription: {}", serde_json::to_string_pretty(&subs).unwrap());
        let (proj, proj_initial) = project(
            &composed_graph,
            composed_initial,
            &subs,
            Role::new("D"),
            false,
        );
        println!(
            "{}: {}",
            Role::new("D").to_string(),
            serde_json::to_string_pretty(&to_json_machine(proj, proj_initial)).unwrap()
        );
        /* let all_roles = vec![Role::new("T"), Role::new("FL"), Role::new("D"), Role::new("F"), Role::new("QCR")];
        for role in all_roles {
            let (proj, proj_initial) = project(&composed_graph, composed_initial, &subs, role.clone());
            //println!("{}: {}", role.clone().to_string(), serde_json::to_string_pretty(&to_json_machine(proj, proj_initial)).unwrap());
            println!("{}\n$$$$\n", serde_json::to_string_pretty(&to_json_machine(proj, proj_initial)).unwrap());
        } */
    }

    #[test]
    #[ignore]
    fn test_det_proj_3() {
        setup_logger();
        let composition = compose_protocols(get_interfacing_swarms_warehouse());
        assert!(composition.is_ok());
        let (composed_graph, composed_initial) = composition.unwrap();
        let subs = crate::composition::composition_swarm::exact_well_formed_sub(
            get_interfacing_swarms_warehouse(),
            &BTreeMap::new(),
        );
        assert!(subs.is_ok());
        let subs = subs.unwrap();
        //println!("subscription: {}", serde_json::to_string_pretty(&subs).unwrap());
        let all_roles = vec![Role::new("T"), Role::new("FL"), Role::new("D")];

        for role in all_roles {
            let (proj, proj_initial) = project(
                &composed_graph,
                composed_initial,
                &subs,
                role.clone(),
                false,
            );
            //println!("{}: {}", role.clone().to_string(), serde_json::to_string_pretty(&to_json_machine(proj, proj_initial)).unwrap());
            println!(
                "{}\n$$$$\n",
                serde_json::to_string_pretty(&to_json_machine(proj, proj_initial)).unwrap()
            );
        }
    }

    #[test]
    fn test_projection_information_1() {
        setup_logger();

        let fl_m = MachineType {
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

        let expected_proj = MachineType {
            initial: State::new("0 || { { 0 } } || { { 0 } }"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0 || { { 0 } } || { { 0 } }"),
                    target: State::new("3 || { { 3 } } || { { 0 } }"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("0 || { { 0 } } || { { 0 } }"),
                    target: State::new("1 || { { 1 } } || { { 1 } }"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("pos"),
                    },
                    source: State::new("1 || { { 1 } } || { { 1 } }"),
                    target: State::new("2 || { { 2 } } || { { 1 } }"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("get"),
                        log_type: vec![EventType::new("pos")],
                    },
                    source: State::new("1 || { { 1 } } || { { 1 } }"),
                    target: State::new("1 || { { 1 } } || { { 1 } }"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("part"),
                    },
                    source: State::new("2 || { { 2 } } || { { 1 } }"),
                    target: State::new("2 || { { 0 } } || { { 2 } }"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("2 || { { 0 } } || { { 2 } }"),
                    target: State::new("3 || { { 3 } } || { { 2 } }"),
                },
            ],
        };

        let (fl_m_graph, fl_m_graph_initial, _) = crate::machine::from_json(fl_m);
        let role = Role::new("FL");
        let swarms = get_interfacing_swarms_1();
        let subs1 = crate::composition::composition_swarm::overapprox_well_formed_sub(
            swarms.clone(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(subs1.is_ok());
        let subs1 = subs1.unwrap();
        //println!("subs: {}", serde_json::to_string_pretty(&subs1).unwrap());
        let proto_info = swarms_to_proto_info(swarms.clone());

        let projection_info = projection_information(
            &proto_info,
            &subs1,
            role,
            (fl_m_graph.clone(), fl_m_graph_initial.unwrap()),
            0,
            true,
        );
        let projection_info = match projection_info {
            None => panic!(),
            Some(projection_info) => {
                //println!("proj: {}", serde_json::to_string_pretty(&projection_info.projection).unwrap());
                //println!("map: {}", serde_json::to_string_pretty(&projection_info.proj_to_machine_states).unwrap());
                //println!("branches: {}", serde_json::to_string_pretty(&projection_info.branches).unwrap());
                //println!("special event types: {}", serde_json::to_string_pretty(&projection_info.special_event_types).unwrap());
                projection_info
            }
        };
        let (actual_graph, actual_initial, _) = machine::from_json(projection_info.projection);
        let (expected_graph, expected_initial, _) = crate::machine::from_json(expected_proj);
        let expected_proj_to_machine_states = BTreeMap::from([
            (
                State::new("(0 || { { 0 } }) || { { 0 } }"),
                vec![State::new("0")],
            ),
            (
                State::new("(1 || { { 1 } }) || { { 1 } }"),
                vec![State::new("1")],
            ),
            (
                State::new("(2 || { { 0 } }) || { { 2 } }"),
                vec![State::new("2")],
            ),
            (
                State::new("(2 || { { 2 } }) || { { 1 } }"),
                vec![State::new("2")],
            ),
            (
                State::new("(3 || { { 3 } }) || { { 0 } }"),
                vec![State::new("3")],
            ),
            (
                State::new("(3 || { { 3 } }) || { { 2 } }"),
                vec![State::new("3")],
            ),
        ]);
        let expected_branches = BTreeMap::from([
            (EventType::new("part"), vec![EventType::new("time")]),
            (
                EventType::new("partID"),
                vec![
                    EventType::new("part"),
                    EventType::new("pos"),
                    EventType::new("time"),
                ],
            ),
            (
                EventType::new("pos"),
                vec![EventType::new("part"), EventType::new("time")],
            ),
            (EventType::new("time"), vec![]),
        ]);
        let expected_special_event_types =
            BTreeSet::from([EventType::new("partID"), EventType::new("time")]);
        let errors = equivalent(
            &expected_graph,
            expected_initial.unwrap(),
            &actual_graph,
            actual_initial.unwrap(),
        );
        let is_empty = errors.is_empty();
        //println!("{:?}", errors.map(machine::Error::convert(&expected_graph, &actual_graph)));
        assert!(is_empty);
        assert_eq!(
            expected_proj_to_machine_states,
            projection_info.proj_to_machine_states
        );
        assert_eq!(expected_branches, projection_info.branches);
        assert_eq!(
            expected_special_event_types,
            projection_info.special_event_types
        );
    }
    #[test]
    fn test_projection_information_2() {
        setup_logger();

        let fl_m = MachineType {
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
                    target: State::new("0"),
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

        let expected_proj = MachineType {
            initial: State::new("0 || { { 0 } }"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0 || { { 0 } }"),
                    target: State::new("3 || { { 3 } }"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("0 || { { 0 } }"),
                    target: State::new("1 || { { 1 } }"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("pos"),
                    },
                    source: State::new("1 || { { 1 } }"),
                    target: State::new("0 || { { 2 } }"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("get"),
                        log_type: vec![EventType::new("pos")],
                    },
                    source: State::new("1 || { { 1 } }"),
                    target: State::new("1 || { { 1 } }"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("part"),
                    },
                    source: State::new("0 || { { 2 } }"),
                    target: State::new("0 || { { 0 } }"),
                },
            ],
        };

        let (fl_m_graph, fl_m_graph_initial, _) = crate::machine::from_json(fl_m.clone());
        let role = Role::new("FL");
        let swarms: InterfacingProtocols = InterfacingProtocols(vec![get_proto1()]);
        let swarms_for_sub = get_interfacing_swarms_1();
        let larger_than_necessary_sub =
            crate::composition::composition_swarm::overapprox_well_formed_sub(
                swarms_for_sub,
                &BTreeMap::new(),
                Granularity::TwoStep,
            );
        assert!(larger_than_necessary_sub.is_ok());
        let subs1 = larger_than_necessary_sub.unwrap();
        //println!("subs: {}", serde_json::to_string_pretty(&subs1).unwrap());
        let proto_info = swarms_to_proto_info(swarms.clone());

        let projection_info = projection_information(
            &proto_info,
            &subs1,
            role,
            (fl_m_graph.clone(), fl_m_graph_initial.unwrap()),
            0,
            true,
        );
        let projection_info = match projection_info {
            None => panic!(),
            Some(projection_info) => {
                /* println!("proj: {}", serde_json::to_string_pretty(&projection_info.projection).unwrap());
                println!("fl_m: {}", serde_json::to_string_pretty(&fl_m).unwrap());
                println!("map: {}", serde_json::to_string_pretty(&projection_info.proj_to_machine_states).unwrap());
                println!("branches: {}", serde_json::to_string_pretty(&projection_info.branches).unwrap());
                println!("special event types: {}", serde_json::to_string_pretty(&projection_info.special_event_types).unwrap()); */
                projection_info
            }
        };
        let (actual_graph, actual_initial, _) = machine::from_json(projection_info.projection);
        let (expected_graph, expected_initial, _) = crate::machine::from_json(expected_proj);
        let expected_proj_to_machine_states = BTreeMap::from([
            (State::new("(0 || { { 0 } })"), vec![State::new("0")]),
            (State::new("(0 || { { 2 } })"), vec![State::new("0")]),
            (State::new("(1 || { { 1 } })"), vec![State::new("1")]),
            (State::new("(3 || { { 3 } })"), vec![State::new("3")]),
        ]);
        let expected_branches = BTreeMap::from([
            (
                EventType::new("part"),
                vec![EventType::new("partID"), EventType::new("time")],
            ),
            (
                EventType::new("partID"),
                vec![
                    EventType::new("part"),
                    EventType::new("partID"),
                    EventType::new("pos"),
                    EventType::new("time"),
                ],
            ),
            (
                EventType::new("pos"),
                vec![
                    EventType::new("part"),
                    EventType::new("partID"),
                    EventType::new("time"),
                ],
            ),
            (EventType::new("time"), vec![]),
        ]);
        let expected_special_event_types =
            BTreeSet::from([EventType::new("partID"), EventType::new("time")]);
        let errors = equivalent(
            &expected_graph,
            expected_initial.unwrap(),
            &actual_graph,
            actual_initial.unwrap(),
        );
        let is_empty = errors.is_empty();
        //println!("{:?}", errors.map(machine::Error::convert(&expected_graph, &actual_graph)));
        assert!(is_empty);
        assert_eq!(
            expected_proj_to_machine_states,
            projection_info.proj_to_machine_states
        );
        assert_eq!(expected_branches, projection_info.branches);
        assert_eq!(
            expected_special_event_types,
            projection_info.special_event_types
        );
    }

    mod big_example_i_can_be_deleted {
        use crate::types::DataResult;

        use super::*;

        fn get_steel_press_proto() -> SwarmProtocolType {
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        {
                        "source": "0",
                        "target": "1",
                        "label": {
                            "cmd": "pickUpSteelRoll",
                            "role": "SteelTransport",
                            "logType": [
                            "SteelRoll"
                            ]
                        }
                        },
                        {
                        "source": "1",
                        "target": "2",
                        "label": {
                            "cmd": "pressSteel",
                            "role": "Stamp",
                            "logType": [
                            "SteelParts"
                            ]
                        }
                        },
                        {
                        "source": "2",
                        "target": "0",
                        "label": {
                            "cmd": "assembleBody",
                            "role": "BodyAssembler",
                            "logType": [
                            "PartialCarBody"
                            ]
                        }
                        },
                        {
                        "source": "0",
                        "target": "3",
                        "label": {
                            "cmd": "carBodyDone",
                            "role": "CarBodyChecker",
                            "logType": [
                            "CarBody"
                            ]
                        }
                        }
                    ]
                }"#,
            )
            .unwrap()
        }

        fn get_paint_proto() -> SwarmProtocolType {
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        {
                        "source": "0",
                        "target": "1",
                        "label": {
                            "cmd": "carBodyDone",
                            "role": "CarBodyChecker",
                            "logType": [
                            "CarBody"
                            ]
                        }
                        },
                        {
                        "source": "1",
                        "target": "2",
                        "label": {
                            "cmd": "applyPaint",
                            "role": "Painter",
                            "logType": [
                            "PaintedCarBody"
                            ]
                        }
                        }
                    ]
                    }"#,
            )
            .unwrap()
        }
        fn get_engine_installation_proto() -> SwarmProtocolType {
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        {
                        "source": "0",
                        "target": "1",
                        "label": {
                            "cmd": "applyPaint",
                            "role": "Painter",
                            "logType": [
                            "PaintedCarBody"
                            ]
                        }
                        },
                        {
                        "source": "1",
                        "target": "2",
                        "label": {
                            "cmd": "requestEngine",
                            "role": "EngineInstaller",
                            "logType": [
                            "RequestEngine"
                            ]
                        }
                        },
                        {
                        "source": "2",
                        "target": "3",
                        "label": {
                            "cmd": "request",
                            "role": "Warehouse",
                            "logType": [
                            "ItemRequest"
                            ]
                        }
                        },
                        {
                        "source": "3",
                        "target": "4",
                        "label": {
                            "cmd": "deliver",
                            "role": "Warehouse",
                            "logType": [
                            "ItemDeliver"
                            ]
                        }
                        },
                        {
                        "source": "4",
                        "target": "5",
                        "label": {
                            "cmd": "installEngine",
                            "role": "EngineInstaller",
                            "logType": [
                            "EngineInstalled"
                            ]
                        }
                        }
                    ]
                    }"#,
            )
            .unwrap()
        }
        fn get_warehouse_proto() -> SwarmProtocolType {
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        {
                        "source": "0",
                        "target": "1",
                        "label": {
                            "cmd": "request",
                            "role": "Warehouse",
                            "logType": [
                            "ItemRequest"
                            ]
                        }
                        },
                        {
                        "source": "1",
                        "target": "1",
                        "label": {
                            "cmd": "bid",
                            "role": "Transport",
                            "logType": [
                            "Bid"
                            ]
                        }
                        },
                        {
                        "source": "1",
                        "target": "2",
                        "label": {
                            "cmd": "select",
                            "role": "Transport",
                            "logType": [
                            "Selected"
                            ]
                        }
                        },
                        {
                        "source": "2",
                        "target": "3",
                        "label": {
                            "cmd": "needGuidance",
                            "role": "Transport",
                            "logType": [
                            "ReqGuidance"
                            ]
                        }
                        },
                        {
                        "source": "3",
                        "target": "4",
                        "label": {
                            "cmd": "giveGuidance",
                            "role": "BaseStation",
                            "logType": [
                            "GiveGuidance"
                            ]
                        }
                        },
                        {
                        "source": "4",
                        "target": "5",
                        "label": {
                            "cmd": "basicPickup",
                            "role": "Transport",
                            "logType": [
                            "ItemPickupBasic"
                            ]
                        }
                        },
                        {
                        "source": "2",
                        "target": "5",
                        "label": {
                            "cmd": "smartPickup",
                            "role": "Transport",
                            "logType": [
                            "ItemPickupSmart"
                            ]
                        }
                        },
                        {
                        "source": "5",
                        "target": "6",
                        "label": {
                            "cmd": "handover",
                            "role": "Transport",
                            "logType": [
                            "Handover"
                            ]
                        }
                        },
                        {
                        "source": "6",
                        "target": "0",
                        "label": {
                            "cmd": "deliver",
                            "role": "Warehouse",
                            "logType": [
                            "ItemDeliver"
                            ]
                        }
                        }
                    ]
                    }"#,
            )
            .unwrap()
        }
        fn get_wheel_installation_proto() -> SwarmProtocolType {
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        {
                        "source": "0",
                        "target": "1",
                        "label": {
                            "cmd": "installEngine",
                            "role": "EngineInstaller",
                            "logType": [
                            "EngineInstalled"
                            ]
                        }
                        },
                        {
                        "source": "1",
                        "target": "2",
                        "label": {
                            "cmd": "pickUpWheel",
                            "role": "WheelInstaller",
                            "logType": [
                            "WheelPickup"
                            ]
                        }
                        },
                        {
                        "source": "2",
                        "target": "1",
                        "label": {
                            "cmd": "installWheel",
                            "role": "WheelInstaller",
                            "logType": [
                            "WheelInstalled"
                            ]
                        }
                        },
                        {
                        "source": "1",
                        "target": "3",
                        "label": {
                            "cmd": "wheelsDone",
                            "role": "WheelInstaller",
                            "logType": [
                            "AllWheelsInstalled"
                            ]
                        }
                        },
                        {
                        "source": "3",
                        "target": "4",
                        "label": {
                            "cmd": "carDone",
                            "role": "QualityTransport",
                            "logType": [
                            "FinishedCar"
                            ]
                        }
                        }
                    ]
                    } "#,
            )
            .unwrap()
        }

        fn get_interfacing_protocols_1() -> InterfacingProtocols {
                InterfacingProtocols(vec![
                    get_steel_press_proto(), 
                    get_paint_proto(), 
                    get_engine_installation_proto(), 
                    get_warehouse_proto(),
                    get_wheel_installation_proto()])
            }

        fn get_wheel_installer() -> MachineType {
            serde_json::from_str::<MachineType>(
                r#"{
                    "initial": "s0",
                    "transitions": [
                        {
                        "source": "s1",
                        "target": "s1",
                        "label": {
                            "tag": "Execute",
                            "cmd": "pickUpWheel",
                            "logType": [
                            "WheelPickup"
                            ]
                        }
                        },
                        {
                        "source": "s1",
                        "target": "s1",
                        "label": {
                            "tag": "Execute",
                            "cmd": "wheelsDone",
                            "logType": [
                            "AllWheelsInstalled"
                            ]
                        }
                        },
                        {
                        "source": "s2",
                        "target": "s2",
                        "label": {
                            "tag": "Execute",
                            "cmd": "installWheel",
                            "logType": [
                            "WheelInstalled"
                            ]
                        }
                        },
                        {
                        "source": "s0",
                        "target": "s1",
                        "label": {
                            "tag": "Input",
                            "eventType": "EngineInstalled"
                        }
                        },
                        {
                        "source": "s1",
                        "target": "s2",
                        "label": {
                            "tag": "Input",
                            "eventType": "WheelPickup"
                        }
                        },
                        {
                        "source": "s1",
                        "target": "s3",
                        "label": {
                            "tag": "Input",
                            "eventType": "AllWheelsInstalled"
                        }
                        },
                        {
                        "source": "s2",
                        "target": "s1",
                        "label": {
                            "tag": "Input",
                            "eventType": "WheelInstalled"
                        }
                        }
                    ]
                    }"#,
            )
            .unwrap()
        }
        #[test]
        #[ignore]
        fn projection_information_wheel_installer() {
            let sub_result = overapprox_well_formed_sub(
                get_interfacing_protocols_1(),
                &BTreeMap::new(),
                Granularity::TwoStep,
            );
            let subscriptions = sub_result.unwrap();
            let projection_information =
                crate::composition::projection_information(Role::new(&format!("WheelInstaller")), get_interfacing_protocols_1(), 4, serde_json::to_string(&subscriptions).unwrap(), get_wheel_installer(), true);
            println!("ORIGINAL MACHINE: {}", serde_json::to_string_pretty(&get_wheel_installer()).unwrap());
            match projection_information {
                DataResult::ERROR {..} => assert!(false),
                DataResult::OK { data } =>  {
                    println!("ADAPTED: {}", serde_json::to_string_pretty(&data.projection).unwrap());
                }
            }
        }
    }
}
