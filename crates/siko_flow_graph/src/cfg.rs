use siko_ir::expr::ExprId;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
use siko_util::dot::Graph;
use siko_util::Counter;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct BlockId {
    pub id: usize,
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}", self.id)
    }
}

impl From<usize> for BlockId {
    fn from(id: usize) -> BlockId {
        BlockId { id: id }
    }
}

pub enum BlockElement {
    Expr(ExprId),
    Pattern(PatternId),
    Terminator(ExprId),
}

pub enum Edge {
    If(bool),
    Case(usize),
    Jump,
}

pub struct BasicBlock {
    elements: Vec<BlockElement>,
}

impl BasicBlock {
    pub fn new() -> BasicBlock {
        BasicBlock {
            elements: Vec::new(),
        }
    }
}

pub struct ControlFlowGraph {
    name: String,
    next_block: Counter,
    blocks: BTreeMap<BlockId, BasicBlock>,
    edges: Vec<(BlockId, BlockId, Edge)>,
}

impl ControlFlowGraph {
    pub fn new(name: String) -> ControlFlowGraph {
        ControlFlowGraph {
            name: name,
            next_block: Counter::new(),
            blocks: BTreeMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_expr_to_block(&mut self, block_id: BlockId, expr_id: ExprId) {
        let block = self.blocks.get_mut(&block_id).expect("Block not found");
        block.elements.push(BlockElement::Expr(expr_id));
    }

    pub fn add_terminator_to_block(&mut self, block_id: BlockId, expr_id: ExprId) {
        let block = self.blocks.get_mut(&block_id).expect("Block not found");
        block.elements.push(BlockElement::Terminator(expr_id));
    }

    pub fn add_edge(&mut self, from: BlockId, to: BlockId, edge: Edge) {
        self.edges.push((from, to, edge));
    }

    pub fn create_block(&mut self) -> BlockId {
        let block_id = self.next_block.next().into();
        let block = BasicBlock::new();
        self.blocks.insert(block_id, block);
        block_id
    }

    pub fn to_dot_graph(&self, program: &Program) -> Graph {
        let mut graph = Graph::new(self.name.clone());
        let mut index_map = BTreeMap::new();
        for (id, block) in &self.blocks {
            let block_name = format!("BB{}", id);
            let block_index = graph.add_node(block_name);
            for element in &block.elements {
                let s = match element {
                    BlockElement::Expr(expr_id) => {
                        let expr = &program.exprs.get(expr_id).item;
                        let s = format!("{}{}", expr_id, expr.as_plain_text(program));
                        s
                    }
                    BlockElement::Pattern(pattern_id) => unimplemented!(),
                    BlockElement::Terminator(expr_id) => {
                        let s = format!("BlockEnd{}", expr_id);
                        s
                    }
                };
                graph.add_element(block_index, s);
            }
            index_map.insert(id, block_index);
        }
        for (from, to, edge) in &self.edges {
            let s = match edge {
                Edge::If(b) => Some(format!("If/{}", b)),
                Edge::Case(i) => Some(format!("Case/{}", i)),
                Edge::Jump => None,
            };
            let to_index = index_map.get(to).expect("To index not found");
            let source_index = index_map.get(from).expect("Source index not found");
            graph.add_edge(s, *source_index, *to_index);
        }
        graph
    }
}
