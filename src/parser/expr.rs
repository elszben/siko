use super::util::parse_parens;
use super::util::report_unexpected_token;
use super::util::ParenParseResult;
use crate::constants::BuiltinOperator;
use crate::error::Error;

use crate::parser::parser::Parser;
use crate::parser::token::Token;
use crate::parser::token::TokenKind;
use crate::syntax::expr::Case;
use crate::syntax::expr::Expr;
use crate::syntax::expr::ExprId;
use crate::syntax::pattern::Pattern;
use crate::syntax::pattern::PatternId;

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
    let args = parser.parse_lambda_args()?;
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
        let bind_start_index = parser.get_index();
        let mut pattern_id = None;
        if parser.irrefutable_pattern_follows() {
            pattern_id = Some(parse_pattern(parser)?);
            parser.expect(TokenKind::Op(BuiltinOperator::Bind))?;
        }
        let expr = parser.parse_expr()?;
        parser.expect(TokenKind::EndOfItem)?;
        if let Some(pattern_id) = pattern_id {
            let expr = Expr::Bind(pattern_id, expr);
            let id = parser.add_expr(expr, bind_start_index);
            exprs.push(id);
        } else {
            exprs.push(expr);
        }
        if parser.current(TokenKind::EndOfBlock) {
            break;
        }
    }
    let expr = Expr::Do(exprs);
    let id = parser.add_expr(expr, start_index);
    parser.expect(TokenKind::EndOfBlock)?;
    Ok(id)
}

fn parse_tuple_pattern(parser: &mut Parser) -> Result<PatternId, Error> {
    let start_index = parser.get_index();
    let res = parse_parens(parser, |p| parse_pattern(p), "<pattern>")?;
    match res {
        ParenParseResult::Single(t) => {
            return Ok(t);
        }
        ParenParseResult::Tuple(ts) => {
            let pattern = Pattern::Tuple(ts);
            let id = parser.add_pattern(pattern, start_index);
            return Ok(id);
        }
    }
}

fn parse_sub_pattern(parser: &mut Parser, inner: bool) -> Result<Option<PatternId>, Error> {
    let id = match parser.current_kind() {
        TokenKind::LParen => {
            let id = parse_tuple_pattern(parser)?;
            id
        }
        TokenKind::IntegerLiteral => {
            let start_index = parser.get_index();
            let literal = parser.advance()?;
            if let Token::IntegerLiteral(i) = literal.token {
                let pattern = Pattern::IntegerLiteral(i);
                let id = parser.add_pattern(pattern, start_index);
                id
            } else {
                unreachable!()
            }
        }
        TokenKind::FloatLiteral => {
            let start_index = parser.get_index();
            let literal = parser.advance()?;
            if let Token::FloatLiteral(f) = literal.token {
                let pattern = Pattern::FloatLiteral(f);
                let id = parser.add_pattern(pattern, start_index);
                id
            } else {
                unreachable!()
            }
        }
        TokenKind::StringLiteral => {
            let start_index = parser.get_index();
            let literal = parser.advance()?;
            if let Token::StringLiteral(s) = literal.token {
                let pattern = Pattern::StringLiteral(s);
                let id = parser.add_pattern(pattern, start_index);
                id
            } else {
                unreachable!()
            }
        }
        TokenKind::BoolLiteral => {
            let start_index = parser.get_index();
            let literal = parser.advance()?;
            if let Token::BoolLiteral(b) = literal.token {
                let pattern = Pattern::BoolLiteral(b);
                let id = parser.add_pattern(pattern, start_index);
                id
            } else {
                unreachable!()
            }
        }
        TokenKind::VarIdentifier => {
            let start_index = parser.get_index();
            let name = parser.var_identifier("pattern binding")?;
            let pattern = Pattern::Binding(name);
            let id = parser.add_pattern(pattern, start_index);
            id
        }
        TokenKind::TypeIdentifier => {
            if inner {
                let start_index = parser.get_index();
                let name = parser.parse_qualified_type_name()?;
                let id = parser.add_pattern(Pattern::Constructor(name, Vec::new()), start_index);
                id
            } else {
                let start_index = parser.get_index();
                let name = parser.parse_qualified_type_name()?;
                let mut args = Vec::new();
                loop {
                    let arg = match parse_sub_pattern(parser, true)? {
                        Some(arg) => arg,
                        None => {
                            break;
                        }
                    };
                    args.push(arg);
                }
                let pattern = Pattern::Constructor(name, args);
                let id = parser.add_pattern(pattern, start_index);
                id
            }
        }
        TokenKind::Wildcard => {
            let start_index = parser.get_index();
            parser.expect(TokenKind::Wildcard)?;
            let pattern = Pattern::Wildcard;
            let id = parser.add_pattern(pattern, start_index);
            id
        }
        _ => {
            return Ok(None);
        }
    };
    Ok(Some(id))
}

