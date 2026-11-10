//! `echo-codegen`：把受限 YAML 数据流编译成静态调度图（Rust 源码）。
//!
//! 设计契约见 `docs/15-codegen.md`。
//!
//! v0 只做 **调度表 + 边元数据**，不生成任务体：
//! 应用通过 `task_defs(entries)` 注入入口函数，codegen 保证顺序/预算/边与 YAML 一致。

use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Graph {
    pub frame_ns: u64,
    pub nodes: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub name: String,
    pub slot_ns: u64,
    pub inputs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodegenError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for CodegenError {}

fn err(line: usize, message: impl Into<String>) -> CodegenError {
    CodegenError {
        line,
        message: message.into(),
    }
}

fn parse_u64(raw: &str, line: usize, key: &str) -> Result<u64, CodegenError> {
    raw.parse::<u64>()
        .map_err(|_| err(line, format!("invalid {key}: {raw}")))
}

fn parse_list(raw: &str, line: usize) -> Result<Vec<String>, CodegenError> {
    if !raw.starts_with('[') || !raw.ends_with(']') {
        return Err(err(line, "inputs must look like [a, b]"));
    }
    let inner = raw[1..raw.len() - 1].trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }

    let mut inputs = Vec::new();
    for part in inner.split(',') {
        let name = part.trim();
        if name.is_empty() {
            return Err(err(line, "empty input name"));
        }
        inputs.push(name.to_string());
    }
    Ok(inputs)
}

