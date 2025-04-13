use std::{collections::{BTreeMap, BTreeSet}, cmp::Ordering};


use itertools::Itertools;
use petgraph::{
    graph::EdgeReference,
    visit::{EdgeFiltered, EdgeRef, IntoEdgeReferences, IntoEdgesDirected, IntoNodeReferences},
    Direction::{Incoming, Outgoing},
};

use crate::{composition::composition_swarm::transitive_closure_succeeding, machine::{Error, Side}};

use super::{
    composition_types::{unord_event_pair, EventLabel, ProtoInfo, ProtoStruct, SucceedingNonBranchingJoining, UnordEventPair}, types::{StateName, Transition, Command}, EventType, Machine, MachineLabel, NodeId, Role, State, Subscriptions, SwarmLabel
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

// projection as described in Composing Swarm Protocols by Florian Furbach
pub fn project(
    swarm: &super::Graph,
    initial: NodeId,
    subs: &Subscriptions,
    role: Role,
) -> (Graph, NodeId) {
    //  assume each command emits exactly one event type
    //  find all nodes with incoming edges in subscription union {initial}
    //  these are the nodes of the projection
    //  for all nodes in projection:
    //      starting at node find nearest nodes with outgoing edges in sub. starting at node meaning node included in this search.
    //      for each edge at such a node:
    //          if edge describe commands performed by role add an execute self loop
    //          add an inpute edge terrminating where they terminate in protocol.
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
    println!("In project number of nodes is: {}", machine.node_count());
    print_info(&machine);

    //nfa_to_dfa(machine, m_nodes[initial.index()])
    let (dfa, dfa_initial) = nfa_to_dfa(machine, m_nodes[initial.index()]); // make deterministic. slight deviation from projection operation formally.
    println!("In project dfa number of nodes is: {}", dfa.node_count());
    print_info(&dfa);
    minimal_machine(&dfa, dfa_initial) // when minimizing we get a machine that is a little different but equivalent to the one prescribed by the projection operator formally
    //(machine, m_nodes[initial.index()])
    //(dfa, dfa_initial)
}

fn print_info(g: &Graph) {

    for n in g.node_indices() {
        println!("node: {} and index: {:?}", g[n], n);
    }
    for e in g.edge_references() {
        println!("edge: {:?} -{}-> {:?}", e.source(), e.weight(), e.target());
    }
}

// precondition: the protocols interfaces on the supplied interfaces.
// precondition: the composition of the protocols in swarms is wwf w.r.t. subs.
// the type of the input paremeter not nice? reconsider
pub fn project_combine(
    swarms: &Vec<ProtoStruct>,//&Vec<(super::Graph, NodeId, BTreeSet<EventType>)>,
    subs: &Subscriptions,
    role: Role,
) -> (OptionGraph, Option<NodeId>) {
    println!("In project combine !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    // check this anyway
    if swarms.is_empty()
        || !swarms[0].interface.is_empty()
        || swarms[0].initial.is_none()
        || swarms[1..]
            .iter()
            .any(|p| p.interface.is_empty() || p.initial.is_none())
    {
        return (OptionGraph::new(), None);
    }

    let mapper = |p: &ProtoStruct| -> (Graph, NodeId, BTreeSet<EventType>) {
        let (projection, projection_initial) = project(&p.graph, p.initial.unwrap(), subs, role.clone());
        (projection, projection_initial, p.interface.clone())
    };

    let projections: Vec<_> = swarms.into_iter().map(mapper).collect();

    let (acc_machine, acc_initial, _) = projections[0].clone();
    let (combined_projection, combined_initial) = projections[1..].to_vec().into_iter().fold(
        (acc_machine, acc_initial),
        |(acc, acc_i), (m, i, interface)| compose(acc, acc_i, m, i, interface),
    );
    let (combined_projection, combined_initial) = minimal_machine(&combined_projection, combined_initial);
    // why option here COME BACK
    (
        to_option_machine(&combined_projection),
        Some(combined_initial),
    )

}

pub fn project_combine_all(
    swarms: &Vec<ProtoStruct>,
    subs: &Subscriptions,
) -> Vec<(OptionGraph, Option<NodeId>)> {
    subs.keys()
        .map(|role| project_combine(swarms, subs, role.clone()))
        .collect()
}

// nfa to dfa using subset construction. Hopcroft, Motwani and Ullman section 2.3.5
fn nfa_to_dfa(nfa: Graph, i: NodeId) -> (Graph, NodeId) {
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
                    .and_modify(|v: &mut BTreeSet<NodeId>| { v.insert(target); })
                    .or_insert_with(|| BTreeSet::from([target]));
                m
            })
    };

    // add initial state to dfa
    dfa_nodes.insert(BTreeSet::from([i]), dfa.add_node(state_name(&BTreeSet::from([i]))));
    // add initial state to stack
    stack.push(BTreeSet::from([i]));

    while let Some(states) = stack.pop() {
        let map = outgoing_map(&states);
        println!("--------------------");
        println!("states are: {:?}", states);
        println!("map is: {:?}", map);
        println!("dfa number of states: {}", dfa.node_count());
        for edge in map.keys() {
            if !dfa_nodes.contains_key(&map[edge]) {
                stack.push(map[edge].clone());
            }
            let target: NodeId = *dfa_nodes
                .entry(map[edge].clone())
                .or_insert_with(|| dfa.add_node(state_name(&map[edge])));
                //.or_insert(dfa.add_node(state_name(&map[edge])));
            let src: NodeId = *dfa_nodes.get(&states).unwrap();
            dfa.add_edge(src, target, edge.clone());
        }
        println!("dfa number of states: {}", dfa.node_count());
        println!("--------------------");
    }

    (dfa, dfa_nodes[&BTreeSet::from([i])])
}