fn parse_pattern(parser: &mut Parser) -> Result<PatternId, Error> {
    let id = parse_sub_pattern(parser, false)?;
    match id {
        Some(id) => Ok(id),
        None => report_unexpected_token(parser, format!("<pattern>")),
    }
}

fn parse_case(parser: &mut Parser) -> Result<ExprId, Error> {
    let start_index = parser.get_index();
    parser.expect(TokenKind::KeywordCase)?;
    let body = parser.parse_expr()?;
    parser.expect(TokenKind::KeywordOf)?;
    let mut cases = Vec::new();
    loop {
        let start_index = parser.get_index();
        let mut pattern_id = parse_pattern(parser)?;
        if parser.current(TokenKind::Pipe) {
            parser.expect(TokenKind::Pipe)?;
            let guard_expr = parser.parse_expr()?;
            let pattern = Pattern::Guarded(pattern_id, guard_expr);
            pattern_id = parser.add_pattern(pattern, start_index);
        }
        parser.expect(TokenKind::Op(BuiltinOperator::Arrow))?;
        let case_body = parser.parse_expr()?;
        parser.expect(TokenKind::EndOfItem)?;
        let case = Case {
            pattern_id: pattern_id,
            body: case_body,
        };
        cases.push(case);
        if parser.current(TokenKind::EndOfBlock) {
            break;
        }
    }
    let expr = Expr::CaseOf(body, cases);
    let id = parser.add_expr(expr, start_index);
    parser.expect(TokenKind::EndOfBlock)?;
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
        Token::TypeIdentifier(..) => {
            let path = parser.parse_qualified_name()?;
            let expr = Expr::Path(path);
            let id = parser.add_expr(expr, start_index);
            id
        }
        Token::VarIdentifier(..) => {
            let path = parser.var_identifier("identifier")?;
            let expr = Expr::Path(path);
            let id = parser.add_expr(expr, start_index);
            id
        }
        Token::IntegerLiteral(n) => {
            parser.advance()?;
            let expr = Expr::IntegerLiteral(n);
            let id = parser.add_expr(expr, start_index);
            id
        }
        Token::FloatLiteral(f) => {
            parser.advance()?;
            let expr = Expr::FloatLiteral(f);
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
            if parser.current(TokenKind::Formatter) {
                parser.expect(TokenKind::Formatter)?;
                let items = if parser.current(TokenKind::LParen) {
                    parser.parse_list1_in_parens(|p| parse_ops(p))?
                } else {
                    let item = parser.parse_expr()?;
                    vec![item]
                };
                let expr = Expr::Formatter(s, items);
                let id = parser.add_expr(expr, start_index);
                id
            } else {
                let expr = Expr::StringLiteral(s);
                let id = parser.add_expr(expr, start_index);
                id
            }
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
        Token::KeywordCase => {
            return parse_case(parser);
        }
        _ => {
            return report_unexpected_token(parser, format!("expression"));
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
            | TokenKind::VarIdentifier
            | TokenKind::IntegerLiteral
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
    return parse_binary_op(parser, &[BuiltinOperator::PipeForward], parse_composition);
}

fn parse_composition(parser: &mut Parser) -> Result<ExprId, Error> {
    let start_index = parser.get_index();
    let mut left = parse_primary(parser)?;
    loop {
        if let Some(dot_token) = parser.peek() {
            if dot_token.token.kind() == TokenKind::Dot {
                parser.expect(TokenKind::Dot)?;
                if let Some(next) = parser.peek() {
                    match next.token {
                        Token::VarIdentifier(_) => {
                            let field_name = parser.var_identifier("field name")?;
                            let expr = match field_name.parse::<usize>() {
                                Ok(n) => Expr::TupleFieldAccess(n, left),
                                Err(_) => Expr::FieldAccess(field_name, left),
                            };
                            let id = parser.add_expr(expr, start_index);
                            left = id;
                            continue;
                        }
                        Token::IntegerLiteral(id) => {
                            parser.advance()?;
                            let expr = Expr::TupleFieldAccess(id as usize, left);
                            let id = parser.add_expr(expr, start_index);
                            left = id;
                            continue;
                        }
                        _ => {
                            let function_id_expr = Expr::Builtin(BuiltinOperator::Composition);
                            let function_id_expr_id =
                                parser.add_expr(function_id_expr, start_index);
                            let right = parse_primary(parser)?;
                            let expr = Expr::FunctionCall(function_id_expr_id, vec![left, right]);
                            let id = parser.add_expr(expr, start_index);
                            left = id;
                            continue;
                        }
                    }
                } else {
                    return report_unexpected_token(parser, format!("expression"));
                }
            } else {
                break;
            }
        }
    }
    Ok(left)
}
