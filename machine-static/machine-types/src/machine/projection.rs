use std::collections::BTreeSet;

use petgraph::{
    graph::EdgeReference,
    visit::{EdgeFiltered, EdgeRef, IntoEdgeReferences, IntoEdgesDirected, IntoNodeReferences},
    Direction::{Incoming, Outgoing},
};

use crate::{machine::minimize, types::{projection::{ChainedProjections, ChainedProtos, OptionGraph}, proto_info::{ProtoInfo, ProtoStruct}, typescript_types::EventType}};

use crate::types::{projection::Graph, proto_graph::NodeId, typescript_types::{EventLabel, MachineLabel, Role, StateName, Subscriptions, SwarmLabel}};
use crate::composition;

// Edge reference for graphs representing protocols. Used when filtering out edges that are not to be included in projection.
type ERef<'a> = <&'a crate::types::proto_graph::Graph as IntoEdgeReferences>::EdgeRef;

// Similar to machine::project, except that transitions with event types
// not subscribed to by role are skipped.
pub fn project(
    swarm: &crate::types::proto_graph::Graph,
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
        let (dfa, dfa_initial) = minimize::nfa_to_dfa(machine, m_nodes[initial.index()]); // make deterministic. slight deviation from projection operation formally.
        minimize::minimal_machine(&dfa, dfa_initial) // when minimizing we get a machine that is a little different but equivalent to the one prescribed by the projection operator formally
    } else {
        (machine, m_nodes[initial.index()])
    }
}

// Map the protocols of a proto_info to a ChainedProtos
pub(crate) fn to_chained_protos(proto_info: &ProtoInfo) -> ChainedProtos {
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
pub(crate) fn to_chained_projections(
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

    match combine_projs(projections, composition::gen_state_name) {
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

// rename this to combine_projections.
pub(crate) fn combine_projs<N: Clone, E: Clone + EventLabel>(
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
        |(acc, acc_i), (m, i, interface)| composition::compose(acc, acc_i, m, i, interface, gen_node),
    );
    Some((combined_projection, combined_initial))
}

fn to_option_machine(graph: &Graph) -> OptionGraph {
    graph.map(|_, n| Some(n.state_name().clone()), |_, x| x.clone())
}