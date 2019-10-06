use siko_constants::BuiltinOperator;
use siko_location_info::location::Location;

#[derive(Debug, Clone)]
pub enum Token {
    VarIdentifier(String),
    TypeIdentifier(String),
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
    KeywordCase,
    KeywordOf,
    KeywordClass,
    KeywordConstraint,
    KeywordInstance,
    KeywordProtocol,
    KeywordActor,
    Op(BuiltinOperator),
    Equal,
    Comma,
    LCurly,
    RCurly,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Semicolon,
    Pipe,
    Lambda,
    Dot,
    DoubleDot,
    Formatter,
    Wildcard,
    Colon,
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
            Token::VarIdentifier(..) => TokenKind::VarIdentifier,
            Token::TypeIdentifier(..) => TokenKind::TypeIdentifier,
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
            Token::KeywordCase => TokenKind::KeywordCase,
            Token::KeywordOf => TokenKind::KeywordOf,
            Token::KeywordClass => TokenKind::KeywordClass,
            Token::KeywordConstraint => TokenKind::KeywordConstraint,
            Token::KeywordInstance => TokenKind::KeywordInstance,
            Token::KeywordProtocol => TokenKind::KeywordProtocol,
            Token::KeywordActor => TokenKind::KeywordActor,
            Token::Op(op) => TokenKind::Op(*op),
            Token::Equal => TokenKind::Equal,
            Token::Comma => TokenKind::Comma,
            Token::LCurly => TokenKind::LCurly,
            Token::RCurly => TokenKind::RCurly,
            Token::LParen => TokenKind::LParen,
            Token::RParen => TokenKind::RParen,
            Token::LBracket => TokenKind::LBracket,
            Token::RBracket => TokenKind::RBracket,
            Token::Semicolon => TokenKind::Semicolon,
            Token::Pipe => TokenKind::Pipe,
            Token::Lambda => TokenKind::Lambda,
            Token::Dot => TokenKind::Dot,
            Token::DoubleDot => TokenKind::DoubleDot,
            Token::Formatter => TokenKind::Formatter,
            Token::Wildcard => TokenKind::Wildcard,
            Token::Colon => TokenKind::Colon,
            Token::EndOfItem => TokenKind::EndOfItem,
            Token::EndOfBlock => TokenKind::EndOfBlock,
            Token::EndOfModule => TokenKind::EndOfModule,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TokenKind {
    VarIdentifier,
    TypeIdentifier,
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
    KeywordCase,
    KeywordOf,
    KeywordClass,
    KeywordConstraint,
    KeywordInstance,
    KeywordProtocol,
    KeywordActor,
    Op(BuiltinOperator),
    Equal,
    Comma,
    LCurly,
    RCurly,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Semicolon,
    Pipe,
    Lambda,
    Dot,
    DoubleDot,
    Formatter,
    Wildcard,
    Colon,
    EndOfItem,
    EndOfBlock,
    EndOfModule,
}

impl TokenKind {
    pub fn nice_name(&self) -> String {
        match self {
            TokenKind::TypeIdentifier => format!("type name"),
            TokenKind::VarIdentifier => format!("var name"),
            TokenKind::Pipe => format!("|"),
            TokenKind::LParen => format!("("),
            TokenKind::RParen => format!(")"),
            TokenKind::LBracket => format!("["),
            TokenKind::RBracket => format!("]"),
            TokenKind::LCurly => format!("{{"),
            TokenKind::RCurly => format!("}}"),
            TokenKind::Equal => format!("="),
            TokenKind::Op(BuiltinOperator::Bind) => format!("<-"),
            TokenKind::Op(BuiltinOperator::Arrow) => format!("->"),
            TokenKind::KeywordConstraint => format!("=>"),
            TokenKind::StringLiteral => format!("string literal"),
            TokenKind::FloatLiteral => format!("float literal"),
            TokenKind::IntegerLiteral => format!("integer literal"),
            TokenKind::BoolLiteral => format!("bool literal"),
            _ => {
                let name = format!("{:?}", self);
                let name = name.to_lowercase();
                let name = name.replace("keyword", "keyword ");
                name
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub token: Token,
    pub location: Location,
}
