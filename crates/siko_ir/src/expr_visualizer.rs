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
use std::ops::Index;
use std::path::Path;

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
        write!(self.output_file, "node [shape=circle fontname=Arial];\n")?;
        Ok(())
    }

    fn footer(&mut self) -> IoResult<()> {
        write!(self.output_file, "}}\n")?;
        Ok(())
    }

    /*
     #[allow(unused)]
    pub fn write_digraph(&self, name: &Path, expr_id: ExprId) -> IoResult<()> {
        let mut output = File::create(name)?;
        write!(output, "digraph D {{\n")?;
        write!(output, "node [shape=circle fontname=Arial];\n")?;
        for (index, item) in self.items.iter().enumerate() {
            let label = item.to_string();
            write!(output, "node{} [label=\"{}\"]\n", index, label)?;
            codes.insert(item.clone(), index);
        }
        let mut attr_index = 0;
        for (item, attributes) in &self.attributes {
            for attribute in attributes {
                write!(
                    output,
                    "attr_node{} [label=\"{}\", shape=ellipse, style=filled, fillcolor=red]\n",
                    attr_index, attribute
                )?;
                let code = codes.get(item).expect("item not found");
                write!(output, "node{} -> attr_node{}\n", code, attr_index)?;
                attr_index += 1;
            }
        }
        for relation in &self.relations {
            if relation.item_refs.len() == 2 {
                let first_code = codes.get(&relation.item_refs[0]).expect("first not found");
                let second_code = codes.get(&relation.item_refs[1]).expect("second not found");
                write!(
                    output,
                    "node{} -> node{} [label=\"{}\"]\n",
                    first_code, second_code, relation.name
                )?;
            }
        }
        write!(output, "}}\n")?;
        Ok(())
    }
    */
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
                        "node{} -> node{} [label=\"{}\"]\n",
                        expr_id.id,
                        arg.id,
                        format!("arg{}", index)
                    )
                    .expect("Write failed");
                }
                let func = self.program.functions.get(id);
                format!("StaticFunctionCall({})", func.info)
            }
            Expr::DynamicFunctionCall(id, args) => {
                for (index, arg) in args.iter().enumerate() {
                    write!(
                        self.output_file,
                        "node{} -> node{} [label=\"{}\"]\n",
                        expr_id.id,
                        arg.id,
                        format!("arg{}", index)
                    )
                    .expect("Write failed");
                }
                format!("DyncamiFunctionCall({})", id.id)
            }
            Expr::ClassFunctionCall(id, args) => {
                for (index, arg) in args.iter().enumerate() {
                    write!(
                        self.output_file,
                        "node{} -> node{} [label=\"{}\"]\n",
                        expr_id.id,
                        arg.id,
                        format!("arg{}", index)
                    )
                    .expect("Write failed");
                }
                format!("ClassFunctionCall({})", id.id)
            }
            Expr::ExprValue(id, pattern_id) => {
                write!(
                    self.output_file,
                    "node{} -> node{} [label=\"{}\"]\n",
                    id.id, expr_id.id, "expr_value"
                )
                .expect("Write failed");
                format!("expr_value")
            }
            _ => format!("{}", expr),
        };
        write!(
            self.output_file,
            "node{} [label=\"{}\"]\n",
            expr_id.id, label
        )
        .expect("Write failed");
    }
    fn visit_pattern(&mut self, pattern_id: PatternId, pattern: &Pattern) {}
}
