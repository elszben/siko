use crate::constants::BuiltinOperator;
use crate::location_info::location::Location;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Token {
    Identifier(String),
    StringLiteral(String),
    NumericLiteral(String),
    BoolLiteral(bool),
    KeywordWhere,
    KeywordData,
    KeywordModule,
    KeywordImport,
    KeywordIf,
    KeywordThen,
    KeywordElse,
    KeywordDo,
    KeywordAs,
    KeywordExtern,
    KeywordDoubleColon,
    KeywordHiding,
    Op(BuiltinOperator),
    Equal,
    Comma,
    Dot,
    LCurly,
    RCurly,
    LParen,
    RParen,
    Semicolon,
    Pipe,
    Lambda,
    EndOfItem,
    EndOfBlock,
    EndOfModule,
}

impl Token {
    pub fn get_ident(&self) -> String {
        if let Token::Identifier(i) = self {
            i.clone()
        } else {
            unreachable!()
        }
    }

    pub fn get_op(&self) -> Option<BuiltinOperator> {
        if let Token::Op(o) = self {
            Some(o.clone())
        } else {
            None
        }
    }

    pub fn kind(&self) -> TokenKind {
        match self {
            Token::Identifier(..) => TokenKind::Identifier,
            Token::StringLiteral(..) => TokenKind::StringLiteral,
            Token::NumericLiteral(..) => TokenKind::NumericLiteral,
            Token::BoolLiteral(..) => TokenKind::BoolLiteral,
            Token::KeywordWhere => TokenKind::KeywordWhere,
            Token::KeywordData => TokenKind::KeywordData,
            Token::KeywordModule => TokenKind::KeywordModule,
            Token::KeywordImport => TokenKind::KeywordImport,
            Token::KeywordIf => TokenKind::KeywordIf,
            Token::KeywordThen => TokenKind::KeywordThen,
            Token::KeywordElse => TokenKind::KeywordElse,
            Token::KeywordDo => TokenKind::KeywordDo,
            Token::KeywordAs => TokenKind::KeywordAs,
            Token::KeywordExtern => TokenKind::KeywordExtern,
            Token::KeywordDoubleColon => TokenKind::KeywordDoubleColon,
            Token::KeywordHiding => TokenKind::KeywordHiding,
            Token::Op(op) => TokenKind::Op(*op),
            Token::Equal => TokenKind::Equal,
            Token::Comma => TokenKind::Comma,
            Token::Dot => TokenKind::Dot,
            Token::LCurly => TokenKind::LCurly,
            Token::RCurly => TokenKind::RCurly,
            Token::LParen => TokenKind::LParen,
            Token::RParen => TokenKind::RParen,
            Token::Semicolon => TokenKind::Semicolon,
            Token::Pipe => TokenKind::Pipe,
            Token::Lambda => TokenKind::Lambda,
            Token::EndOfItem => TokenKind::EndOfItem,
            Token::EndOfBlock => TokenKind::EndOfBlock,
            Token::EndOfModule => TokenKind::EndOfModule,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TokenKind {
    Identifier,
    StringLiteral,
    NumericLiteral,
    BoolLiteral,
    KeywordWhere,
    KeywordData,
    KeywordModule,
    KeywordImport,
    KeywordIf,
    KeywordThen,
    KeywordElse,
    KeywordDo,
    KeywordAs,
    KeywordExtern,
    KeywordDoubleColon,
    KeywordHiding,
    Op(BuiltinOperator),
    Equal,
    Comma,
    Dot,
    LCurly,
    RCurly,
    LParen,
    RParen,
    Semicolon,
    Pipe,
    Lambda,
    EndOfItem,
    EndOfBlock,
    EndOfModule,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TokenInfo {
    pub token: Token,
    pub location: Location,
}
