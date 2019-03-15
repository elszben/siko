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
    let path = parser.parse_item_path()?;
    let expr = Expr::Path(path);
    let id = parser.add_expr(expr, start_index);
    Ok(id)
}

fn parse_paren_expr(parser: &mut Parser) -> Result<ExprId, Error> {
    let start_index = parser.get_index();
    let res = parse_parens(parser, |p| p.parse_expr(), " expression")?;
    match res {
        ParenParseResult::Single(e) => {
            return Ok(e);
        }
        ParenParseResult::Tuple(exprs) => {
            let expr = Expr::Tuple(exprs);
            let id = parser.add_expr(expr, start_index);
            return Ok(id);
        }
    }
}

fn parse_lambda(parser: &mut Parser) -> Result<ExprId, Error> {
    let start_index = parser.get_index();
    parser.expect(TokenKind::Lambda)?;
    let args = parser.parse_list1(TokenKind::Identifier, TokenKind::Comma)?;
    let args: Vec<_> = to_string_list(args);
    parser.expect(TokenKind::Op(BuiltinOperator::Arrow))?;
    let expr_id = parser.parse_expr()?;
    let lambda_expr = Expr::Lambda(args, expr_id);
    let id = parser.add_expr(lambda_expr, start_index);
    Ok(id)
}

fn parse_do(parser: &mut Parser) -> Result<ExprId, Error> {
    let start_index = parser.get_index();
    parser.expect(TokenKind::KeywordDo)?;
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
        parser.expect(TokenKind::EndOfItem)?;
        if let Some(bind_var) = bind_var {
            let expr = Expr::Bind(bind_var, expr);
            let id = parser.add_expr(expr, bind_start_index);
            exprs.push(id);
        } else {
            exprs.push(expr);
        }
        if parser.current(TokenKind::EndOfBlock) {
            break;
        }
    }
    parser.expect(TokenKind::EndOfBlock)?;
    let expr = Expr::Do(exprs);
    let id = parser.add_expr(expr, start_index);
    Ok(id)
}

fn parse_if(parser: &mut Parser) -> Result<ExprId, Error> {
    let start_index = parser.get_index();
    parser.expect(TokenKind::KeywordIf)?;
    let cond = parser.parse_expr()?;
    parser.expect(TokenKind::KeywordThen)?;
    let true_branch = parser.parse_expr()?;
    parser.expect(TokenKind::KeywordElse)?;
    let false_branch = parser.parse_expr()?;
    let expr = Expr::If(cond, true_branch, false_branch);
    let id = parser.add_expr(expr, start_index);
    Ok(id)
}

fn parse_arg(parser: &mut Parser) -> Result<ExprId, Error> {
    let start_index = parser.get_index();
    let token_info = parser.peek().expect("Ran out of tokens");
    let id = match token_info.token {
        Token::Identifier(..) => {
            return parse_path(parser);
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
                    return Ok(id);
                }
            }
            let n = n.parse().expect("Failed to parse int");
            let expr = Expr::IntegerLiteral(n);
            let id = parser.add_expr(expr, start_index);
            id
        }
        Token::BoolLiteral(b) => {
            parser.advance()?;
            let expr = Expr::BoolLiteral(b);
            let id = parser.add_expr(expr, start_index);
            id
        }
        Token::StringLiteral(s) => {
            parser.advance()?;
            let expr = Expr::StringLiteral(s);
            let id = parser.add_expr(expr, start_index);
            id
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
            return parse_lambda(parser);
        }
        _ => {
            return report_unexpected_token(parser, "Expected expression");
        }
    };
    Ok(id)
}

fn parse_primary(parser: &mut Parser) -> Result<ExprId, Error> {
    let start_index = parser.get_index();
    let f = parse_unary(parser, false)?;
    let mut args = Vec::new();
    loop {
        match parser.current_kind() {
            TokenKind::Op(BuiltinOperator::Not)
            | TokenKind::Identifier
            | TokenKind::NumericLiteral
            | TokenKind::BoolLiteral
            | TokenKind::StringLiteral
            | TokenKind::LParen
            | TokenKind::KeywordIf
            | TokenKind::KeywordDo
            | TokenKind::Lambda => {}
            _ => break,
        }
        let arg = parse_unary(parser, true)?;
        args.push(arg);
    }
    if args.is_empty() {
        Ok(f)
    } else {
        let expr = Expr::FunctionCall(f, args);
        let id = parser.add_expr(expr, start_index);
        Ok(id)
    }
}