/// 解析受限 YAML。
///
/// 支持的语法见 `docs/15-codegen.md`；不是通用 YAML 解析器。
pub fn parse(input: &str) -> Result<Graph, CodegenError> {
    let mut frame_ns: Option<u64> = None;
    let mut nodes: Vec<Node> = Vec::new();
    let mut in_nodes = false;
    let mut current: Option<usize> = None;

    for (index, raw) in input.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if !in_nodes {
            if let Some(rest) = line.strip_prefix("frame_ns:") {
                if frame_ns.is_some() {
                    return Err(err(line_no, "duplicate frame_ns"));
                }
                frame_ns = Some(parse_u64(rest.trim(), line_no, "frame_ns")?);
            } else if line == "nodes:" {
                in_nodes = true;
            } else {
                return Err(err(line_no, "expected 'frame_ns:' or 'nodes:'"));
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("- ") {
            let rest = rest.trim();
            let Some(name) = rest.strip_prefix("name:") else {
                return Err(err(line_no, "node entry must start with '- name:'"));
            };
            let name = name.trim();
            if name.is_empty() {
                return Err(err(line_no, "empty node name"));
            }
            if nodes.iter().any(|node| node.name == name) {
                return Err(err(line_no, format!("duplicate node name: {name}")));
            }
            nodes.push(Node {
                name: name.to_string(),
                slot_ns: 0,
                inputs: Vec::new(),
            });
            current = Some(nodes.len() - 1);
        } else if let Some(rest) = line.strip_prefix("slot_ns:") {
            let index = current.ok_or_else(|| err(line_no, "slot_ns before any node"))?;
            if nodes[index].slot_ns != 0 {
                return Err(err(line_no, "duplicate slot_ns"));
            }
            let value = parse_u64(rest.trim(), line_no, "slot_ns")?;
            if value == 0 {
                return Err(err(line_no, "slot_ns must be > 0"));
            }
            nodes[index].slot_ns = value;
        } else if let Some(rest) = line.strip_prefix("inputs:") {
            let index = current.ok_or_else(|| err(line_no, "inputs before any node"))?;
            nodes[index].inputs = parse_list(rest.trim(), line_no)?;
        } else {
            return Err(err(line_no, "unexpected key"));
        }
    }

    let frame_ns = frame_ns.ok_or_else(|| err(0, "missing frame_ns"))?;
    if frame_ns == 0 {
        return Err(err(0, "frame_ns must be > 0"));
    }
    if nodes.is_empty() {
        return Err(err(0, "at least one node required"));
    }

    let mut slot_total = 0u64;
    for node in &nodes {
        if node.slot_ns == 0 {
            return Err(err(0, format!("node '{}' missing slot_ns", node.name)));
        }
        slot_total = slot_total
            .checked_add(node.slot_ns)
            .ok_or_else(|| err(0, "slot_ns sum overflow"))?;
    }
    if slot_total > frame_ns {
        return Err(err(
            0,
            format!("slot sum {slot_total} exceeds frame {frame_ns}"),
        ));
    }

    // 边只能指向更早的节点：YAML 顺序即调度顺序。
    for (index, node) in nodes.iter().enumerate() {
        for input in &node.inputs {
            if nodes[..index].iter().any(|earlier| &earlier.name == input) {
                continue;
            }
            return Err(err(
                0,
                format!(
                    "node '{}' input '{}' is unknown or forward",
                    node.name, input
                ),
            ));
        }
    }

    Ok(Graph { frame_ns, nodes })
}

/// 生成 `static_graph.rs` 内容。
pub fn generate(graph: &Graph, source: &str) -> String {
    let mut out = String::new();
    writeln!(
        out,
        "// @generated by tools/codegen from {source}. Do not edit."
    )
    .unwrap();
    writeln!(out).unwrap();
    writeln!(out, "pub const FRAME_NS: u64 = {};", graph.frame_ns).unwrap();
    writeln!(out, "pub const NODE_COUNT: usize = {};", graph.nodes.len()).unwrap();

    writeln!(out, "pub const NODE_NAMES: [&str; NODE_COUNT] = [").unwrap();
    for node in &graph.nodes {
        writeln!(out, "    {:?},", node.name).unwrap();
    }
    writeln!(out, "];").unwrap();

    writeln!(out, "pub const NODE_SLOTS_NS: [u64; NODE_COUNT] = [").unwrap();
    for node in &graph.nodes {
        writeln!(out, "    {},", node.slot_ns).unwrap();
    }
    writeln!(out, "];").unwrap();

    let edges: Vec<(&str, &str)> = graph
        .nodes
        .iter()
        .flat_map(|node| {
            node.inputs
                .iter()
                .map(move |input| (input.as_str(), node.name.as_str()))
        })
        .collect();

    writeln!(out, "pub const EDGES: [(&str, &str); {}] = [", edges.len()).unwrap();
    for (from, to) in &edges {
        writeln!(out, "    ({:?}, {:?}),", from, to).unwrap();
    }
    writeln!(out, "];").unwrap();
    writeln!(out).unwrap();

    writeln!(
        out,
        "pub fn task_defs(entries: [fn(); NODE_COUNT]) -> [echo_sched::TaskDef; NODE_COUNT] {{"
    )
    .unwrap();
    writeln!(out, "    [").unwrap();
    for index in 0..graph.nodes.len() {
        writeln!(
            out,
            "        echo_sched::TaskDef::new(NODE_NAMES[{index}], NODE_SLOTS_NS[{index}], entries[{index}]),"
        )
        .unwrap();
    }
    writeln!(out, "    ]").unwrap();
    writeln!(out, "}}").unwrap();

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = "\
frame_ns: 1000
nodes:
  - name: sensor
    slot_ns: 200
  - name: control
    slot_ns: 100
    inputs: [sensor]
  - name: actuator
    slot_ns: 50
    inputs: [control]
";

    #[test]
    fn parses_three_node_graph() {
        let graph = parse(YAML).expect("valid yaml");
        assert_eq!(graph.frame_ns, 1000);
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.nodes[0].name, "sensor");
        assert_eq!(graph.nodes[1].inputs, ["sensor"]);
        assert_eq!(graph.nodes[2].inputs, ["control"]);
    }

    #[test]
    fn rejects_forward_edge() {
        let yaml = "\
frame_ns: 1000
nodes:
  - name: control
    slot_ns: 100
    inputs: [actuator]
  - name: actuator
    slot_ns: 50
";
        let error = parse(yaml).expect_err("forward edge must fail");
        assert!(error.message.contains("unknown or forward"));
    }

    #[test]
    fn rejects_duplicate_name() {
        let yaml = "\
frame_ns: 1000
nodes:
  - name: sensor
    slot_ns: 100
  - name: sensor
    slot_ns: 100
";
        let error = parse(yaml).expect_err("duplicate name must fail");
        assert!(error.message.contains("duplicate node name"));
    }

    #[test]
    fn rejects_budget_overrun() {
        let yaml = "\
frame_ns: 1000
nodes:
  - name: a
    slot_ns: 600
  - name: b
    slot_ns: 500
";
        let error = parse(yaml).expect_err("overrun must fail");
        assert!(error.message.contains("exceeds frame"));
    }

    #[test]
    fn rejects_unknown_input() {
        let yaml = "\
frame_ns: 1000
nodes:
  - name: control
    slot_ns: 100
    inputs: [missing]
";
        let error = parse(yaml).expect_err("unknown input must fail");
        assert!(error.message.contains("unknown or forward"));
    }

    #[test]
    fn generates_expected_shape() {
        let graph = parse(YAML).expect("valid yaml");
        let code = generate(&graph, "three_node.yaml");
        assert!(code.contains("pub const FRAME_NS: u64 = 1000;"));
        assert!(code.contains("pub const NODE_COUNT: usize = 3;"));
        assert!(code.contains("(\"sensor\", \"control\")"));
        assert!(code.contains("echo_sched::TaskDef::new(NODE_NAMES[0]"));
    }

    #[test]
    fn checked_in_conformance_file_is_current() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let yaml = std::fs::read_to_string(root.join("dataflows/three_node.yaml"))
            .expect("dataflow fixture must exist");
        let checked_in = std::fs::read_to_string(root.join("dataflows/generated/three_node.rs"))
            .expect("generated conformance file must exist");

        let graph = parse(&yaml).expect("fixture must parse");
        assert_eq!(
            generate(&graph, "dataflows/three_node.yaml"),
            checked_in,
            "regenerate with: cargo run -p echo-codegen -- dataflows/three_node.yaml \
             dataflows/generated/three_node.rs"
        );
    }
}
