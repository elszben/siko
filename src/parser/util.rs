use crate::error::Error;
use crate::parser::parser::Parser;
use crate::token::TokenInfo;
use crate::token::TokenKind;

pub fn report_unexpected_token<T>(parser: &mut Parser, msg: &str) -> Result<T, Error> {
    if parser.is_done() {
        let last = parser.get_last();
        return Err(Error::parse_err(
            format!("{}", msg),
            parser.get_file_path(),
            last.location,
        ));
    } else {
        let found = parser.advance()?;
        return Err(Error::parse_err(
            format!("{}, found {:?}", msg, found.token.kind()),
            parser.get_file_path(),
            found.location,
        ));
    }
}

pub fn to_string_list(tokens: Vec<TokenInfo>) -> Vec<String> {
    tokens.into_iter().map(|t| t.token.get_ident()).collect()
}

pub enum ParenParseResult<T> {
    Single(T),
    Tuple(Vec<T>),
}

pub fn parse_parens<T>(
    parser: &mut Parser,
    inner_parser: fn(&mut Parser) -> Result<T, Error>,
    item_name: &str,
) -> Result<ParenParseResult<T>, Error> {
    parser.expect(TokenKind::LParen)?;
    let mut parts = Vec::new();
    let mut comma_found = false;
    loop {
        if parser.current(TokenKind::RParen) {
            break;
        }
        let part = inner_parser(parser)?;
        parts.push(part);

        if parser.current(TokenKind::Comma) {
            parser.advance()?;
            comma_found = true;
        } else if parser.current(TokenKind::RParen) {
            break;
        } else {
            return report_unexpected_token(parser, &format!("Expected , or {}", item_name));
        }
    }
    parser.expect(TokenKind::RParen)?;
    if comma_found || parts.is_empty() {
        Ok(ParenParseResult::Tuple(parts))
    } else {
        Ok(ParenParseResult::Single(parts.pop().unwrap()))
    }
}
