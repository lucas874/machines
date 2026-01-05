use std::collections::BTreeSet;

use crate::types::proto_info;
use crate::types::{
    proto_info::ProtoInfo,
    typescript_types::{EventType, Role, Subscriptions},
};

pub mod exact;
pub mod overapproximation;

// Handle looping event types.
// For each event type t that does not lead to a terminal state, check looping condition from determinacy:
// if t is not in subscriptions, add it to all roles in roles(t, G).
// Awkwardly placed here because it is used by exact and overapproximation.
fn add_looping_event_types(proto_info: &ProtoInfo, subscriptions: &mut Subscriptions) {
    let _span = tracing::info_span!("add_looping_event_types").entered();

    // For each event type t in the set of event types that can not reach a terminal state, check predicate adding t to subs of all involved roles if false.
    for t in &proto_info.infinitely_looping_events {
        let t_and_after_t: BTreeSet<EventType> = [t.clone()]
            .into_iter()
            .chain(
                proto_info
                    .succeeding_events
                    .get(t)
                    .cloned()
                    .unwrap_or_else(|| BTreeSet::new()),
            )
            .collect();
        let involved_roles = proto_info::roles_on_path(t.clone(), proto_info, subscriptions);

        // If there is not an event type among t_and_after_t such that all roles subscribe to this event type, add t to the subscription of all involved roles.
        if !all_roles_sub_to_same(t_and_after_t, &involved_roles, &subscriptions) {
            for r in involved_roles.iter() {
                subscriptions
                    .entry(r.clone())
                    .and_modify(|set| {
                        set.insert(t.clone());
                    })
                    .or_insert_with(|| BTreeSet::from([t.clone()]));
            }
        }
    }
}

// True if there exists an event type in event_types such that all roles in involved_roles subscribe to it.
fn all_roles_sub_to_same(
    event_types: BTreeSet<EventType>,
    involved_roles: &BTreeSet<Role>,
    subs: &Subscriptions,
) -> bool {
    let _span = tracing::info_span!("all_roles_sub_to_same").entered();
    let empty = BTreeSet::new();
    event_types.into_iter().any(|t_| {
        involved_roles
            .iter()
            .all(|r| subs.get(r).unwrap_or(&empty).contains(&t_))
    })
}
