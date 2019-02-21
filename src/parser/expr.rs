use super::util::parse_parens;
use super::util::report_unexpected_token;
use super::util::to_string_list;
use super::util::ParenParseResult;
use crate::constants::BuiltinOperator;
use crate::error::Error;

use crate::parser::parser::Parser;
use crate::syntax::expr::Expr;
use crate::syntax::expr::ExprId;
use crate::token::Token;
use crate::token::TokenKind;

fn parse_path(parser: &mut Parser) -> Result<ExprId, Error> {
    let start_index = parser.get_index();
    let path = parser.parse_item_path("Expected identifer")?;
    let expr = Expr::Path(path);
    let id = parser.add_expr(expr, start_index);
    Ok(id)
}

fn parse_paren_expr(parser: &mut Parser) -> Result<Option<ExprId>, Error> {
    let start_index = parser.get_index();
    if let Some(res) = parse_parens(parser, |p| p.parse_expr())? {
        match res {
            ParenParseResult::Single(e) => {
                return Ok(Some(e));
            }
            ParenParseResult::Tuple(exprs) => {
                let expr = Expr::Tuple(exprs);
                let id = parser.add_expr(expr, start_index);
                return Ok(Some(id));
            }
        }
    } else {
        Ok(None)
    }
}

fn parse_lambda(parser: &mut Parser) -> Result<ExprId, Error> {
    let start_index = parser.get_index();
    let lambda_token = parser.expect(TokenKind::Lambda)?;
    if let Some(args) = parser.parse_list1(TokenKind::Identifier, TokenKind::Comma)? {
        let args: Vec<_> = to_string_list(args);
        let arrow_token = parser.expect(TokenKind::Op(BuiltinOperator::Arrow))?;
        let expr = parser.parse_expr()?;
        let expr_id = match expr {
            Some(expr_id) => expr_id,
            None => {
                return report_unexpected_token(
                    arrow_token,
                    parser,
                    "Expected expression as lambda body",
                );
            }
        };
        let lambda_expr = Expr::Lambda(args, expr_id);
        let id = parser.add_expr(lambda_expr, start_index);
        Ok(id)
    } else {
        return report_unexpected_token(lambda_token, parser, "Expected lambda argument");
    }
}

fn parse_do(parser: &mut Parser) -> Result<Option<ExprId>, Error> {
    let start_index = parser.get_index();
    let do_token = parser.expect(TokenKind::KeywordDo)?;
    let mut exprs = Vec::new();
    loop {
        let mut bind_var = None;
        let bind_start_index = parser.get_index();
        if let Some(first) = parser.peek() {
            if let Some(second) = parser.lookahead(1) {
                match first.token {
                    Token::Identifier(i) => {
                        if second.token.kind() == TokenKind::Op(BuiltinOperator::Bind) {
                            bind_var = Some(i);
                            parser.advance()?;
                            parser.advance()?;
                        }
                    }
                    _ => {}
                }
            }
        }
        let expr = parser.parse_expr()?;
        let expr: ExprId = match expr {
            Some(expr) => {
                parser.expect(TokenKind::EndOfItem)?;
                expr
            }
            None => {
                if exprs.is_empty() {
                    return report_unexpected_token(
                        do_token,
                        parser,
                        "Expected expression as do body",
                    );
                } else {
                    break;
                }
            }
        };
        if let Some(bind_var) = bind_var {
            let expr = Expr::Bind(bind_var, expr);
            let id = parser.add_expr(expr, bind_start_index);
            exprs.push(id);
        } else {
            exprs.push(expr);
        }
    }
    parser.expect(TokenKind::EndOfBlock)?;
    let expr = Expr::Do(exprs);
    let id = parser.add_expr(expr, start_index);
    Ok(Some(id))
}

