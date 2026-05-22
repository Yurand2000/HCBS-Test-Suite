use std::collections::{HashMap, HashSet};

use hcbs_utils::prelude::*;
use hcbs_test_suite::utils::*;
use graphviz_rust::dot_structures::*;

#[derive(clap::Parser, Debug)]
pub struct MyArgs {
    /// if given, must fail
    #[arg(short = 'f')]
    pub must_fail: bool,

    /// graph file
    pub graph_file: String,
}

pub fn main(args: MyArgs) -> anyhow::Result<()> {
    let graph = std::fs::read_to_string(&args.graph_file)?;
    let graph = parse_hcbs_graph(&graph)?;

    batch_test_header(&args.graph_file, "cgroup hierarchy");

    let res =
        if args.must_fail {
            if test_hcbs_graph(&graph).is_err() {
                Ok(())
            } else {
                Err(anyhow::format_err!("Expected hierarchy setup failure"))
            }
        } else {
            test_hcbs_graph(&graph)
        };

    batch_test_result(res)?;

    Ok(())
}

#[derive(Debug)]
pub struct HCBSNode {
    id: String,
    runtime: Either<u64, Max>,
    period: u64,
    children: Vec<HCBSNode>,
}

fn test_hcbs_graph(graph: &HCBSNode) -> anyhow::Result<()> {
    create_hcbs_graph(graph, "")?;
    destroy_hcbs_graph(graph, "")?;

    Ok(())
}

fn create_hcbs_graph(graph: &HCBSNode, parent: &str) -> anyhow::Result<()> {
    let name =
        if parent == "" {
            graph.id.to_owned()
        } else {
            format!("{}/{}", parent, graph.id)
        };

    create_cgroup(&name)?;
    let err = {
        set_cgroup_us(&name, graph.runtime, graph.period)?;
        for child in graph.children.iter() {
            create_hcbs_graph(child, &name)?;
        }

        Ok(())
    };

    if err.is_err() {
        delete_cgroup(&name)?;
    }
    err
}

fn destroy_hcbs_graph(graph: &HCBSNode, parent: &str) -> anyhow::Result<()> {
    let name =
        if parent == "" {
            graph.id.to_owned()
        } else {
            format!("{}/{}", parent, graph.id)
        };

    for child in graph.children.iter() {
        destroy_hcbs_graph(child, &name)?;
    }

    delete_cgroup(&name)
}

fn parse_hcbs_graph(dot: &str) -> anyhow::Result<HCBSNode> {
    let graph = graphviz_rust::parse(dot)
        .map_err(|err| anyhow::format_err!("Error in parsing DOT string: {err}"))?;

    let Graph::DiGraph { id: _, strict: true, stmts } = graph
        else { anyhow::bail!("Expected a strict DiGraph") };

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for stmt in stmts {
        match stmt {
            Stmt::Node(node) => { nodes.push(node); },
            Stmt::Edge(edge) => { edges.push(edge); },
            Stmt::Attribute(Attribute(id, value)) =>
                anyhow::bail!("Unexpected Attribute for graph: {id} = {value}"),
            Stmt::GAttribute(gattr) =>
                anyhow::bail!("Unexpected GAttribute for graph: {gattr:?}"),
            Stmt::Subgraph(subgraph) =>
                anyhow::bail!("Unexpected Subgraph for graph: {subgraph:?}"),
        };
    }

    #[derive(Debug)]
    struct HCBSNodeTmp {
        runtime: Either<u64, Max>,
        period: u64,
        parent: Option<String>,
        children: HashSet<String>,
    }

    let mut nodes = nodes.into_iter()
        .map(|node| {
            let id = node.id.0.to_string();
            let mut runtime = None;
            let mut period = None;

            for Attribute(key, value) in node.attributes {
                let key = key.to_string();
                let value = value.to_string();

                if key == "runtime" {
                    if value == "max" {
                        runtime = Some(Either::Right(Max));
                    } else {
                        runtime = Some(Either::Left(value.parse::<u64>()? * 1000));
                    }
                } else if key == "period" {
                    period = Some(value.parse::<u64>()? * 1000);
                }
            }

            let Some(runtime) = runtime
                else { anyhow::bail!("Expected runtime for node {id}") };

            let Some(period) = period
                else { anyhow::bail!("Expected period for node {id}") };

            Ok((id, HCBSNodeTmp {
                runtime,
                period,
                parent: None,
                children: HashSet::new(),
            }))
        })
        .collect::<anyhow::Result<HashMap<_, _>>>()
        .map_err(|err| anyhow::format_err!("Error in parsing node: {err}") )?;

    for edge in edges.into_iter() {
        let chain =
            match edge.ty {
                EdgeTy::Pair(start, end) => vec![start, end],
                EdgeTy::Chain(chain) => chain,
            };

        for window in chain.windows(2) {
            let &Vertex::N(NodeId(Id::Plain(ref start), _)) = &window[0]
                else { anyhow::bail!("Expected node id for edge chain, got {:?}", &window[0]) };
            let &Vertex::N(NodeId(Id::Plain(ref end), _)) = &window[1]
                else { anyhow::bail!("Expected node id for edge chain, got {:?}", &window[1]) };

            nodes.get_mut(start)
                .ok_or_else(|| anyhow::format_err!("Node {start} not declared") )?
                .children.insert(end.clone());

            if nodes.get_mut(end)
                .ok_or_else(|| anyhow::format_err!("Node {end} not declared") )?
                .parent.replace(start.clone())
                .is_some() {
                    anyhow::bail!("Node {end} already had a parent");
            }

            if nodes.get(end).unwrap()
                .children.contains(start) {
                anyhow::bail!("Detected loop between nodes '{start}' and '{end}'");
            }
        }
    }

    if nodes.iter()
        .filter(|node| node.1.parent.is_none())
        .count() != 1 {
            anyhow::bail!("Graph has more than one connected component");
    }

    let mut frontier = HashMap::new();
    for (id, node) in nodes.extract_if(|_, node| node.children.is_empty()) {
        frontier.insert(id.clone(), HCBSNode {
            id: id,
            runtime: node.runtime,
            period: node.period,
            children: Vec::new(),
        });
    }

    while !nodes.is_empty() {
        let (new_id, new_node) = nodes.extract_if(|_, node| {
            node.children.iter().all(|key| frontier.contains_key(key))
        }).next().unwrap();

        let new_node = HCBSNode {
            id: new_id.clone(),
            runtime: new_node.runtime,
            period: new_node.period,
            children: frontier
                .extract_if(|id, _| new_node.children.contains(id))
                .map(|(_, node)| node)
                .collect(),
        };

        frontier.insert(new_id, new_node);
    }

    assert!(frontier.len() == 1, "Frontier: {frontier:#?}");

    let (root_id, mut root_node) =
        frontier.into_iter().next().unwrap();

    if root_id != "root" {
        anyhow::bail!("Root node must be named 'root'");
    }

    root_node.id = ".".to_owned();
    Ok(root_node)
}
