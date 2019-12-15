use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::function::FunctionId;
use siko_ir::pattern::Pattern;
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

pub struct BasicBlock {
    id: BlockId,
    sources: Vec<BlockId>,
    elements: Vec<BlockElement>,
}

impl BasicBlock {
    pub fn new(id: BlockId) -> BasicBlock {
        BasicBlock {
            id: id,
            sources: Vec::new(),
            elements: Vec::new(),
        }
    }
}

pub struct ControlFlowGraph {
    name: String,
    next_block: Counter,
    blocks: BTreeMap<BlockId, BasicBlock>,
}

impl ControlFlowGraph {
    fn create(program: &Program, function_id: FunctionId) -> Option<ControlFlowGraph> {
        let function = program.functions.get(&function_id);
        let name = format!("{}", function.info);
        let name = name.replace("/", "_");
        let mut cfg = ControlFlowGraph {
            name: name,
            next_block: Counter::new(),
            blocks: BTreeMap::new(),
        };
        if let Some(body) = function.get_body() {
            cfg.process_block(body, program, None);
            Some(cfg)
        } else {
            None
        }
    }

    fn add_expr_to_block(&mut self, block_id: BlockId, expr_id: ExprId) {
        let block = self.blocks.get_mut(&block_id).expect("Block not found");
        block.elements.push(BlockElement::Expr(expr_id));
    }

    fn add_terminator_to_block(&mut self, block_id: BlockId, expr_id: ExprId) {
        let block = self.blocks.get_mut(&block_id).expect("Block not found");
        block.elements.push(BlockElement::Terminator(expr_id));
    }

    fn add_source(&mut self, block_id: BlockId, source: BlockId) {
        let block = self.blocks.get_mut(&block_id).expect("Block not found");
        block.sources.push(source);
    }

    fn process_expr(&mut self, expr_id: ExprId, program: &Program, block_id: BlockId) -> BlockId {
        let expr = &program.exprs.get(&expr_id).item;
        match expr {
            Expr::ArgRef(_) => {
                self.add_expr_to_block(block_id, expr_id);
                block_id
            }
            Expr::Bind(_, rhs) => {
                let block_id = self.process_expr(*rhs, program, block_id);
                self.add_expr_to_block(block_id, expr_id);
                block_id
            }
            Expr::CaseOf(case_expr, cases, _) => {
                let block_id = self.process_expr(*case_expr, program, block_id);
                self.add_expr_to_block(block_id, expr_id);
                let next = self.create_block();
                for case in cases {
                    let case_block_id = self.process_block(case.body, program, Some(block_id));
                    self.add_source(next, case_block_id);
                }
                next
            }
            Expr::ClassFunctionCall(_, args) => {
                let mut block_id = block_id;
                for arg in args {
                    block_id = self.process_expr(*arg, program, block_id);
                }
                self.add_expr_to_block(block_id, expr_id);
                block_id
            }
            Expr::DynamicFunctionCall(_, args) => {
                let mut block_id = block_id;
                for arg in args {
                    block_id = self.process_expr(*arg, program, block_id);
                }
                self.add_expr_to_block(block_id, expr_id);
                block_id
            }
            Expr::Do(items) => {
                let mut block_id = block_id;
                self.add_expr_to_block(block_id, expr_id);
                let next = self.create_block();
                self.add_source(next, block_id);
                block_id = next;
                for item in items {
                    block_id = self.process_expr(*item, program, block_id);
                }
                self.add_terminator_to_block(block_id, expr_id);
                block_id
            }
            Expr::ExprValue(_, _) => {
                self.add_expr_to_block(block_id, expr_id);
                block_id
            }
            Expr::FieldAccess(_, receiver_expr_id) => {
                let block_id = self.process_expr(*receiver_expr_id, program, block_id);
                self.add_expr_to_block(block_id, expr_id);
                block_id
            }
            Expr::FloatLiteral(_) => {
                self.add_expr_to_block(block_id, expr_id);
                block_id
            }
            Expr::Formatter(_, args) => {
                let mut block_id = block_id;
                for arg in args {
                    block_id = self.process_expr(*arg, program, block_id);
                }
                self.add_expr_to_block(block_id, expr_id);
                block_id
            }
            Expr::If(cond, true_branch, false_branch) => {
                let block_id = self.process_expr(*cond, program, block_id);
                let true_block_id = self.process_block(*true_branch, program, Some(block_id));
                let false_block_id = self.process_block(*false_branch, program, Some(block_id));
                self.add_expr_to_block(block_id, expr_id);
                let next = self.create_block();
                self.add_source(next, true_block_id);
                self.add_source(next, false_block_id);
                next
            }
            Expr::IntegerLiteral(_) => {
                self.add_expr_to_block(block_id, expr_id);
                block_id
            }
            Expr::List(items) => {
                let mut block_id = block_id;
                for item in items {
                    block_id = self.process_expr(*item, program, block_id);
                }
                self.add_expr_to_block(block_id, expr_id);
                block_id
            }
            Expr::StaticFunctionCall(_, args) => {
                let mut block_id = block_id;
                for arg in args {
                    block_id = self.process_expr(*arg, program, block_id);
                }
                self.add_expr_to_block(block_id, expr_id);
                block_id
            }
            Expr::StringLiteral(_) => {
                self.add_expr_to_block(block_id, expr_id);
                block_id
            }
            Expr::RecordInitialization(_, items) => {
                let mut block_id = block_id;
                for item in items {
                    block_id = self.process_expr(item.expr_id, program, block_id);
                }
                self.add_expr_to_block(block_id, expr_id);
                block_id
            }
            Expr::RecordUpdate(receiver_expr_id, updates) => {
                let mut block_id = self.process_expr(*receiver_expr_id, program, block_id);
                for update in updates {
                    for item in &update.items {
                        block_id = self.process_block(item.expr_id, program, Some(block_id));
                    }
                }
                self.add_expr_to_block(block_id, expr_id);
                block_id
            }
            Expr::Tuple(items) => {
                let mut block_id = block_id;
                for item in items {
                    block_id = self.process_expr(*item, program, block_id);
                }
                self.add_expr_to_block(block_id, expr_id);
                block_id
            }
            Expr::TupleFieldAccess(_, receiver_expr_id) => {
                let block_id = self.process_expr(*receiver_expr_id, program, block_id);
                self.add_expr_to_block(block_id, expr_id);
                block_id
            }
        }
    }

