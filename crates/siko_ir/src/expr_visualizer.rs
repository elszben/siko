use crate::expr::Expr;
use crate::expr::ExprId;
use crate::function::FunctionId;
use crate::function::FunctionInfo;
use crate::pattern::Pattern;
use crate::pattern::PatternId;
use crate::program::Program;
use crate::walker::walk_expr;
use crate::walker::Visitor;
use std::fs::File;
use std::io::Result as IoResult;
use std::io::Write;

pub struct ExprVisualizer<'a> {
    program: &'a Program,
    output_file: File,
}

impl<'a> ExprVisualizer<'a> {
    pub fn new(name: String, program: &'a Program) -> ExprVisualizer<'a> {
        println!("name '{}'", name);
        ExprVisualizer {
            program: program,
            output_file: File::create(name).expect("Failed to open file"),
        }
    }

    pub fn generate(function_id: &FunctionId, program: &Program) {
        let function = program.functions.get(function_id);
        let (body, name) = match &function.info {
            FunctionInfo::NamedFunction(info) => {
                let body = info.body;
                if let Some(body) = body {
                    let name = format!("{}", info);
                    (body, name)
                } else {
                    return;
                }
            }
            FunctionInfo::Lambda(info) => {
                let body = info.body;
                let name = format!("{}", info);
                (body, name)
            }
            _ => {
                return;
            }
        };

        let mut visualizer = ExprVisualizer::new(
            format!("/home/laci/git/siko/dots/{}.dot", name.replace("/", "_")),
            program,
        );

        visualizer.header().expect("Write failed");

        walk_expr(&body, &mut visualizer);

        visualizer.footer().expect("Write failed");
    }

    fn header(&mut self) -> IoResult<()> {
        write!(self.output_file, "digraph D {{\n")?;
        write!(self.output_file, "node [shape=rectangle fontname=Arial];\n")?;
        Ok(())
    }

    fn footer(&mut self) -> IoResult<()> {
        write!(self.output_file, "}}\n")?;
        Ok(())
    }
}

impl<'a> Visitor for ExprVisualizer<'a> {
    fn get_program(&self) -> &Program {
        self.program
    }
    fn visit_expr(&mut self, expr_id: ExprId, expr: &Expr) {
        let label = match expr {
            Expr::StaticFunctionCall(id, args) => {
                for (index, arg) in args.iter().enumerate() {
                    write!(
                        self.output_file,
                        "expr{} -> expr{} [label=\"{}\"]\n",
                        arg.id,
                        expr_id.id,
                        format!("arg{}", index)
                    )
                    .expect("Write failed");
                }
                let func = self.program.functions.get(id);
                format!("StaticFunctionCall({})", func.info)
            }
            Expr::DynamicFunctionCall(func_expr, args) => {
                for (index, arg) in args.iter().enumerate() {
                    write!(
                        self.output_file,
                        "expr{} -> expr{} [label=\"{}\"]\n",
                        arg.id,
                        expr_id.id,
                        format!("arg{}", index)
                    )
                    .expect("Write failed");
                }
                write!(
                    self.output_file,
                    "expr{} -> expr{} [label=\"{}\"]\n",
                    func_expr.id, expr_id.id, "func_expr"
                )
                .expect("Write failed");
                format!("DynamicFunctionCall")
            }
            Expr::ClassFunctionCall(id, args) => {
                let member = self.program.class_members.get(id);
                for (index, arg) in args.iter().enumerate() {
                    write!(
                        self.output_file,
                        "expr{} -> expr{} [label=\"{}\"]\n",
                        arg.id,
                        expr_id.id,
                        format!("arg{}", index)
                    )
                    .expect("Write failed");
                }
                format!("ClassFunctionCall({})", member.name)
            }
            Expr::ExprValue(_, pattern_id) => {
                write!(
                    self.output_file,
                    "pattern{} -> expr{} [label=\"{}\"]\n",
                    pattern_id.id, expr_id.id, "expr_value"
                )
                .expect("Write failed");
                format!("ExprValue")
            }
            Expr::Do(exprs) => {
                for (index, step) in exprs.iter().enumerate() {
                    write!(
                        self.output_file,
                        "expr{} -> expr{} [label=\"{}\"]\n",
                        expr_id.id,
                        step.id,
                        format!("stmt{}", index)
                    )
                    .expect("Write failed");
                }
                format!("Do")
            }
            Expr::Bind(pattern_id, expr) => {
                write!(
                    self.output_file,
                    "expr{} -> pattern{} [label=\"{}\"]\n",
                    expr.id, pattern_id.id, "bind"
                )
                .expect("Write failed");
                write!(
                    self.output_file,
                    "expr{} -> expr{} [label=\"{}\"]\n",
                    expr_id.id, expr.id, "Bind-Expr"
                )
                .expect("Write failed");
                write!(
                    self.output_file,
                    "expr{} -> pattern{} [label=\"{}\"]\n",
                    expr_id.id, pattern_id.id, "Bind-Pattern"
                )
                .expect("Write failed");
                format!("Bind")
            }
            _ => format!("{}", expr),
        };
        write!(
            self.output_file,
            "expr{} [label=\"{}\"]\n",
            expr_id.id, label
        )
        .expect("Write failed");
    }

    fn visit_pattern(&mut self, pattern_id: PatternId, pattern: &Pattern) {}
}