fn minimal_machine(graph: &Graph, i: NodeId) -> (Graph, NodeId) {
    println!("entered minimal machine");
    println!("graph is: {}", serde_json::to_string_pretty(&to_json_machine(graph.clone(), i)).unwrap());
    println!("number of nodes: {}", graph.node_count());
    let partition = partition_refinement(graph);
    let mut minimal = Graph::new();
    let mut node_to_partition = BTreeMap::new();
    let mut partition_to_minimal_graph_node = BTreeMap::new();
    let mut edges = BTreeSet::new();
    let state_name = |nodes: &BTreeSet<NodeId>| -> State {
        let name = format!("{{ {} }}", nodes.iter().map(|n| graph[*n].clone()).join(", "));
        println!("name: {}", name);
        State::new(&name)
    };

    for n in graph.node_indices() {
        node_to_partition.insert(n, partition.iter().find(|block| block.contains(&n)).unwrap());
    }
    println!("number of keys: {}", node_to_partition.keys().len());
    for (k,v) in node_to_partition.clone() {
        println!("node: {:?}", graph[k]);
        print!("in block: {{ ");
        for s in v {
            print!("{:?} ", graph[*s]);
        }
        println!(" }}");
        println!();
    }
    for block in &partition {
        println!("----block-----");
        for s in block {
            println!("state: {:?}", graph[*s]);
        }
        println!("----");
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

    println!("In minimal number of nodes is: {}", minimal.node_count());
    print_info(&minimal);
    (minimal, initial)


}


fn partition_refinement(graph: &Graph) -> BTreeSet<BTreeSet<NodeId>> {
    let mut partition_old = BTreeSet::new();
    let tmp: (BTreeSet<_>, BTreeSet<_>) = graph
        .node_indices()
        .partition(|n| graph.edges_directed(*n, Outgoing).count() == 0);
    let mut partition: BTreeSet<BTreeSet<NodeId>> = BTreeSet::from([tmp.0, tmp.1]);

    let pre_labels = |block: &BTreeSet<NodeId>| -> BTreeSet<MachineLabel> {
        block.iter().flat_map(|n| graph.edges_directed(*n, Incoming).map(|e|e.weight().clone())).collect()
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

fn refine_partition(graph: &Graph, partition: BTreeSet<BTreeSet<NodeId>>, superblock: &BTreeSet<NodeId>, label: &MachineLabel) -> BTreeSet<BTreeSet<NodeId>> {
    partition
        .iter()
        .flat_map(|block| refine_block(graph, block, superblock, label))
        .collect()
}

fn refine_block(graph: &Graph, block: &BTreeSet<NodeId>, superblock: &BTreeSet<NodeId>, label: &MachineLabel) -> BTreeSet<BTreeSet<NodeId>> {
    let predicate = |node: &NodeId| -> bool {
        graph.edges_directed(*node, Outgoing).any(|e| *e.weight() == *label && superblock.contains(&e.target()))
    };

    let tmp: (BTreeSet<_>, BTreeSet<_>) = block
        .iter()
        .partition(|n| predicate(n));

    BTreeSet::from([tmp.0, tmp.1]).into_iter().filter(|s| !s.is_empty()).collect()
}

fn visit_successors_stop_on_branch(proj: &OptionGraph, machine_state: NodeId, et: &EventType, special_events: &BTreeSet<EventType>, concurrent_events: &BTreeSet<UnordEventPair>) -> BTreeSet<EventType> {
    let mut visited = BTreeSet::new();
    let mut to_visit = Vec::from([machine_state]);
    let mut event_types = BTreeSet::new();
    //event_types.insert(et.clone());
    while let Some(node) = to_visit.pop() {
        visited.insert(node);
        for e in proj.edges_directed(node, Outgoing) {
            if !concurrent_events.contains(&unord_event_pair(e.weight().get_event_type(), et.clone())) {
                event_types.insert(e.weight().get_event_type());
            }
            if !special_events.contains(&e.weight().get_event_type())
                && !visited.contains(&e.target()) {
                    to_visit.push(e.target());
            }
        }
    }
    event_types
}

pub fn paths_from_event_types(proj: &OptionGraph, proto_info: &ProtoInfo) -> SucceedingNonBranchingJoining {
    let mut m: BTreeMap<EventType, BTreeSet<EventType>> = BTreeMap::new();
    let get_pre_joins = |e: &EventType| -> BTreeSet<EventType> {
        let pre = proto_info.immediately_pre.get(e).cloned().unwrap_or_default();
        let product = pre.clone().into_iter().cartesian_product(&pre);
        product.filter(|(e1, e2)| *e1 != **e2 && proto_info.concurrent_events.contains(&unord_event_pair(e1.clone(), (*e2).clone())))
            .map(|(e1, e2)| [e1, e2.clone()])
            .flatten()
            .collect()
    };

    let special_events = proto_info.branching_events.clone().into_iter()
        .flatten()
        .chain(proto_info.joining_events.clone().into_iter()
            .filter(|e| !get_pre_joins(e).is_empty()))
        .collect();

    let after_pairs: BTreeSet<UnordEventPair> = transitive_closure_succeeding(proto_info.succeeding_events.clone())
        .into_iter()
        .map(|(e, es)| [e].into_iter()
            .cartesian_product(&es)
            .map(|(e1, e2)| unord_event_pair(e1, e2.clone()))
            .collect::<BTreeSet<UnordEventPair>>())
        .flatten()
        .collect();
    let concurrent_events: BTreeSet<UnordEventPair> = proto_info.concurrent_events.difference(&after_pairs).cloned().collect();

    for node in proj.node_indices() {
        for edge in proj.edges_directed(node, Outgoing) {
            let mut paths_this_edge = visit_successors_stop_on_branch(proj, edge.target(), &edge.weight().get_event_type(), &special_events, &concurrent_events);
            m.entry(edge.weight().get_event_type()).and_modify(|s| s.append(&mut paths_this_edge)).or_insert_with(|| paths_this_edge);
        }
    }
    m
}

// precondition: both machines are projected from wwf protocols?
// precondition: m1 and m2 subscribe to all events in interface? Sort of works without but not really?
// takes type parameters to make it work for machines and protocols.
pub(in crate::composition) fn compose<N: StateName + From<String>, E: EventLabel>(
    m1: petgraph::Graph<N, E>,
    i1: NodeId,
    m2: petgraph::Graph<N, E>,
    i2: NodeId,
    interface: BTreeSet<EventType>,
) -> (petgraph::Graph<N, E>, NodeId) {
    let mut machine = petgraph::Graph::<N, E>::new();
    let mut node_map: BTreeMap<(NodeId, NodeId), NodeId> = BTreeMap::new();

    let gen_state_name = |s1: &N, s2: &N| -> N {
        let name = format!("{} || {}", s1.state_name(), s2.state_name());
        N::from(name)
    };

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

    let combined_initial = machine.add_node(gen_state_name(&m1[i1], &m2[i2]));
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
                let new_dst = machine.add_node(gen_state_name(&m1[dst1], &m2[dst2]));
                machine.add_edge(src, new_dst, e);
                node_map.insert((dst1, dst2), new_dst);
                worklist.push((new_dst, (dst1, dst2)));
            }
        }
    }

    (machine, combined_initial)
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
        Some(s) => s.to_string()
    }
}
/// error messages are designed assuming that `left` is the reference and `right` the tested
pub fn equivalent(left: &OptionGraph, li: NodeId, right: &OptionGraph, ri: NodeId) -> Vec<Error> {
    use Side::*;

    let _span = tracing::debug_span!("equivalent").entered();

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

pub(in crate::composition) fn to_option_machine(graph: &Graph) -> OptionGraph {
    graph.map(|_, n| Some(n.state_name().clone()), |_, x| x.clone())
}

pub fn to_json_machine(graph: Graph, initial: NodeId) -> Machine {
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

    Machine {
        initial: graph[initial].clone(),
        transitions,
    }
}

pub fn from_option_to_machine(
    graph: petgraph::Graph<Option<State>, MachineLabel>,
    initial: NodeId,
) -> Machine {
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

    Machine {
        initial: graph[initial].clone().unwrap_or(State::from("")),
        transitions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        composition::{
            composition_swarm::{compose_protocols, exact_weak_well_formed_sub, from_json, overapprox_weak_well_formed_sub, swarms_to_proto_info},
            composition_types::{CompositionComponent, Granularity, InterfacingSwarms},
        }, types::{Command, EventType, Role, Transition}, Machine, Subscriptions, SwarmProtocol
    };

    pub(in crate::composition) fn from_option_machine(graph: &OptionGraph) -> Graph {
        graph.map(|_, n| n.clone().unwrap().state_name().clone(), |_, x| x.clone())
    }
    // Example from coplaws slides
    fn get_proto1() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
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
    fn get_proto2() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
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
    fn get_proto3() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
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
    fn get_proto32() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
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
    fn get_proto333() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                    { "source": "1", "target": "2", "label": { "cmd": "test", "logType": ["report"], "role": "TR" } },
                    { "source": "2", "target": "3", "label": { "cmd": "accept", "logType": ["ok"], "role": "QCR" } },
                    { "source": "2", "target": "3", "label": { "cmd": "reject", "logType": ["notOk"], "role": "QCR" } },
                    { "source": "3", "target": "4", "label": { "cmd": "reject1", "logType": ["notOk1"], "role": "QCR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_interfacing_swarms_1() -> InterfacingSwarms<Role> {
        InterfacingSwarms(
            vec![
                CompositionComponent {
                    protocol: get_proto1(),
                    interface: None,
                },
                CompositionComponent {
                    protocol: get_proto2(),
                    interface: Some(Role::new("T")),
                },
            ]
        )
    }

    fn get_interfacing_swarms_1_reversed() -> InterfacingSwarms<Role> {
        InterfacingSwarms(
            vec![
                CompositionComponent {
                    protocol: get_proto2(),
                    interface: None,
                },
                CompositionComponent {
                    protocol: get_proto1(),
                    interface: Some(Role::new("T")),
                },
            ]
        )
    }

    fn get_interfacing_swarms_2() -> InterfacingSwarms<Role> {
        InterfacingSwarms(
            vec![
                CompositionComponent {
                    protocol: get_proto1(),
                    interface: None,
                },
                CompositionComponent {
                    protocol: get_proto2(),
                    interface: Some(Role::new("T")),
                },
                CompositionComponent {
                    protocol: get_proto3(),
                    interface: Some(Role::new("F")),
                },
            ]
        )
    }

    fn get_interfacing_swarms_2_reversed() -> InterfacingSwarms<Role> {
        InterfacingSwarms(
            vec![
                CompositionComponent {
                    protocol: get_proto3(),
                    interface: None,
                },
                CompositionComponent {
                    protocol: get_proto2(),
                    interface: Some(Role::new("F")),
                },
                CompositionComponent {
                    protocol: get_proto1(),
                    interface: Some(Role::new("T")),
                },
            ]
        )
    }

    fn get_interfacing_swarms_3() -> InterfacingSwarms<Role> {
        InterfacingSwarms(
            vec![
                CompositionComponent {
                    protocol: get_proto1(),
                    interface: None,
                },
                CompositionComponent {
                    protocol: get_proto2(),
                    interface: Some(Role::new("T")),
                },
                CompositionComponent {
                    protocol: get_proto32(),
                    interface: Some(Role::new("F")),
                },
            ]
        )
    }

    fn get_interfacing_swarms_333() -> InterfacingSwarms<Role> {
        InterfacingSwarms(
            vec![
                CompositionComponent {
                    protocol: get_proto1(),
                    interface: None,
                },
                CompositionComponent {
                    protocol: get_proto2(),
                    interface: Some(Role::new("T")),
                },
                CompositionComponent {
                    protocol: get_proto333(),
                    interface: Some(Role::new("F")),
                },
            ]
        )
    }

    fn get_interfacing_swarms_whhhh() -> InterfacingSwarms<Role> {
        InterfacingSwarms(
            vec![
                CompositionComponent {
                    protocol: get_proto1(),
                    interface: None,
                },
            ]
        )
    }

    #[test]
    fn test_projection_1() {
        // From Combining Swarm Protocols, example 5.
        let proto = serde_json::from_str::<SwarmProtocol>(
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
        let (proj, proj_initial) = project(&g, i.unwrap(), &subs, role);
        let expected_m = Machine {
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
        // warehouse example from coplaws slides
        let proto = get_proto1();
        let result_subs = exact_weak_well_formed_sub(InterfacingSwarms(vec![CompositionComponent::<Role>{protocol: proto.clone(), interface: None}]), &BTreeMap::new());
        assert!(result_subs.is_ok());
        let subs = result_subs.unwrap();
        let role = Role::new("FL");
        let (g, i, _) = from_json(proto);
        let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone());
        let right_m = Machine {
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

        println!("left {:?}: {}", role.clone(), serde_json::to_string_pretty(&to_json_machine(left.clone(), left_initial)).unwrap());
        println!("right {:?}: {}", role, serde_json::to_string_pretty(&from_option_to_machine(right.clone(), right_initial.unwrap())).unwrap());
        assert!(errors.is_empty());

        let errors = equivalent(
            &to_option_machine(&left),
            left_initial,
            &right,
            right_initial.unwrap());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_projection_3() {
        // car factory from coplaws example
        let proto = get_proto2();
        let result_subs = exact_weak_well_formed_sub(InterfacingSwarms(vec![CompositionComponent::<Role>{protocol: proto.clone(), interface: None}]), &BTreeMap::new());
        assert!(result_subs.is_ok());
        let subs = result_subs.unwrap();
        let role = Role::new("F");
        let (g, i, _) = from_json(proto);
        let (proj, proj_initial) = project(&g, i.unwrap(), &subs, role);
        let expected_m = Machine {
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
        // car factory from coplaws example
        let protos = get_interfacing_swarms_1();
        let result_subs = overapprox_weak_well_formed_sub(protos.clone(), &BTreeMap::from([(Role::new("T"), BTreeSet::from([EventType::new("car")]))]), Granularity::Coarse);
        assert!(result_subs.is_ok());
        let subs = result_subs.unwrap();

        let role = Role::new("T");
        let (g, i) = compose_protocols(protos).unwrap();
        let (proj, proj_initial) = project(&g, i, &subs, role);
        let expected_m = Machine {
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
        // warehouse example from coplaws slides
        let proto = get_proto1();
        let result_subs = exact_weak_well_formed_sub(InterfacingSwarms(vec![CompositionComponent::<Role>{protocol: proto.clone(), interface: None}]), &BTreeMap::new());
        assert!(result_subs.is_ok());
        let subs = result_subs.unwrap();
        let role = Role::new("FL");
        let (g, i, _) = from_json(proto);
        let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone());
        let right_m = Machine {
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

        println!("left {:?}: {}", role.clone(), serde_json::to_string_pretty(&to_json_machine(left.clone(), left_initial)).unwrap());
        println!("right {:?}: {}", role, serde_json::to_string_pretty(&from_option_to_machine(right.clone(), right_initial.unwrap())).unwrap());
        assert!(errors.is_empty());

        let errors = equivalent(
            &to_option_machine(&left),
            left_initial,
            &right,
            right_initial.unwrap());
        assert!(!errors.is_empty());
        let errors: Vec<String> = errors.into_iter().map(crate::machine::Error::convert(&to_option_machine(&left), &right)).collect();
        println!("{:?}", errors)

    }
    #[test]
    fn test_projection_fail_2() {
        // warehouse example from coplaws slides
        let proto = get_proto1();
        let result_subs = exact_weak_well_formed_sub(InterfacingSwarms(vec![CompositionComponent::<Role>{protocol: proto.clone(), interface: None}]), &BTreeMap::new());
        assert!(result_subs.is_ok());
        let subs = result_subs.unwrap();
        let role = Role::new("FL");
        let (g, i, _) = from_json(proto);
        let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone());
        let right_m = Machine {
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

        println!("left {:?}: {}", role.clone(), serde_json::to_string_pretty(&to_json_machine(left.clone(), left_initial)).unwrap());
        println!("right {:?}: {}", role, serde_json::to_string_pretty(&from_option_to_machine(right.clone(), right_initial.unwrap())).unwrap());
        assert!(errors.is_empty());

        let errors = equivalent(
            &to_option_machine(&left),
            left_initial,
            &right,
            right_initial.unwrap());
        assert!(!errors.is_empty());
        let errors: Vec<String> = errors.into_iter().map(crate::machine::Error::convert(&to_option_machine(&left), &right)).collect();
        println!("{:?}", errors)

    }
    #[test]
    fn test_projection_fail_3() {
        // warehouse example from coplaws slides
        let proto = get_proto1();
        let result_subs = exact_weak_well_formed_sub(InterfacingSwarms(vec![CompositionComponent::<Role>{protocol: proto.clone(), interface: None}]), &BTreeMap::new());
        assert!(result_subs.is_ok());
        let subs = result_subs.unwrap();
        let role = Role::new("FL");
        let (g, i, _) = from_json(proto);
        let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone());
        let right_m = Machine {
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

        println!("left {:?}: {}", role.clone(), serde_json::to_string_pretty(&to_json_machine(left.clone(), left_initial)).unwrap());
        println!("right {:?}: {}", role, serde_json::to_string_pretty(&from_option_to_machine(right.clone(), right_initial.unwrap())).unwrap());
        assert!(errors.is_empty());

        let errors = equivalent(
            &to_option_machine(&left),
            left_initial,
            &right,
            right_initial.unwrap());
        assert!(!errors.is_empty());
        let errors: Vec<String> = errors.into_iter().map(crate::machine::Error::convert(&to_option_machine(&left), &right)).collect();
        println!("{:?}", errors)
    }
    #[test]
    fn test_projection_fail_4() {
        // warehouse example from coplaws slides
        let proto = get_proto1();
        let result_subs = exact_weak_well_formed_sub(InterfacingSwarms(vec![CompositionComponent::<Role>{protocol: proto.clone(), interface: None}]), &BTreeMap::new());
        assert!(result_subs.is_ok());
        let subs = result_subs.unwrap();
        let role = Role::new("FL");
        let (g, i, _) = from_json(proto);
        let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone());
        let right_m = Machine {
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

        println!("left {:?}: {}", role.clone(), serde_json::to_string_pretty(&to_json_machine(left.clone(), left_initial)).unwrap());
        println!("right {:?}: {}", role, serde_json::to_string_pretty(&from_option_to_machine(right.clone(), right_initial.unwrap())).unwrap());
        assert!(errors.is_empty());

        let errors = equivalent(
            &to_option_machine(&left),
            left_initial,
            &right,
            right_initial.unwrap());
        assert!(!errors.is_empty());
        let errors: Vec<String> = errors.into_iter().map(crate::machine::Error::convert(&to_option_machine(&left), &right)).collect();
        println!("{:?}", errors)

    }
    #[test]
    fn test_combine_machines_1() {
        // Example from coplaws slides. Use generated WWF subscriptions. Project over T.
        let role = Role::new("T");
        let subs1 = crate::composition::composition_swarm::overapprox_weak_well_formed_sub(get_interfacing_swarms_1(), &BTreeMap::new(), Granularity::Coarse);
        assert!(subs1.is_ok());
        let subs1 = subs1.unwrap();
        let proto_info = swarms_to_proto_info(get_interfacing_swarms_1(), &subs1);
        assert!(proto_info.no_errors());

        let (proj_combined1, proj_combined_initial1) =
            project_combine(&proto_info.protocols, &subs1, role.clone());

        let subs2 = crate::composition::composition_swarm::overapprox_weak_well_formed_sub(get_interfacing_swarms_1_reversed(), &BTreeMap::new(), Granularity::Coarse);
        assert!(subs2.is_ok());
        let subs2 = subs2.unwrap();
        let proto_info = swarms_to_proto_info(get_interfacing_swarms_1_reversed(), &subs2);
        assert!(proto_info.no_errors());

        let (proj_combined2, proj_combined_initial2) =
            project_combine(&proto_info.protocols, &subs2, role.clone());

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
        let (proj, proj_initial) = project(&composed_graph, composed_initial, &subs1, role.clone());
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
        // fails when you use the exact subscriptions because that way not all roles subscribe to ALL interfaces? Ordering gets messed up.
        // the projected over the explicit composition may be correct, but the combined projections look weird and out of order.
        let composition = compose_protocols(get_interfacing_swarms_2());
        assert!(composition.is_ok());
        let (composed_graph, composed_initial) = composition.unwrap();
        let subs = crate::composition::composition_swarm::overapprox_weak_well_formed_sub(get_interfacing_swarms_2(), &BTreeMap::new(), Granularity::Coarse);
        assert!(subs.is_ok());
        let subs = subs.unwrap();
        let all_roles = vec![Role::new("T"), Role::new("FL"), Role::new("D"), Role::new("F"), Role::new("TR"), Role::new("QCR")];

        for role in all_roles {
            let subs1 = crate::composition::composition_swarm::overapprox_weak_well_formed_sub(get_interfacing_swarms_2(), &BTreeMap::new(), Granularity::Coarse);
            assert!(subs1.is_ok());
            let subs1 = subs1.unwrap();
            let proto_info = swarms_to_proto_info(get_interfacing_swarms_2(), &subs1);
            assert!(proto_info.no_errors());

            let (proj_combined1, proj_combined_initial1) =
                project_combine(&proto_info.protocols, &subs1, role.clone());

            let subs2 = crate::composition::composition_swarm::overapprox_weak_well_formed_sub(get_interfacing_swarms_2_reversed(), &BTreeMap::new(), Granularity::Coarse);
            assert!(subs2.is_ok());
            let subs2 = subs2.unwrap();
            let proto_info = swarms_to_proto_info(get_interfacing_swarms_2_reversed(), &subs2);
            assert!(proto_info.no_errors());

            let (proj_combined2, proj_combined_initial2) =
                project_combine(&proto_info.protocols, &subs2, role.clone());

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

            let (proj, proj_initial) = project(&composed_graph, composed_initial, &subs, role.clone());
            let errors =  equivalent(
                &proj_combined2,
                proj_combined_initial2.unwrap(),
                &to_option_machine(&proj),
                proj_initial
            );

            assert!(errors.is_empty());
            }
    }

    #[test]
    fn test_example_from_text_machine() {
        let role = Role::new("F");
        let subs = crate::composition::composition_swarm::overapprox_weak_well_formed_sub(get_interfacing_swarms_3(), &BTreeMap::new(), Granularity::Medium);
        assert!(subs.is_ok());
        let subs = subs.unwrap();
        let proto_info = swarms_to_proto_info(get_interfacing_swarms_3(), &subs);
            assert!(proto_info.no_errors());
        let (proj, proj_initial) =
            project_combine(&proto_info.protocols, &subs, role.clone());
        println!("projection of {}: {}", role.to_string(), serde_json::to_string_pretty(&from_option_to_machine(proj, proj_initial.unwrap())).unwrap());
    }

    #[test]
    fn test_all_projs_whf() {
        let composition = compose_protocols(get_interfacing_swarms_1());
        assert!(composition.is_ok());
        let (composed_graph, composed_initial) = composition.unwrap();
        let subs = crate::composition::composition_swarm::exact_weak_well_formed_sub(get_interfacing_swarms_1(), &BTreeMap::new());
        assert!(subs.is_ok());
        let subs = subs.unwrap();
        println!("subscription: {}", serde_json::to_string_pretty(&subs).unwrap());
        let all_roles = vec![Role::new("T"), Role::new("FL"), Role::new("D"), Role::new("F")];

        for role in all_roles {
            let (proj, proj_initial) = project(&composed_graph, composed_initial, &subs, role.clone());
            println!("{}: {}", role.clone().to_string(), serde_json::to_string_pretty(&to_json_machine(proj, proj_initial)).unwrap());
        }
    }

    #[test]
    #[ignore]
    fn test_all_projs_whfqcr() {
        let mut input_sub = BTreeMap::new();
        input_sub.insert(Role::new("T"), BTreeSet::from([EventType::new("notOk1")]));
        let subs = crate::composition::composition_swarm::overapprox_weak_well_formed_sub(get_interfacing_swarms_333(), &input_sub, Granularity::Medium).unwrap();
        let all_roles = vec![Role::new("T"), Role::new("FL"), Role::new("D"), Role::new("F"), Role::new("QCR")];
        let proto_info = swarms_to_proto_info(get_interfacing_swarms_333(), &subs);
            assert!(proto_info.no_errors());
        //println!("conc: {:?}", proto_info.concurrent_events);
        for role in all_roles {
            let (proj, proj_initial) =
                project_combine(&proto_info.protocols, &subs, role.clone());
            //let branching_event_types = proto_info.branching_events.clone().into_iter().flatten().collect::<BTreeSet<EventType>>();
            let branch_thing = paths_from_event_types(&proj, &proto_info);
            println!("role: {}\n branch thing: {}", role.to_string(), serde_json::to_string_pretty(&branch_thing).unwrap());
            let thing = from_option_to_machine(proj, proj_initial.unwrap());
            println!("proj: {}", serde_json::to_string_pretty(&thing).unwrap())
        }
    }

    #[test]
    #[ignore]
    fn test_all_projs_whfqcr1() {
        let subs = crate::composition::composition_swarm::overapprox_weak_well_formed_sub(get_interfacing_swarms_3(), &BTreeMap::new(), Granularity::Medium).unwrap();
        let all_roles = vec![Role::new("T"), Role::new("FL"), Role::new("D"), Role::new("F"), Role::new("QCR")];
        let proto_info = swarms_to_proto_info(get_interfacing_swarms_3(), &subs);
            assert!(proto_info.no_errors());
        //println!("conc: {:?}", proto_info.concurrent_events);
        for role in all_roles {
            let (proj, proj_initial) =
                project_combine(&proto_info.protocols, &subs, role.clone());
            //let branching_event_types = proto_info.branching_events.clone().into_iter().flatten().collect::<BTreeSet<EventType>>();
            let branch_thing = paths_from_event_types(&proj, &proto_info);
            println!("role: {}\n branch thing: {}", role.to_string(), serde_json::to_string_pretty(&branch_thing).unwrap());
            let thing = from_option_to_machine(proj, proj_initial.unwrap());
            println!("proj: {}", serde_json::to_string_pretty(&thing).unwrap())
        }
    }

    #[test]
    #[ignore]
    fn test_all_projs_wh_only() {
        let input_sub = BTreeMap::new();
        let subs = crate::composition::composition_swarm::overapprox_weak_well_formed_sub(get_interfacing_swarms_whhhh(), &input_sub, Granularity::Medium).unwrap();
        let all_roles = vec![Role::new("T"), Role::new("FL"), Role::new("D")];
        println!("subs: {}", serde_json::to_string_pretty(&subs).unwrap());
        let proto_info = swarms_to_proto_info(get_interfacing_swarms_whhhh(), &subs);
            assert!(proto_info.no_errors());
        //println!("conc: {:?}", proto_info.concurrent_events);

        for role in all_roles {
            let (proj, proj_initial) =
                project_combine(&proto_info.protocols, &subs, role.clone());
            //let branching_event_types = proto_info.branching_events.clone().into_iter().flatten().collect::<BTreeSet<EventType>>();
            let _branch_thing = paths_from_event_types(&proj, &proto_info);
            //println!("role: {}\n branch thing: {}", role.to_string(), serde_json::to_string_pretty(&branch_thing).unwrap());
            let thing = from_option_to_machine(proj, proj_initial.unwrap());
            println!("{}\n$$$$", serde_json::to_string_pretty(&thing).unwrap())
            //let thing = project(&proto_info.protocols.)
        }
    }

}
