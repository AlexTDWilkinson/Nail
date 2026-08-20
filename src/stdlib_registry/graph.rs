//! Graph module stdlib registry entries.
//!
//! A graph is two parallel arrays: edges_from[i] -> edges_to[i] is one
//! directed edge, the shape a language without tuples holds pairs in. A node
//! exists by appearing in an edge, so an isolated node is simply not in the
//! graph. Every function answers in the order the edge arrays first mention
//! nodes, so the same input always gives the same answer.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Graph:
        "graph_topological_sort" => "std_lib::graph::topological_sort", (edges_from: [(K: i|s)], edges_to: [K]) -> ([K]!e),
            "An order that puts each edge's first node before its second. When every edge points from a prerequisite to the thing that needs it, this is the order to build, migrate or load in. Errors when the edge arrays differ in length, or when the edges loop, and the cycle error names the loop.",
            "edges_from:a:s = [`schema`, `schema`, `api`];\nedges_to:a:s = [`api`, `seed`, `web`];\nbuild_order:a:s = danger(graph_topological_sort(edges_from, edges_to));\nprint(array_join(build_order, ` then `));";
        "graph_has_cycle" => "std_lib::graph::has_cycle", (edges_from: [(K: i|s)], edges_to: [K]) -> (b!e),
            "Whether following the edges around can ever come back to a node already passed through. The question to ask when a cycle is a real possibility rather than a mistake, since graph_topological_sort treats one as an error. Errors when the edge arrays differ in length.",
            "edges_from:a:s = [`wake`, `brew`];\nedges_to:a:s = [`brew`, `wake`];\nstuck:b = danger(graph_has_cycle(edges_from, edges_to));\nprint(stuck);";
        "graph_connected_components" => "std_lib::graph::connected_components", (edges_from: [(K: i|s)], edges_to: [K]) -> ([[K]]!e),
            "The groups of nodes that touch through edges read in either direction, each group in its own array. Merging pairs into clusters, records that share an email, islands on a grid, are all this one call. Errors when the edge arrays differ in length.",
            "edges_from:a:s = [`alice`, `bob`, `dana`];\nedges_to:a:s = [`bob`, `carol`, `erin`];\ngroups:a:a:s = danger(graph_connected_components(edges_from, edges_to));\nprint(array_length(groups));";
        "graph_reachable" => "std_lib::graph::reachable", (edges_from: [(K: i|s)], edges_to: [K], start: K) -> ([K]!e),
            "Every node the edges lead to from the start, following them one way only, the start itself first. Swapping the two edge arrays turns the question around into what reaches this node. Errors when the edge arrays differ in length or the start appears in no edge.",
            "edges_from:a:s = [`web`, `api`, `api`];\nedges_to:a:s = [`api`, `db`, `cache`];\npulled_in:a:s = danger(graph_reachable(edges_from, edges_to, `web`));\nprint(array_join(pulled_in, `, `));";
        "graph_shortest_path" => "std_lib::graph::shortest_path", (edges_from: [(K: i|s)], edges_to: [K], start: K, goal: K) -> ([K]!e),
            "The route from start to goal that crosses the fewest edges, both ends included. Errors when the edge arrays differ in length, when either end appears in no edge, or when no route exists.",
            "edges_from:a:s = [`home`, `home`, `park`, `mall`];\nedges_to:a:s = [`park`, `mall`, `office`, `office`];\nroute:a:s = danger(graph_shortest_path(edges_from, edges_to, `home`, `office`));\nprint(array_join(route, ` -> `));";
    }

    // The weighted route answers with two things at once, a path and what it
    // costs, so it returns the GRAPH_Path struct and takes string node ids,
    // since a struct's fields name concrete types.
    m.insert("graph_shortest_path_weighted", StdlibFunction {
        rust_path: "std_lib::graph::shortest_path_weighted".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("GRAPH_Path", "nail::std_lib::graph")],
        module: StdlibModule::Graph,
        parameters: vec![
            nail_param!(edges_from: [s]),
            nail_param!(edges_to: [s]),
            nail_param!(weights: [f]),
            nail_param!(start: s),
            nail_param!(goal: s),
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("GRAPH_Path".to_string()))),
        diverging: false,
        description: "The cheapest route from start to goal when every edge carries a cost, with the route and its total together in a GRAPH_Path. One weight per edge, by position. Errors when the arrays differ in length, when a weight is negative or not a number, when either end appears in no edge, or when no route exists.",
        example: "edges_from:a:s = [`home`, `home`, `park`];\nedges_to:a:s = [`park`, `office`, `office`];\nweights:a:f = [1.0, 5.0, 1.5];\nroute:GRAPH_Path = danger(graph_shortest_path_weighted(edges_from, edges_to, weights, `home`, `office`));\nprint(route.cost);",
    });
}
