pub mod cfg;
pub mod dfg;
pub mod environment;

use crate::cfg::BlockId;
use crate::cfg::ControlFlowGraph;
use crate::cfg::Edge as CfgEdge;
use crate::dfg::DataflowGraph;
use crate::dfg::Edge as DfgEdge;
use crate::dfg::ValueId;
use crate::dfg::ValueSource;
use crate::environment::CallableKind;
use crate::environment::Environment;
use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::function::FunctionId;
use siko_ir::pattern::Pattern;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
use std::collections::BTreeMap;

fn process_pattern(
    pattern_id: PatternId,
    program: &Program,
    block_id: BlockId,
    cfg: &mut ControlFlowGraph,
    dfg: &mut DataflowGraph,
    environment: &mut Environment,
) -> BlockId {
    let pattern = &program.patterns.get(&pattern_id).item;
    match pattern {
        Pattern::Binding(_) => {
            let value_id = dfg.create_value(ValueSource::Pattern(pattern_id));
            environment.add(pattern_id, value_id);
            block_id
        }
        Pattern::Guarded(item, guard_expr) => {
            process_pattern(*item, program, block_id, cfg, dfg, environment);
            let (block_id, value_id) =
                process_expr(*guard_expr, program, block_id, cfg, dfg, environment);
            block_id
        }
        Pattern::Tuple(items) => {
            for item in items {
                process_pattern(*item, program, block_id, cfg, dfg, environment);
            }
            block_id
        }
        Pattern::Variant(_, _, items) => {
            for item in items {
                process_pattern(*item, program, block_id, cfg, dfg, environment);
            }
            block_id
        }
        Pattern::IntegerLiteral(_) => block_id,
        Pattern::StringLiteral(_) => block_id,
        Pattern::FloatLiteral(_) => block_id,
        Pattern::Typed(item, _) => {
            return process_pattern(*item, program, block_id, cfg, dfg, environment);
        }
        Pattern::Record(_, items) => {
            for item in items {
                process_pattern(*item, program, block_id, cfg, dfg, environment);
            }
            block_id
        }
        Pattern::Wildcard => block_id,
    }
}

