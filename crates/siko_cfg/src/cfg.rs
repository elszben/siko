use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::function::FunctionId;
use siko_ir::pattern::Pattern;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
use siko_util::dot::Graph;
use siko_util::Counter;
use std::collections::BTreeMap;

pub struct BasicBlock {
    id: usize,
    source: Option<usize>,
    exprs: Vec<ExprId>,
    patterns: Vec<PatternId>,
}

impl BasicBlock {
    pub fn new(id: usize, source: Option<usize>) -> BasicBlock {
        BasicBlock {
            id: id,
            source: source,
            exprs: Vec::new(),
            patterns: Vec::new(),
        }
    }
}

pub struct ControlFlowGraph {
    name: String,
    next_block: Counter,
    blocks: BTreeMap<usize, BasicBlock>,
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

    fn process_expr(&mut self, expr_id: ExprId, program: &Program, block: &mut BasicBlock) {
        block.exprs.push(expr_id);
        let expr = &program.exprs.get(&expr_id).item;
        match expr {
            Expr::ArgRef(_) => {}
            Expr::Bind(_, rhs) => {
                self.process_expr(*rhs, program, block);
            }
            Expr::CaseOf(case_expr, cases, _) => {
                for case in cases {
                    self.process_block(case.body, program, Some(block.id));
                }
                block.exprs.push(*case_expr);
            }
            Expr::ClassFunctionCall(_, args) => {
                for arg in args {
                    self.process_expr(*arg, program, block);
                }
            }
            Expr::DynamicFunctionCall(_, args) => {
                for arg in args {
                    self.process_expr(*arg, program, block);
                }
            }
            Expr::Do(items) => {
                for item in items {
                    self.process_expr(*item, program, block);
                }
            }
            Expr::ExprValue(_, _) => {}
            Expr::FieldAccess(_, receiver_expr_id) => {
                self.process_expr(*receiver_expr_id, program, block);
            }
            Expr::FloatLiteral(_) => {}
            Expr::Formatter(_, args) => {
                for arg in args {
                    self.process_expr(*arg, program, block);
                }
            }
            Expr::If(cond, true_branch, false_branch) => {
                self.process_expr(*cond, program, block);
                self.process_block(*true_branch, program, Some(block.id));
                self.process_block(*false_branch, program, Some(block.id));
            }
            Expr::IntegerLiteral(_) => {}
            Expr::List(items) => {
                for item in items {
                    self.process_expr(*item, program, block);
                }
            }
            Expr::StaticFunctionCall(_, args) => {
                for arg in args {
                    self.process_expr(*arg, program, block);
                }
            }
            Expr::StringLiteral(_) => {}
            Expr::RecordInitialization(_, items) => {
                for item in items {
                    self.process_expr(item.expr_id, program, block);
                }
            }
            Expr::RecordUpdate(receiver_expr_id, updates) => {
                self.process_expr(*receiver_expr_id, program, block);
                for update in updates {
                    for item in &update.items {
                        self.process_block(item.expr_id, program, Some(block.id));
                    }
                }
            }
            Expr::Tuple(items) => {
                for item in items {
                    self.process_expr(*item, program, block);
                }
            }
            Expr::TupleFieldAccess(_, receiver_expr_id) => {
                self.process_expr(*receiver_expr_id, program, block);
            }
        }
    }

    fn process_block(&mut self, expr_id: ExprId, program: &Program, source: Option<usize>) {
        let block_id = self.next_block.next();
        let mut block = BasicBlock::new(block_id, source);
        self.process_expr(expr_id, program, &mut block);
        self.blocks.insert(block_id, block);
    }

    pub fn process_functions(program: &Program) -> BTreeMap<FunctionId, ControlFlowGraph> {
        let mut cfgs = BTreeMap::new();
        for (id, _) in program.functions.items.iter() {
            if let Some(cfg) = ControlFlowGraph::create(program, *id) {
                let dot_graph = cfg.to_dot_graph();
                dot_graph.generate_dot().expect("CFG dump failed");
                cfgs.insert(*id, cfg);
            }
        }
        cfgs
    }

    pub fn to_dot_graph(&self) -> Graph {
        let mut graph = Graph::new(self.name.clone());
        let mut index_map = BTreeMap::new();
        for (id, _) in &self.blocks {
            let block_name = format!("BB{}", id);
            let block_index = graph.add_node(block_name.clone());
            index_map.insert(id, block_index);
        }
        for (id, block) in &self.blocks {
            if let Some(source) = block.source {
                let to_index = index_map.get(&id).expect("To index not found");
                let source_index = index_map.get(&source).expect("Source index not found");
                graph.add_edge(format!(" "), *source_index, *to_index);
            }
        }
        graph
    }
}
