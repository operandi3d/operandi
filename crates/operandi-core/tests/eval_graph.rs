//! The evaluation graph's data structure (EVAL-GRAPH-PROPOSAL step 1):
//! dirty marking, loops refused, each dirty node computed once.
use operandi_core::eval_graph::{EvalGraph, NodeKey};

fn ch(item: u64, name: &str) -> NodeKey {
    NodeKey::Channel(item, name.into())
}

/// Time → a, b; a → c; b → c. Values: time is set, a = t+1, b = t*2,
/// c = a+b.
fn diamond() -> (EvalGraph<f64>, [operandi_core::eval_graph::NodeId; 4]) {
    let mut g = EvalGraph::new();
    let t = g.node(NodeKey::Time);
    let a = g.node(ch(1, "a"));
    let b = g.node(ch(1, "b"));
    let c = g.node(NodeKey::Field(1, "c".into()));
    g.connect(t, a).unwrap();
    g.connect(t, b).unwrap();
    g.connect(a, c).unwrap();
    g.connect(b, c).unwrap();
    g.set(t, 3.);
    (g, [t, a, b, c])
}

fn compute(key: &NodeKey, inputs: &[&f64]) -> f64 {
    match key {
        NodeKey::Channel(_, n) if n == "a" => inputs[0] + 1.,
        NodeKey::Channel(_, n) if n == "b" => inputs[0] * 2.,
        _ => inputs.iter().copied().sum(),
    }
}

#[test]
fn pull_computes_inputs_first_and_each_dirty_node_once() {
    let (mut g, [_, _, _, c]) = diamond();
    assert_eq!(g.pull(c, &mut compute), 4. + 6.);
    assert_eq!(g.computes(), 3, "a, b and c once each");
    // Clean now: pulling again computes nothing.
    assert_eq!(g.pull(c, &mut compute), 10.);
    assert_eq!(g.computes(), 3);
    assert!(g.dirty_nodes().is_empty());
}

#[test]
fn setting_a_source_dirties_only_what_reads_it() {
    let (mut g, [t, a, b, c]) = diamond();
    g.pull(c, &mut compute);
    g.set(t, 5.);
    assert_eq!(g.dirty_nodes(), vec![a, b, c]);
    assert_eq!(g.pull(c, &mut compute), 6. + 10.);
    // Marking b alone redoes b and c, not a.
    let before = g.computes();
    assert_eq!(g.mark_dirty(b), 2);
    g.pull(c, &mut compute);
    assert_eq!(g.computes() - before, 2);
}

#[test]
fn marking_stops_at_nodes_already_dirty() {
    let (mut g, [_, a, b, c]) = diamond();
    g.pull(c, &mut compute);
    assert_eq!(g.mark_dirty(a), 2, "a and c");
    assert_eq!(g.mark_dirty(b), 1, "c was dirty already");
    assert_eq!(g.mark_dirty(a), 0);
}

#[test]
fn a_loop_is_refused_and_leaves_the_graph_as_it_was() {
    let (mut g, [t, a, _, c]) = diamond();
    let edges = g.describe_edges();
    let e = g.connect(c, a).unwrap_err();
    assert!(e.contains("Dependency loop"), "{e}");
    assert!(g.connect(c, c).is_err());
    assert!(g.connect(a, t).is_err());
    assert_eq!(g.describe_edges(), edges);
    // Connecting twice is one edge.
    g.connect(a, c).unwrap();
    assert_eq!(g.describe_edges().len(), edges.len());
}

#[test]
fn keys_name_one_node_and_describe_lists_everything() {
    let (mut g, [_, a, _, _]) = diamond();
    assert_eq!(g.node(ch(1, "a")), a);
    assert_eq!(g.id(&ch(1, "a")), Some(a));
    assert_eq!(g.id(&ch(2, "a")), None);
    assert_eq!(g.key(a).item(), Some(1));
    assert_eq!(g.key(a).label(), "Channel 1 a");
    assert_eq!(g.describe_nodes().len(), 4);
    assert_eq!(g.describe_edges().len(), 4);
    assert_eq!(g.mark_key_dirty(&ch(9, "missing")), 0);
}

#[test]
fn a_long_chain_pulls_without_running_out_of_stack() {
    let mut g = EvalGraph::<f64>::new();
    let mut prev = g.node(NodeKey::Time);
    g.set(prev, 0.);
    for i in 0..200_000 {
        let n = g.node(ch(i, "x"));
        g.connect(prev, n).unwrap();
        prev = n;
    }
    let v = g.pull(prev, &mut |_, inputs| inputs[0] + 1.);
    assert_eq!(v, 200_000.);
}

#[test]
fn undo_style_all_dirty_recomputes_everything_once() {
    let (mut g, [_, _, _, c]) = diamond();
    g.pull(c, &mut compute);
    g.mark_all_dirty();
    let before = g.computes();
    // Time is a source with a value: recomputed through compute too, as
    // the caller asked for everything.
    g.pull(c, &mut |k, i| match k {
        NodeKey::Time => 3.,
        _ => compute(k, i),
    });
    assert_eq!(g.computes() - before, 4);
}