fn parse_if(parser: &mut Parser) -> Result<Option<ExprId>, Error> {
    let start_index = parser.get_index();
    let if_token = parser.expect(TokenKind::KeywordIf)?;
    let cond = parser.parse_expr()?;
    let cond = match cond {
        Some(cond) => cond,
        None => {
            return report_unexpected_token(if_token, parser, "Expected expression as if condition");
        }
    };
    let then_token = parser.expect(TokenKind::KeywordThen)?;
    let true_branch = parser.parse_expr()?;
    let true_branch = match true_branch {
        Some(true_branch) => true_branch,
        None => {
            return report_unexpected_token(
                then_token,
                parser,
                "Expected expression as if true branch",
            );
        }
    };
    let else_token = parser.expect(TokenKind::KeywordElse)?;
    let false_branch = parser.parse_expr()?;
    let false_branch = match false_branch {
        Some(false_branch) => false_branch,
        None => {
            return report_unexpected_token(
                else_token,
                parser,
                "Expected expression as if false branch",
            );
        }
    };
    let expr = Expr::If(cond, true_branch, false_branch);
    let id = parser.add_expr(expr, start_index);
    Ok(Some(id))
}

fn parse_arg(parser: &mut Parser) -> Result<Option<ExprId>, Error> {
    let start_index = parser.get_index();
    if let Some(token_info) = parser.peek() {
        let id = match token_info.token {
            Token::Identifier(..) => {
                let id = parse_path(parser)?;
                return Ok(Some(id));
            }
            Token::NumericLiteral(n) => {
                parser.advance()?;
                if let Some(token_info) = parser.peek() {
                    if let TokenKind::Dot = token_info.token.kind() {
                        parser.advance()?;
                        let mut float = format!("{}.", n);
                        if let Some(token_info) = parser.peek() {
                            if let Token::NumericLiteral(n2) = token_info.token {
                                parser.advance()?;
                                float = format!("{}{}", float, n2);
                            }
                        }
                        let f = float.parse().expect("Failed to parse float");
                        let expr = Expr::FloatLiteral(f);
                        let id = parser.add_expr(expr, start_index);
                        return Ok(Some(id));
                    }
                }
                let n = n.parse().expect("Failed to parse int");
                let expr = Expr::IntegerLiteral(n);
                let id = parser.add_expr(expr, start_index);
                Some(id)
            }
            Token::BoolLiteral(b) => {
                parser.advance()?;
                let expr = Expr::BoolLiteral(b);
                let id = parser.add_expr(expr, start_index);
                Some(id)
            }
            Token::StringLiteral(s) => {
                parser.advance()?;
                let expr = Expr::StringLiteral(s);
                let id = parser.add_expr(expr, start_index);
                Some(id)
            }
            Token::LParen => {
                return parse_paren_expr(parser);
            }
            Token::KeywordIf => {
                return parse_if(parser);
            }
            Token::KeywordDo => {
                return parse_do(parser);
            }
            Token::Lambda => {
                let id = parse_lambda(parser)?;
                return Ok(Some(id));
            }
            _ => None,
        };
        Ok(id)
    } else {
        Ok(None)
    }
}

fn parse_primary(parser: &mut Parser) -> Result<Option<ExprId>, Error> {
    let start_index = parser.get_index();
    if let Some(f) = parse_unary(parser, false)? {
        let mut args = Vec::new();
        while !parser.is_done() {
            if let Some(arg) = parse_unary(parser, true)? {
                args.push(arg);
            } else {
                break;
            }
        }
        if args.is_empty() {
            Ok(Some(f))
        } else {
            let expr = Expr::FunctionCall(f, args);
            let id = parser.add_expr(expr, start_index);
            Ok(Some(id))
        }
    } else {
        Ok(None)
    }
}