    fn create_block(&mut self) -> BlockId {
        let block_id = self.next_block.next().into();
        let block = BasicBlock::new(block_id);
        self.blocks.insert(block_id, block);
        block_id
    }

    fn process_block(
        &mut self,
        expr_id: ExprId,
        program: &Program,
        source: Option<BlockId>,
    ) -> BlockId {
        let block_id = self.create_block();
        if let Some(source) = source {
            self.add_source(block_id, source);
        }
        let new_block_id = self.process_expr(expr_id, program, block_id);
        new_block_id
    }

    pub fn process_functions(program: &Program) -> BTreeMap<FunctionId, ControlFlowGraph> {
        let mut cfgs = BTreeMap::new();
        for (id, _) in program.functions.items.iter() {
            if let Some(cfg) = ControlFlowGraph::create(program, *id) {
                let dot_graph = cfg.to_dot_graph(program);
                dot_graph.generate_dot().expect("CFG dump failed");
                cfgs.insert(*id, cfg);
            }
        }
        cfgs
    }

    pub fn to_dot_graph(&self, program: &Program) -> Graph {
        let mut graph = Graph::new(self.name.clone());
        let mut index_map = BTreeMap::new();
        for (id, block) in &self.blocks {
            let block_name = format!("BB{}", id);
            let block_index = graph.add_node(block_name.clone());
            for element in &block.elements {
                let s = match element {
                    BlockElement::Expr(expr_id) => {
                        let expr = &program.exprs.get(expr_id).item;
                        let s = format!("{}{}", expr_id, expr.as_plain_text());
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
        for (id, block) in &self.blocks {
            for source in &block.sources {
                let to_index = index_map.get(&id).expect("To index not found");
                let source_index = index_map.get(&source).expect("Source index not found");
                graph.add_edge(format!(" "), *source_index, *to_index);
            }
        }
        graph
    }
}