fn process_expr(
    expr_id: ExprId,
    program: &Program,
    block_id: BlockId,
    cfg: &mut ControlFlowGraph,
    dfg: &mut DataflowGraph,
    environment: &mut Environment,
) -> (BlockId, ValueId) {
    let expr = &program.exprs.get(&expr_id).item;
    match expr {
        Expr::ArgRef(arg_ref) => {
            cfg.add_expr_to_block(block_id, expr_id);
            let value_id = environment.get_arg(arg_ref);
            (block_id, value_id)
        }
        Expr::Bind(pattern_id, rhs) => {
            let (block_id, value_id) = process_expr(*rhs, program, block_id, cfg, dfg, environment);
            process_pattern(*pattern_id, program, block_id, cfg, dfg, environment);
            cfg.add_expr_to_block(block_id, expr_id);
            (block_id, value_id)
        }
        Expr::CaseOf(case_expr, cases, _) => {
            let (block_id, value_id) =
                process_expr(*case_expr, program, block_id, cfg, dfg, environment);
            cfg.add_expr_to_block(block_id, expr_id);
            let next_block_id = cfg.create_block();
            let next_value_id = dfg.create_value(ValueSource::Expr(expr_id));
            for (index, case) in cases.iter().enumerate() {
                let block_id =
                    process_pattern(case.pattern_id, program, block_id, cfg, dfg, environment);
                let (case_block_id, value_id) = process_block(
                    case.body,
                    program,
                    CfgEdge::Case(index),
                    Some(block_id),
                    cfg,
                    dfg,
                    environment,
                );
                cfg.add_edge(case_block_id, next_block_id, CfgEdge::Case(index));
                dfg.add_edge(value_id, next_value_id, DfgEdge::Case(index));
            }
            (next_block_id, next_value_id)
        }
        Expr::ClassFunctionCall(_, args) => {
            let mut block_id = block_id;
            let next_value_id = dfg.create_value(ValueSource::Expr(expr_id));
            for (index, arg) in args.iter().enumerate() {
                let (bid, arg_value_id) =
                    process_expr(*arg, program, block_id, cfg, dfg, environment);
                block_id = bid;
                dfg.add_edge(arg_value_id, next_value_id, DfgEdge::FnArg(index));
            }
            cfg.add_expr_to_block(block_id, expr_id);
            (block_id, next_value_id)
        }
        Expr::DynamicFunctionCall(_, args) => {
            let mut block_id = block_id;
            let next_value_id = dfg.create_value(ValueSource::Expr(expr_id));
            for (index, arg) in args.iter().enumerate() {
                let (bid, arg_value_id) =
                    process_expr(*arg, program, block_id, cfg, dfg, environment);
                block_id = bid;
                dfg.add_edge(arg_value_id, next_value_id, DfgEdge::FnArg(index));
            }
            cfg.add_expr_to_block(block_id, expr_id);
            (block_id, next_value_id)
        }
        Expr::Do(items) => {
            let mut block_id = block_id;
            cfg.add_expr_to_block(block_id, expr_id);
            let next = cfg.create_block();
            cfg.add_edge(block_id, next, CfgEdge::Jump);
            let mut block_value_id = None;
            block_id = next;
            for item in items {
                let (bid, value_id) = process_expr(*item, program, block_id, cfg, dfg, environment);
                block_id = bid;
                block_value_id = Some(value_id);
            }
            cfg.add_terminator_to_block(block_id, expr_id);
            (block_id, block_value_id.expect("empty do"))
        }
        Expr::ExprValue(_, pattern_id) => {
            let value_id = environment.get_value(pattern_id);
            cfg.add_expr_to_block(block_id, expr_id);
            (block_id, value_id)
        }
        Expr::FieldAccess(infos, receiver_expr_id) => {
            assert_eq!(infos.len(), 1);
            let (block_id, value_id) =
                process_expr(*receiver_expr_id, program, block_id, cfg, dfg, environment);
            let value_id = dfg.create_value(ValueSource::Expr(expr_id));
            cfg.add_expr_to_block(block_id, expr_id);
            (block_id, value_id)
        }
        Expr::FloatLiteral(_) => {
            let value_id = dfg.create_value(ValueSource::Expr(expr_id));
            cfg.add_expr_to_block(block_id, expr_id);
            (block_id, value_id)
        }
        Expr::Formatter(_, args) => {
            let mut block_id = block_id;
            for arg in args {
                let (bid, value_id) = process_expr(*arg, program, block_id, cfg, dfg, environment);
                block_id = bid;
            }
            let value_id = dfg.create_value(ValueSource::Expr(expr_id));
            cfg.add_expr_to_block(block_id, expr_id);
            (block_id, value_id)
        }
        Expr::If(cond, true_branch, false_branch) => {
            let (block_id, value_id) =
                process_expr(*cond, program, block_id, cfg, dfg, environment);
            let (true_block_id, true_value_id) = process_block(
                *true_branch,
                program,
                CfgEdge::If(true),
                Some(block_id),
                cfg,
                dfg,
                environment,
            );
            let (false_block_id, false_value_id) = process_block(
                *false_branch,
                program,
                CfgEdge::If(false),
                Some(block_id),
                cfg,
                dfg,
                environment,
            );
            let next_value_id = dfg.create_value(ValueSource::Expr(expr_id));
            cfg.add_expr_to_block(block_id, expr_id);
            let next = cfg.create_block();
            cfg.add_edge(true_block_id, next, CfgEdge::Jump);
            cfg.add_edge(false_block_id, next, CfgEdge::Jump);
            dfg.add_edge(true_value_id, next_value_id, DfgEdge::If(true));
            dfg.add_edge(false_value_id, next_value_id, DfgEdge::If(false));
            (next, value_id)
        }
        Expr::IntegerLiteral(_) => {
            let value_id = dfg.create_value(ValueSource::Expr(expr_id));
            cfg.add_expr_to_block(block_id, expr_id);
            (block_id, value_id)
        }
        Expr::List(items) => {
            let mut block_id = block_id;
            let next_value_id = dfg.create_value(ValueSource::Expr(expr_id));
            for (index, item) in items.iter().enumerate() {
                let (bid, value_id) = process_expr(*item, program, block_id, cfg, dfg, environment);
                block_id = bid;
                dfg.add_edge(value_id, next_value_id, DfgEdge::ListElement(index));
            }
            cfg.add_expr_to_block(block_id, expr_id);
            (block_id, next_value_id)
        }
        Expr::StaticFunctionCall(_, args) => {
            let mut block_id = block_id;
            let next_value_id = dfg.create_value(ValueSource::Expr(expr_id));
            for (index, arg) in args.iter().enumerate() {
                let (bid, arg_value_id) =
                    process_expr(*arg, program, block_id, cfg, dfg, environment);
                block_id = bid;
                dfg.add_edge(arg_value_id, next_value_id, DfgEdge::FnArg(index));
            }
            cfg.add_expr_to_block(block_id, expr_id);
            (block_id, next_value_id)
        }
        Expr::StringLiteral(_) => {
            let value_id = dfg.create_value(ValueSource::Expr(expr_id));
            cfg.add_expr_to_block(block_id, expr_id);
            (block_id, value_id)
        }
        Expr::RecordInitialization(_, items) => {
            let mut block_id = block_id;
            let next_value_id = dfg.create_value(ValueSource::Expr(expr_id));
            for item in items {
                let (bid, value_id) =
                    process_expr(item.expr_id, program, block_id, cfg, dfg, environment);
                block_id = bid;
                dfg.add_edge(value_id, next_value_id, DfgEdge::RecordField(item.index));
            }
            cfg.add_expr_to_block(block_id, expr_id);
            (block_id, next_value_id)
        }
        Expr::RecordUpdate(receiver_expr_id, updates) => {
            let (mut block_id, value_id) =
                process_expr(*receiver_expr_id, program, block_id, cfg, dfg, environment);
            let next_value_id = dfg.create_value(ValueSource::Expr(expr_id));
            dfg.add_edge(value_id, next_value_id, DfgEdge::RecordUpdateSource);
            assert_eq!(updates.len(), 1);
            let update = &updates[0];
            for item in &update.items {
                let (bid, value_id) =
                    process_expr(item.expr_id, program, block_id, cfg, dfg, environment);
                block_id = bid;
                dfg.add_edge(value_id, next_value_id, DfgEdge::RecordField(item.index));
            }
            cfg.add_expr_to_block(block_id, expr_id);
            (block_id, next_value_id)
        }
        Expr::Tuple(items) => {
            let mut block_id = block_id;
            for item in items {
                let (bid, value_id) = process_expr(*item, program, block_id, cfg, dfg, environment);
                block_id = bid;
            }
            let value_id = dfg.create_value(ValueSource::Expr(expr_id));
            cfg.add_expr_to_block(block_id, expr_id);
            (block_id, value_id)
        }
        Expr::TupleFieldAccess(_, receiver_expr_id) => {
            let (block_id, value_id) =
                process_expr(*receiver_expr_id, program, block_id, cfg, dfg, environment);
            let value_id = dfg.create_value(ValueSource::Expr(expr_id));
            cfg.add_expr_to_block(block_id, expr_id);
            (block_id, value_id)
        }
    }
}

