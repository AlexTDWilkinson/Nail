//! Graph algorithms over edge lists.
//!
//! A graph arrives as two parallel arrays: edges_from[i] -> edges_to[i] is one
//! directed edge. Nail has no tuples, so a pair of arrays is how a program
//! holds a list of pairs, the same shape array_zip_with reads. A node exists
//! by appearing in an edge. Every function walks nodes in the order the edge
//! arrays first mention them, so the same input always gives the same answer.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt::Display;
use std::hash::Hash;

/// The route Dijkstra found and what it costs, together, because a route
/// handed back without its cost sends the caller straight back over the
/// edges to add it up again.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GRAPH_Path {
    pub nodes: Vec<String>,
    pub cost: f64,
}

/// Nodes in first-appearance order, plus every edge as a pair of indices into
/// that order. Working in indices keeps the algorithms deterministic without
/// sorting, and keeps the walks off the node type entirely.
fn index_edges<K: Hash + Eq + Clone>(edges_from: &[K], edges_to: &[K], function_name: &str) -> Result<(Vec<K>, Vec<(usize, usize)>), String> {
    if edges_from.len() != edges_to.len() {
        return Err(format!(
            "{}: edges_from has {} elements but edges_to has {}, and an edge needs one of each",
            function_name,
            edges_from.len(),
            edges_to.len()
        ));
    }
    let mut order: Vec<K> = Vec::new();
    let mut index_of: HashMap<K, usize> = HashMap::new();
    let mut edges: Vec<(usize, usize)> = Vec::with_capacity(edges_from.len());
    for (source, target) in edges_from.iter().zip(edges_to.iter()) {
        let source_index = intern(source, &mut order, &mut index_of);
        let target_index = intern(target, &mut order, &mut index_of);
        edges.push((source_index, target_index));
    }
    return Ok((order, edges));
}

fn intern<K: Hash + Eq + Clone>(node: &K, order: &mut Vec<K>, index_of: &mut HashMap<K, usize>) -> usize {
    if let Some(&index) = index_of.get(node) {
        return index;
    }
    let index = order.len();
    order.push(node.clone());
    index_of.insert(node.clone(), index);
    return index;
}

/// Kahn's algorithm: emit nodes whose prerequisites are all emitted, in
/// first-appearance order among the ready. Nodes left unemitted are exactly
/// the ones caught in or downstream of a cycle.
fn kahn_sort(node_count: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    let mut successors: Vec<Vec<usize>> = vec![Vec::new(); node_count];
    let mut indegree: Vec<usize> = vec![0; node_count];
    for (source, target) in edges {
        successors[*source].push(*target);
        indegree[*target] += 1;
    }
    let mut ready: VecDeque<usize> = (0..node_count).filter(|&node| indegree[node] == 0).collect();
    let mut sorted: Vec<usize> = Vec::with_capacity(node_count);
    while let Some(node) = ready.pop_front() {
        sorted.push(node);
        for &next in &successors[node] {
            indegree[next] -= 1;
            if indegree[next] == 0 {
                ready.push_back(next);
            }
        }
    }
    return sorted;
}

/// One actual cycle among the unemitted nodes, written out forwards. Every
/// unemitted node still has an unemitted predecessor (that is why it was never
/// freed), so walking predecessors from any of them must repeat a node, and
/// the repeated stretch read backwards is a forward cycle.
fn describe_cycle<K: Display>(order: &[K], edges: &[(usize, usize)], emitted: &[bool]) -> String {
    let mut predecessor: Vec<Option<usize>> = vec![None; order.len()];
    for (source, target) in edges {
        if !emitted[*source] && !emitted[*target] {
            predecessor[*target] = Some(*source);
        }
    }
    let start = match (0..order.len()).find(|&node| !emitted[node]) {
        Some(node) => node,
        None => return String::new(),
    };
    let mut seen_at: HashMap<usize, usize> = HashMap::new();
    let mut path: Vec<usize> = Vec::new();
    let mut current = start;
    loop {
        if let Some(&position) = seen_at.get(&current) {
            let mut names: Vec<String> = path[position..].iter().rev().map(|&node| order[node].to_string()).collect();
            names.push(names[0].clone());
            return names.join(" -> ");
        }
        seen_at.insert(current, path.len());
        path.push(current);
        current = match predecessor[current] {
            Some(previous) => previous,
            None => return order[current].to_string(),
        };
    }
}

