use crate::Counter;
use std::fs::File;
use std::io::Result as IoResult;
use std::io::Write;

pub struct Graph {
    pub name: String,
    pub next_index: Counter,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl Graph {
    pub fn new(name: String) -> Graph {
        Graph {
            name: name,
            next_index: Counter::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, name: String) -> usize {
        let index = self.next_index.next();
        let node = Node {
            name: name,
            index: index,
        };
        self.nodes.push(node);
        index
    }

    pub fn add_edge(&mut self, name: String, from: usize, to: usize) {
        let edge = Edge {
            name: name,
            from: from,
            to: to,
        };
        self.edges.push(edge);
    }

    pub fn generate_dot(&self) -> IoResult<()> {
        let filename = format!("dots/{}.dot", self.name);
        let mut output = File::create(filename)?;
        write!(output, "digraph D {{\n")?;
        write!(output, "node [shape=rectangle fontname=Arial];\n")?;

        for node in &self.nodes {
            write!(output, "node{} [label=\"{}\"]\n", node.index, node.name)?;
        }

        for edge in &self.edges {
            write!(
                output,
                "node{} -> node{} [label=\"{}\"]\n",
                edge.from, edge.to, edge.name
            )?;
        }

        write!(output, "}}\n")?;
        Ok(())
    }
}

pub struct Node {
    pub name: String,
    pub index: usize,
}

pub struct Edge {
    pub name: String,
    pub from: usize,
    pub to: usize,
}