fn process_block(
    expr_id: ExprId,
    program: &Program,
    edge: CfgEdge,
    source: Option<BlockId>,
    cfg: &mut ControlFlowGraph,
    dfg: &mut DataflowGraph,
    environment: &mut Environment,
) -> (BlockId, ValueId) {
    let block_id = cfg.create_block();
    if let Some(source) = source {
        cfg.add_edge(source, block_id, edge);
    }
    let (new_block_id, value_id) = process_expr(expr_id, program, block_id, cfg, dfg, environment);
    (new_block_id, value_id)
}

fn process_function(
    program: &Program,
    function_id: FunctionId,
) -> Option<(ControlFlowGraph, DataflowGraph)> {
    let function = program.functions.get(&function_id);
    if let Some(body) = function.get_body() {
        let name = format!("{}", function.info);
        let name = name.replace("/", "_");
        let cfg_name = format!("cfg_{}", name);
        let dfg_name = format!("dfg_{}", name);
        let mut cfg = ControlFlowGraph::new(cfg_name);
        let mut dfg = DataflowGraph::new(dfg_name);
        let mut args = Vec::new();
        for index in 0..(function.arg_locations.len() + function.implicit_arg_count) {
            let value_id = dfg.create_value(ValueSource::Arg(index));
            args.push(value_id);
        }
        let mut environment = Environment::new(
            CallableKind::FunctionId(function_id),
            args,
            function.implicit_arg_count,
        );
        process_block(
            body,
            program,
            CfgEdge::Jump,
            None,
            &mut cfg,
            &mut dfg,
            &mut environment,
        );
        Some((cfg, dfg))
    } else {
        None
    }
}

pub fn process_functions(program: &Program) -> BTreeMap<FunctionId, ControlFlowGraph> {
    let mut cfgs = BTreeMap::new();
    for (id, _) in program.functions.items.iter() {
        if let Some((cfg, dfg)) = process_function(program, *id) {
            let dot_graph = cfg.to_dot_graph(program);
            dot_graph.generate_dot().expect("CFG dump failed");
            let dot_graph = dfg.to_dot_graph();
            dot_graph.generate_dot().expect("DFG dump failed");
            cfgs.insert(*id, cfg);
        }
    }
    cfgs
}
