use crate::error::Error;
use crate::parser::parser::Parser;
use crate::parser::token::TokenKind;

pub enum ParserErrorReason {
    UnexpectedToken { expected: String },
    Custom { msg: String },
}

pub fn report_unexpected_token<T>(parser: &mut Parser, expected: String) -> Result<T, Error> {
    report_parser_error(
        parser,
        ParserErrorReason::UnexpectedToken { expected: expected },
    )
}

pub fn report_parser_error<T>(parser: &mut Parser, reason: ParserErrorReason) -> Result<T, Error> {
    if parser.is_done() {
        let last = parser.get_last();
        match reason {
            ParserErrorReason::UnexpectedToken { expected } => {
                return Err(Error::parse_err(
                    format!("expected {}", expected),
                    parser.get_file_path(),
                    last.location,
                ));
            }
            ParserErrorReason::Custom { msg } => {
                return Err(Error::parse_err(
                    format!("{}", msg),
                    parser.get_file_path(),
                    last.location,
                ));
            }
        }
    } else {
        let found = parser.peek().expect("empty");
        match reason {
            ParserErrorReason::UnexpectedToken { expected } => {
                if found.token.kind() == TokenKind::EndOfItem
                    || found.token.kind() == TokenKind::EndOfBlock
                    || found.token.kind() == TokenKind::EndOfModule
                {
                    return Err(Error::parse_err(
                        format!("expected {}", expected),
                        parser.get_file_path(),
                        found.location,
                    ));
                } else {
                    return Err(Error::parse_err(
                        format!(
                            "expected {}, found {}",
                            expected,
                            found.token.kind().nice_name()
                        ),
                        parser.get_file_path(),
                        found.location,
                    ));
                }
            }
            ParserErrorReason::Custom { msg } => {
                return Err(Error::parse_err(
                    format!("{}", msg),
                    parser.get_file_path(),
                    found.location,
                ));
            }
        }
    }
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
            return report_unexpected_token(parser, format!(", or {}", item_name));
        }
    }
    parser.expect(TokenKind::RParen)?;
    if comma_found || parts.is_empty() {
        Ok(ParenParseResult::Tuple(parts))
    } else {
        Ok(ParenParseResult::Single(parts.pop().unwrap()))
    }
}
