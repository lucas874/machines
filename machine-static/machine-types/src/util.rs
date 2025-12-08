use std::collections::{BTreeMap, BTreeSet};

// combine maps with sets as values
pub(crate) fn combine_maps<K: Ord + Clone, V: Ord + Clone>(
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