/// An order that puts each edge's source before its target, or the cycle that
/// makes such an order impossible.
pub fn topological_sort<K: Hash + Eq + Clone + Display>(edges_from: Vec<K>, edges_to: Vec<K>) -> Result<Vec<K>, String> {
    let (order, edges) = index_edges(&edges_from, &edges_to, "graph_topological_sort")?;
    let sorted = kahn_sort(order.len(), &edges);
    if sorted.len() < order.len() {
        let mut emitted = vec![false; order.len()];
        for &node in &sorted {
            emitted[node] = true;
        }
        return Err(format!("graph_topological_sort: the edges form a cycle: {}", describe_cycle(&order, &edges, &emitted)));
    }
    return Ok(sorted.into_iter().map(|node| order[node].clone()).collect());
}

/// Whether following the edges forward can ever return to a node already
/// passed through.
pub fn has_cycle<K: Hash + Eq + Clone>(edges_from: Vec<K>, edges_to: Vec<K>) -> Result<bool, String> {
    let (order, edges) = index_edges(&edges_from, &edges_to, "graph_has_cycle")?;
    return Ok(kahn_sort(order.len(), &edges).len() < order.len());
}

/// The groups of nodes joined by edges read in either direction, each group
/// in first-visit order, groups in the order their first node appeared.
pub fn connected_components<K: Hash + Eq + Clone>(edges_from: Vec<K>, edges_to: Vec<K>) -> Result<Vec<Vec<K>>, String> {
    let (order, edges) = index_edges(&edges_from, &edges_to, "graph_connected_components")?;
    let mut neighbors: Vec<Vec<usize>> = vec![Vec::new(); order.len()];
    for (source, target) in &edges {
        neighbors[*source].push(*target);
        neighbors[*target].push(*source);
    }
    let mut assigned = vec![false; order.len()];
    let mut components: Vec<Vec<K>> = Vec::new();
    for seed in 0..order.len() {
        if assigned[seed] {
            continue;
        }
        assigned[seed] = true;
        let mut component: Vec<K> = Vec::new();
        let mut queue: VecDeque<usize> = VecDeque::from([seed]);
        while let Some(node) = queue.pop_front() {
            component.push(order[node].clone());
            for &next in &neighbors[node] {
                if !assigned[next] {
                    assigned[next] = true;
                    queue.push_back(next);
                }
            }
        }
        components.push(component);
    }
    return Ok(components);
}

/// Every node the edges lead to from the start, following them forward only,
/// in breadth-first order with the start itself first.
pub fn reachable<K: Hash + Eq + Clone + Display>(edges_from: Vec<K>, edges_to: Vec<K>, start: K) -> Result<Vec<K>, String> {
    let (order, edges) = index_edges(&edges_from, &edges_to, "graph_reachable")?;
    let start_index = match order.iter().position(|node| *node == start) {
        Some(index) => index,
        None => return Err(format!("graph_reachable: start node {} appears in no edge", start)),
    };
    let mut successors: Vec<Vec<usize>> = vec![Vec::new(); order.len()];
    for (source, target) in &edges {
        successors[*source].push(*target);
    }
    let mut visited = vec![false; order.len()];
    visited[start_index] = true;
    let mut found: Vec<K> = Vec::new();
    let mut queue: VecDeque<usize> = VecDeque::from([start_index]);
    while let Some(node) = queue.pop_front() {
        found.push(order[node].clone());
        for &next in &successors[node] {
            if !visited[next] {
                visited[next] = true;
                queue.push_back(next);
            }
        }
    }
    return Ok(found);
}

/// The route from start to goal that crosses the fewest edges, both ends
/// included, by breadth-first search.
pub fn shortest_path<K: Hash + Eq + Clone + Display>(edges_from: Vec<K>, edges_to: Vec<K>, start: K, goal: K) -> Result<Vec<K>, String> {
    let (order, edges) = index_edges(&edges_from, &edges_to, "graph_shortest_path")?;
    let start_index = match order.iter().position(|node| *node == start) {
        Some(index) => index,
        None => return Err(format!("graph_shortest_path: start node {} appears in no edge", start)),
    };
    let goal_index = match order.iter().position(|node| *node == goal) {
        Some(index) => index,
        None => return Err(format!("graph_shortest_path: goal node {} appears in no edge", goal)),
    };
    let mut successors: Vec<Vec<usize>> = vec![Vec::new(); order.len()];
    for (source, target) in &edges {
        successors[*source].push(*target);
    }
    let mut parent: Vec<Option<usize>> = vec![None; order.len()];
    let mut visited = vec![false; order.len()];
    visited[start_index] = true;
    let mut queue: VecDeque<usize> = VecDeque::from([start_index]);
    while let Some(node) = queue.pop_front() {
        if node == goal_index {
            return Ok(walk_parents(&order, &parent, start_index, goal_index));
        }
        for &next in &successors[node] {
            if !visited[next] {
                visited[next] = true;
                parent[next] = Some(node);
                queue.push_back(next);
            }
        }
    }
    return Err(format!("graph_shortest_path: no route from {} to {}", start, goal));
}