fn parse_unary(parser: &mut Parser, is_arg: bool) -> Result<ExprId, Error> {
    let start_index = parser.get_index();
    let ops: &[BuiltinOperator] = if is_arg {
        &[BuiltinOperator::Not]
    } else {
        &[BuiltinOperator::Not, BuiltinOperator::Sub]
    };
    if let Some((op, _)) = parser.consume_op(ops) {
        let function_id_expr = Expr::Builtin(op);
        let function_id_expr_id = parser.add_expr(function_id_expr, start_index);
        let right = parse_unary(parser, is_arg)?;
        let op = if op == BuiltinOperator::Sub {
            BuiltinOperator::Minus
        } else {
            op
        };
        if op == BuiltinOperator::Minus {
            let location_id = parser.get_program().get_expr_location(&right);
            let right_expr = parser.get_program().get_expr(&right);
            // FIXME: fix location of these literals
            if let Expr::IntegerLiteral(n) = right_expr {
                let expr = Expr::IntegerLiteral(-n);
                parser.get_program().add_expr(right, expr, location_id);
                return Ok(right);
            }
            if let Expr::FloatLiteral(n) = right_expr {
                let expr = Expr::FloatLiteral(-n);
                parser.get_program().add_expr(right, expr, location_id);
                return Ok(right);
            }
        }
        let expr = Expr::FunctionCall(function_id_expr_id, vec![right]);
        let id = parser.add_expr(expr, start_index);
        Ok(id)
    } else {
        return parse_arg(parser);
    }
}

fn parse_binary_op(
    parser: &mut Parser,
    ops: &[BuiltinOperator],
    next: fn(&mut Parser) -> Result<ExprId, Error>,
) -> Result<ExprId, Error> {
    let start_index = parser.get_index();
    let mut left = next(parser)?;
    loop {
        if let Some((op, _)) = parser.consume_op(ops) {
            let function_id_expr = Expr::Builtin(op);
            let function_id_expr_id = parser.add_expr(function_id_expr, start_index);
            let right = next(parser)?;
            let expr = Expr::FunctionCall(function_id_expr_id, vec![left, right]);
            let id = parser.add_expr(expr, start_index);
            left = id;
            continue;
        } else {
            break;
        }
    }
    Ok(left)
}

pub fn parse_ops(parser: &mut Parser) -> Result<ExprId, Error> {
    return parse_andor(parser);
}

fn parse_andor(parser: &mut Parser) -> Result<ExprId, Error> {
    return parse_binary_op(
        parser,
        &[BuiltinOperator::And, BuiltinOperator::Or],
        parse_equal,
    );
}

fn parse_equal(parser: &mut Parser) -> Result<ExprId, Error> {
    return parse_binary_op(
        parser,
        &[BuiltinOperator::Equals, BuiltinOperator::NotEquals],
        parse_ord_ops,
    );
}

fn parse_ord_ops(parser: &mut Parser) -> Result<ExprId, Error> {
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

fn parse_addsub(parser: &mut Parser) -> Result<ExprId, Error> {
    return parse_binary_op(
        parser,
        &[BuiltinOperator::Add, BuiltinOperator::Sub],
        parse_muldiv,
    );
}

fn parse_muldiv(parser: &mut Parser) -> Result<ExprId, Error> {
    return parse_binary_op(
        parser,
        &[BuiltinOperator::Mul, BuiltinOperator::Div],
        parse_pipe_forward,
    );
}

fn parse_pipe_forward(parser: &mut Parser) -> Result<ExprId, Error> {
    return parse_binary_op(parser, &[BuiltinOperator::PipeForward], parse_primary);
}
