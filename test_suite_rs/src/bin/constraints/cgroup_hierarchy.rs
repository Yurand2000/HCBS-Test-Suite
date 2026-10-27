use std::collections::{HashMap, HashSet};

use hcbs_test_suite::{cgroup::cgroup_v2::create_cgroup_cpuset, prelude::*};
use graphviz_rust::dot_structures::*;
use libtux::pid::cpuset::CpuSet;

#[derive(clap::Parser, Debug)]
pub struct MyArgs {
    /// if given, must fail
    #[arg(short = 'f')]
    pub must_fail: bool,

    /// multicpu graph file
    #[arg(long = "multi")]
    pub multicpu: bool,

    /// graph file
    pub graph_file: String,
}

pub fn main(args: MyArgs) -> anyhow::Result<()> {
    let graph = std::fs::read_to_string(&args.graph_file)?;
    let graph = parse_hcbs_graph(&graph, args.multicpu)?;

    batch_test_header(&args.graph_file, "cgroup hierarchy");

    let multicpu = is_multicpu_enabled()?;

    let res =
        if args.must_fail {
            if test_hcbs_graph(&graph, multicpu, args.multicpu).is_err() {
                Ok(())
            } else {
                Err(anyhow::format_err!("Expected hierarchy setup failure"))
            }
        } else {
            test_hcbs_graph(&graph, multicpu, args.multicpu)
        };

    batch_test_result(res)?;

    Ok(())
}

#[derive(Debug)]
pub struct HCBSNode {
    id: String,
    config: CgroupConfig,
    internal: std::collections::HashMap<CpuID, Time>,
    children: Vec<HCBSNode>,
}

fn test_hcbs_graph(graph: &HCBSNode, multicpu: bool, multicpu_graph: bool) -> anyhow::Result<()> {
    create_hcbs_graph(graph, "", multicpu_graph)?;
    test_internal_runtime_hcbs_graph(graph, "", multicpu)?;
    destroy_hcbs_graph(graph, "")?;

    Ok(())
}

fn create_hcbs_graph(graph: &HCBSNode, parent: &str, multicpu_graph: bool) -> anyhow::Result<()> {
    let name =
        if parent == "" {
            graph.id.to_owned()
        } else {
            format!("{}/{}", parent, graph.id)
        };

    if multicpu_graph {
        create_cgroup_cpuset(&name)?;
    } else {
        create_cgroup(&name)?;
    }

    let err = {
        if let Some(cpuset) = &graph.config.cpuset {
            set_cgroup_cpumask(&name, cpuset)?;
        }

        match &graph.config.config {
            CgroupTimeConfig::Max => {
                set_cgroup_max(&name)?;
            },
            CgroupTimeConfig::Time { default: (runtime, period), overrides } => {
                set_cgroup_us(&name, *runtime, *period)?;
                for (cpu, (runtime, period)) in overrides {
                    set_cgroup_override(&name, &CpuSet::single(*cpu), *runtime, *period)?;
                }
            },
        }

        for child in graph.children.iter() {
            create_hcbs_graph(child, &name, multicpu_graph)?;
        }

        Ok(())
    };

    if err.is_err() {
        delete_cgroup(&name)?;
    }
    err
}