fn walk_parents<K: Clone>(order: &[K], parent: &[Option<usize>], start_index: usize, goal_index: usize) -> Vec<K> {
    let mut indices = vec![goal_index];
    let mut current = goal_index;
    while current != start_index {
        current = parent[current].expect("every node on a found route has a parent back to the start");
        indices.push(current);
    }
    indices.reverse();
    return indices.into_iter().map(|node| order[node].clone()).collect();
}

/// One pending Dijkstra visit. Ordered by cost alone, backwards, so that a
/// max-heap of these hands out the cheapest first.
struct PendingVisit {
    cost: f64,
    node: usize,
}

impl PartialEq for PendingVisit {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}
impl Eq for PendingVisit {}
impl PartialOrd for PendingVisit {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for PendingVisit {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.cost.total_cmp(&self.cost)
    }
}

/// The cheapest route from start to goal when every edge carries a cost, with
/// the route and its total returned together. Costs must not be negative,
/// which is the assumption Dijkstra's answer rests on.
pub fn shortest_path_weighted(edges_from: Vec<String>, edges_to: Vec<String>, weights: Vec<f64>, start: String, goal: String) -> Result<GRAPH_Path, String> {
    if edges_from.len() != edges_to.len() {
        return Err(format!(
            "graph_shortest_path_weighted: edges_from has {} elements but edges_to has {}, and an edge needs one of each",
            edges_from.len(),
            edges_to.len()
        ));
    }
    if weights.len() != edges_from.len() {
        return Err(format!("graph_shortest_path_weighted: {} edges but {} weights, and every edge needs its cost", edges_from.len(), weights.len()));
    }
    for (position, weight) in weights.iter().enumerate() {
        if !weight.is_finite() {
            return Err(format!("graph_shortest_path_weighted: the weight at position {} is not a finite number", position));
        }
        if *weight < 0.0 {
            return Err(format!(
                "graph_shortest_path_weighted: the weight {} at position {} is negative, and the cheapest-route answer is only right when no edge pays you to cross it",
                weight, position
            ));
        }
    }
    let (order, edges) = index_edges(&edges_from, &edges_to, "graph_shortest_path_weighted")?;
    let start_index = match order.iter().position(|node| *node == start) {
        Some(index) => index,
        None => return Err(format!("graph_shortest_path_weighted: start node {} appears in no edge", start)),
    };
    let goal_index = match order.iter().position(|node| *node == goal) {
        Some(index) => index,
        None => return Err(format!("graph_shortest_path_weighted: goal node {} appears in no edge", goal)),
    };
    let mut successors: Vec<Vec<(usize, f64)>> = vec![Vec::new(); order.len()];
    for ((source, target), weight) in edges.iter().zip(weights.iter()) {
        successors[*source].push((*target, *weight));
    }
    let mut best_cost: Vec<f64> = vec![f64::INFINITY; order.len()];
    let mut parent: Vec<Option<usize>> = vec![None; order.len()];
    best_cost[start_index] = 0.0;
    let mut pending: std::collections::BinaryHeap<PendingVisit> = std::collections::BinaryHeap::new();
    pending.push(PendingVisit { cost: 0.0, node: start_index });
    while let Some(visit) = pending.pop() {
        if visit.cost > best_cost[visit.node] {
            continue;
        }
        if visit.node == goal_index {
            break;
        }
        for &(next, weight) in &successors[visit.node] {
            let candidate = visit.cost + weight;
            if candidate < best_cost[next] {
                best_cost[next] = candidate;
                parent[next] = Some(visit.node);
                pending.push(PendingVisit { cost: candidate, node: next });
            }
        }
    }
    if best_cost[goal_index].is_infinite() {
        return Err(format!("graph_shortest_path_weighted: no route from {} to {}", start, goal));
    }
    return Ok(GRAPH_Path { nodes: walk_parents(&order, &parent, start_index, goal_index), cost: best_cost[goal_index] });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn topological_sort_puts_prerequisites_first() {
        let sorted = topological_sort(words(&["schema", "schema", "api"]), words(&["api", "seed", "web"])).unwrap();
        assert_eq!(sorted, words(&["schema", "api", "seed", "web"]));
    }

    #[test]
    fn topological_sort_works_on_int_nodes() {
        let sorted = topological_sort(vec![1_i64, 2], vec![2, 3]).unwrap();
        assert_eq!(sorted, vec![1, 2, 3]);
    }

    #[test]
    fn topological_sort_names_the_cycle_forwards() {
        let refusal = topological_sort(words(&["wake", "brew", "brew"]), words(&["brew", "wake", "pour"])).unwrap_err();
        assert!(refusal.contains("wake -> brew -> wake") || refusal.contains("brew -> wake -> brew"), "{}", refusal);
    }

    #[test]
    fn topological_sort_rejects_a_self_loop() {
        let refusal = topological_sort(words(&["boot"]), words(&["boot"])).unwrap_err();
        assert!(refusal.contains("boot -> boot"), "{}", refusal);
    }

    #[test]
    fn mismatched_arrays_are_refused() {
        let refusal = topological_sort(words(&["one", "two"]), words(&["three"])).unwrap_err();
        assert!(refusal.contains("2 elements but edges_to has 1"), "{}", refusal);
    }

    #[test]
    fn has_cycle_tells_the_two_apart() {
        assert!(!has_cycle(words(&["schema"]), words(&["api"])).unwrap());
        assert!(has_cycle(words(&["wake", "brew"]), words(&["brew", "wake"])).unwrap());
    }

    #[test]
    fn components_group_through_either_direction() {
        let groups = connected_components(words(&["alice", "bob", "dana"]), words(&["bob", "carol", "erin"])).unwrap();
        assert_eq!(groups, vec![words(&["alice", "bob", "carol"]), words(&["dana", "erin"])]);
    }

    #[test]
    fn reachable_follows_edges_one_way() {
        let found = reachable(words(&["web", "api", "api", "worker"]), words(&["api", "db", "cache", "db"]), "web".to_string()).unwrap();
        assert_eq!(found, words(&["web", "api", "db", "cache"]));
    }

    #[test]
    fn reachable_reversed_answers_what_depends_on_this() {
        let found = reachable(words(&["api", "db", "cache", "db"]), words(&["web", "api", "api", "worker"]), "db".to_string()).unwrap();
        assert_eq!(found, words(&["db", "api", "worker", "web"]));
    }

    #[test]
    fn reachable_refuses_a_node_no_edge_mentions() {
        let refusal = reachable(words(&["web"]), words(&["api"]), "ghost".to_string()).unwrap_err();
        assert!(refusal.contains("ghost appears in no edge"), "{}", refusal);
    }

    #[test]
    fn shortest_path_counts_edges_not_luck() {
        let route = shortest_path(words(&["home", "home", "park", "mall", "park"]), words(&["park", "mall", "office", "office", "mall"]), "home".to_string(), "office".to_string()).unwrap();
        assert_eq!(route, words(&["home", "park", "office"]));
    }

    #[test]
    fn shortest_path_to_itself_is_just_the_node() {
        let route = shortest_path(words(&["home"]), words(&["park"]), "home".to_string(), "home".to_string()).unwrap();
        assert_eq!(route, words(&["home"]));
    }

    #[test]
    fn shortest_path_with_no_route_says_so() {
        let refusal = shortest_path(words(&["home", "office"]), words(&["park", "mall"]), "home".to_string(), "office".to_string()).unwrap_err();
        assert!(refusal.contains("no route from home to office"), "{}", refusal);
    }

    #[test]
    fn weighted_route_returns_path_and_cost_together() {
        let found = shortest_path_weighted(words(&["home", "home", "park"]), words(&["park", "office", "office"]), vec![1.0, 5.0, 1.5], "home".to_string(), "office".to_string()).unwrap();
        assert_eq!(found.nodes, words(&["home", "park", "office"]));
        assert!((found.cost - 2.5).abs() < 1e-9);
    }

    #[test]
    fn weighted_route_refuses_a_negative_weight() {
        let refusal = shortest_path_weighted(words(&["home"]), words(&["park"]), vec![-1.0], "home".to_string(), "park".to_string()).unwrap_err();
        assert!(refusal.contains("negative"), "{}", refusal);
    }

    #[test]
    fn weighted_route_refuses_a_missing_weight() {
        let refusal = shortest_path_weighted(words(&["home"]), words(&["park"]), vec![], "home".to_string(), "park".to_string()).unwrap_err();
        assert!(refusal.contains("1 edges but 0 weights"), "{}", refusal);
    }
}
