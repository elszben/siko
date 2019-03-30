use crate::constants::BuiltinOperator;
use crate::location_info::location::Location;

#[derive(Debug, Clone)]
pub enum Token {
    Identifier(String),
    StringLiteral(String),
    IntegerLiteral(i64),
    FloatLiteral(f64),
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
    LCurly,
    RCurly,
    LParen,
    RParen,
    Semicolon,
    Pipe,
    Lambda,
    DoubleDot,
    EndOfItem,
    EndOfBlock,
    EndOfModule,
}

impl Token {
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
            Token::IntegerLiteral(..) => TokenKind::IntegerLiteral,
            Token::FloatLiteral(..) => TokenKind::FloatLiteral,
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
            Token::LCurly => TokenKind::LCurly,
            Token::RCurly => TokenKind::RCurly,
            Token::LParen => TokenKind::LParen,
            Token::RParen => TokenKind::RParen,
            Token::Semicolon => TokenKind::Semicolon,
            Token::Pipe => TokenKind::Pipe,
            Token::Lambda => TokenKind::Lambda,
            Token::DoubleDot => TokenKind::DoubleDot,
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
    IntegerLiteral,
    FloatLiteral,
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
    LCurly,
    RCurly,
    LParen,
    RParen,
    Semicolon,
    Pipe,
    Lambda,
    DoubleDot,
    EndOfItem,
    EndOfBlock,
    EndOfModule,
}

impl TokenKind {
    pub fn nice_name(&self) -> String {
        let name = format!("{:?}", self);
        let name = name.to_lowercase();
        let name = name.replace("keyword", "keyword ");
        name
    }
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub token: Token,
    pub location: Location,
}