fn parse_unary(parser: &mut Parser, is_arg: bool) -> Result<Option<ExprId>, Error> {
    let start_index = parser.get_index();
    let ops: &[BuiltinOperator] = if is_arg {
        &[BuiltinOperator::Not]
    } else {
        &[BuiltinOperator::Not, BuiltinOperator::Sub]
    };
    if let Some((op, op_token)) = parser.consume_op(ops) {
        let function_id_expr = Expr::Builtin(op);
        let function_id_expr_id = parser.add_expr(function_id_expr, start_index);
        let right = parse_unary(parser, is_arg)?;
        let right = match right {
            Some(right) => right,
            None => {
                let found = parser.current_kind();
                return Err(Error::parse_err(
                    format!(
                        "Expected expression at right side of {:?}, found {:?}",
                        op_token.token.kind(),
                        found
                    ),
                    parser.get_file_path(),
                    op_token.location,
                ));
            }
        };
        let op = if op == BuiltinOperator::Sub {
            BuiltinOperator::Minus
        } else {
            op
        };
        if op == BuiltinOperator::Minus {
            let right_expr = parser.get_program().get_expr(&right);
            if let Expr::IntegerLiteral(n) = right_expr {
                let expr = Expr::IntegerLiteral(-n);
                parser.get_program().add_expr(right, expr);
                return Ok(Some(right));
            }
            if let Expr::FloatLiteral(n) = right_expr {
                let expr = Expr::FloatLiteral(-n);
                parser.get_program().add_expr(right, expr);
                return Ok(Some(right));
            }
        }
        let expr = Expr::FunctionCall(function_id_expr_id, vec![right]);
        let id = parser.add_expr(expr, start_index);
        Ok(Some(id))
    } else {
        return parse_arg(parser);
    }
}

fn parse_binary_op(
    parser: &mut Parser,
    ops: &[BuiltinOperator],
    next: fn(&mut Parser) -> Result<Option<ExprId>, Error>,
) -> Result<Option<ExprId>, Error> {
    let start_index = parser.get_index();
    let left = next(parser)?;
    let mut left = match left {
        Some(left) => left,
        None => return Ok(None),
    };
    loop {
        if let Some((op, op_token)) = parser.consume_op(ops) {
            let function_id_expr = Expr::Builtin(op);
            let function_id_expr_id = parser.add_expr(function_id_expr, start_index);
            let right = next(parser)?;
            let right = match right {
                Some(right) => right,
                None => {
                    let found = parser.current_kind();
                    return Err(Error::parse_err(
                        format!(
                            "Expected expression at right side of {:?}, found {:?}",
                            op_token.token.kind(),
                            found
                        ),
                        parser.get_file_path(),
                        op_token.location,
                    ));
                }
            };
            let expr = Expr::FunctionCall(function_id_expr_id, vec![left, right]);
            let id = parser.add_expr(expr, start_index);
            left = id;
            continue;
        } else {
            break;
        }
    }
    Ok(Some(left))
}

pub fn parse_ops(parser: &mut Parser) -> Result<Option<ExprId>, Error> {
    return parse_andor(parser);
}

fn parse_andor(parser: &mut Parser) -> Result<Option<ExprId>, Error> {
    return parse_binary_op(
        parser,
        &[BuiltinOperator::And, BuiltinOperator::Or],
        parse_equal,
    );
}

fn parse_equal(parser: &mut Parser) -> Result<Option<ExprId>, Error> {
    return parse_binary_op(
        parser,
        &[BuiltinOperator::Equals, BuiltinOperator::NotEquals],
        parse_ord_ops,
    );
}

fn parse_ord_ops(parser: &mut Parser) -> Result<Option<ExprId>, Error> {
    return parse_binary_op(
        parser,
        &[
            BuiltinOperator::LessThan,
            BuiltinOperator::LessOrEqualThan,
            BuiltinOperator::GreaterThan,
            BuiltinOperator::GreaterOrEqualThan,
        ],
        parse_addsub,
    );
}

fn parse_addsub(parser: &mut Parser) -> Result<Option<ExprId>, Error> {
    return parse_binary_op(
        parser,
        &[BuiltinOperator::Add, BuiltinOperator::Sub],
        parse_muldiv,
    );
}

fn parse_muldiv(parser: &mut Parser) -> Result<Option<ExprId>, Error> {
    return parse_binary_op(
        parser,
        &[BuiltinOperator::Mul, BuiltinOperator::Div],
        parse_pipe_forward,
    );
}

fn parse_pipe_forward(parser: &mut Parser) -> Result<Option<ExprId>, Error> {
    return parse_binary_op(parser, &[BuiltinOperator::PipeForward], parse_primary);
}