fn test_internal_runtime_hcbs_graph(graph: &HCBSNode, parent: &str, multicpu: bool) -> anyhow::Result<()> {
    let name =
        if parent == "" {
            return Ok(());
        } else {
            format!("{}/{}", parent, graph.id)
        };

    let file = format!("/sys/fs/cgroup/{name}/cpu.rt.internal");

    if multicpu {
        let parse_single = |s: &str| -> anyhow::Result<_> {
            let mut s = s.split_whitespace();
            let cpu = s.next()
                .ok_or_else(|| anyhow::format_err!("Expected cpuid"))
                .and_then(|s| Ok(CpuID(s.parse::<u32>()?)))?;

            let runtime = s.next()
                .ok_or_else(|| anyhow::format_err!("Expected runtime"))
                .and_then(|s| s.strip_prefix("runtime=").ok_or_else(|| anyhow::format_err!("Expected runtime")))
                .and_then(|s| Ok(Time::millis(s.parse::<f64>()?)))?;

            let period = s.next()
                .ok_or_else(|| anyhow::format_err!("Expected period"))
                .and_then(|s| s.strip_prefix("period=").ok_or_else(|| anyhow::format_err!("Expected period")))
                .and_then(|s| Ok(Time::millis(s.parse::<f64>()?)))?;

            Ok((cpu, runtime, period))
        };

        let actual = __read_file_parse(&file, |data| -> anyhow::Result<_> {
            let mut actual = std::collections::HashMap::new();
            for line in data.split_terminator('\n') {
                let (cpu, runtime, _) = parse_single(line)?;

                actual.insert(cpu, runtime);
            }

            Ok(actual)
        })?;

        for (cpu, expected) in graph.internal.iter() {
            let actual = actual.get(cpu).unwrap();

            if actual != expected {
                anyhow::bail!("Expected internal runtime for node {} (cpu: {}) to be {}, got {}", graph.id, cpu, expected, actual);
            }
        }
    } else {
        let actual = __read_file_parse(&file, |data| data.parse::<f64>().map(|time| Time::micros(time)) )?;

        for (cpu, expected) in graph.internal.iter() {
            if actual != *expected {
                anyhow::bail!("Expected internal runtime for node {} (cpu: {}) to be {}, got {}", graph.id, cpu, expected, actual);
            }
        }
    }

    Ok(())
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

fn parse_hcbs_graph(dot: &str, multicpu_graph: bool) -> anyhow::Result<HCBSNode> {
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
        config: CgroupConfig,
        internal: std::collections::HashMap<CpuID, Time>,
        parent: Option<String>,
        children: HashSet<String>,
    }

    let mut nodes = nodes.into_iter()
        .map(|node| {
            let id = node.id.0.to_string();
            let (mut runtime, mut period) = (Time::zero(), Time::zero());
            let mut overrides = std::collections::HashMap::new();
            let mut is_max = false;
            let mut cpuset = None;
            let mut internal = std::collections::HashMap::new();

            for Attribute(key, value) in node.attributes {
                let key = key.to_string();
                let value =
                    match value {
                        Id::Html(v) => panic!("unexpected: {v}"),
                        Id::Escaped(v) => v.strip_prefix('"').unwrap().strip_suffix('"').unwrap().to_owned(),
                        Id::Plain(v) => v,
                        Id::Anonymous(v) => panic!("unexpected: {v}"),
                    };

                let value = value.to_string();

                if key == "runtime" {
                    if value == "max" {
                        is_max = true;
                    } else {
                        is_max = false;
                        runtime = Time::millis(value.parse().map_err(|err| anyhow::format_err!("Error parsing runtime attribute: {err}"))?);
                    }
                } else if key == "period" {
                    period = Time::millis(value.parse().map_err(|err| anyhow::format_err!("Error parsing period attribute: {err}"))?);
                } else if key == "cpuset" {
                    cpuset = Some(value.parse().map_err(|err| anyhow::format_err!("Error parsing cpuset attribute (value {value}): {err}"))?);
                } else if key == "internal" {
                    if multicpu_graph {
                        for v in value.split('|') {
                            let mut v = v.split_whitespace();
                            let cpuset = v.next()
                                .ok_or(anyhow::format_err!("Expected Cpuset for internal"))?
                                .parse::<CpuSet>()
                                .map_err(|err| anyhow::format_err!("Expected Cpuset for internal: {err}"))?;

                            let runtime = v.next()
                                .ok_or(anyhow::format_err!("Expected runtime for internal"))?
                                .parse::<f64>()
                                .map(|t| Time::millis(t))
                                .map_err(|err| anyhow::format_err!("Expected runtime for internal: {err}"))?;

                            for cpu in cpuset.iter() {
                                internal.insert(*cpu, runtime);
                            }
                        }
                    } else {
                        let runtime = value.parse::<f64>()
                            .map(|t| Time::millis(t))
                            .map_err(|err| anyhow::format_err!("Expected runtime for internal: {err}"))?;

                        for cpu in CpuSet::online()?.iter() {
                            internal.insert(*cpu, runtime);
                        }
                    }
                } else if key == "override" {
                    for v in value.split('|') {
                        let mut v = v.split_whitespace();
                        let cpuset = v.next()
                            .ok_or(anyhow::format_err!("Expected Cpuset for override"))?
                            .parse::<CpuSet>()
                            .map_err(|err| anyhow::format_err!("Expected Cpuset for override: {err}"))?;

                        let runtime = v.next()
                            .ok_or(anyhow::format_err!("Expected runtime for override"))?
                            .parse::<f64>()
                            .map(|t| Time::millis(t))
                            .map_err(|err| anyhow::format_err!("Expected runtime for override: {err}"))?;

                        let period = v.next()
                            .ok_or(anyhow::format_err!("Expected period for override"))?
                            .parse::<f64>()
                            .map(|t| Time::millis(t))
                            .map_err(|err| anyhow::format_err!("Expected period for override: {err}"))?;

                        for cpu in cpuset.iter() {
                            overrides.insert(*cpu, (runtime, period));
                        }
                    }
                }
            }

            Ok((id, HCBSNodeTmp {
                config: CgroupConfig {
                    cpuset,
                    config: if is_max {
                        CgroupTimeConfig::Max
                    } else {
                        CgroupTimeConfig::Time {
                            default: (runtime, period),
                            overrides: overrides,
                        }
                    }
                },
                internal,
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
            config: node.config,
            internal: node.internal,
            children: Vec::new(),
        });
    }

    while !nodes.is_empty() {
        let (new_id, new_node) = nodes.extract_if(|_, node| {
            node.children.iter().all(|key| frontier.contains_key(key))
        }).next().unwrap();

        let new_node = HCBSNode {
            id: new_id.clone(),
            config: new_node.config,
            internal: new_node.internal,
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
