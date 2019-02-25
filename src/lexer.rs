use crate::constants::BuiltinOperator;
use crate::error::Error;
use crate::location_info::filepath::FilePath;
use crate::location_info::location::Location;
use crate::location_info::span::Span;
use crate::token::Token;
use crate::token::TokenInfo;
use crate::token::TokenKind;

#[derive(Debug)]
pub struct Lexer {
    file_path: FilePath,
    index: usize,
    line_index: usize,
    input: Vec<char>,
    line_offset: usize,
    tokens: Vec<TokenInfo>,
}

impl Lexer {
    pub fn new(input: &str, file_path: FilePath) -> Lexer {
        Lexer {
            file_path: file_path,
            index: 0,
            line_index: 0,
            input: input.chars().collect(),
            line_offset: 0,
            tokens: Vec::new(),
        }
    }

    fn is_done(&self) -> bool {
        self.index >= self.input.len()
    }

    fn advance(&mut self) -> Result<char, Error> {
        if self.is_done() {
            return Err(Error::lexer_err(
                format!("unexpected end of file"),
                self.file_path.clone(),
                Location::new(self.line_index, Span::single(self.line_offset)),
            ));
        } else {
            let c = self.input[self.index];
            self.index += 1;
            self.line_offset += 1;
            Ok(c)
        }
    }

    fn peek(&mut self) -> Result<char, Error> {
        if self.is_done() {
            return Err(Error::lexer_err(
                format!("unexpected end of file"),
                self.file_path.clone(),
                Location::new(self.line_index, Span::single(self.line_offset)),
            ));
        } else {
            let c = self.input[self.index];
            Ok(c)
        }
    }

    fn is_identifier(c: char) -> bool {
        match c {
            'a'...'z' | 'A'...'Z' | '0'...'9' | '_' => true,
            _ => false,
        }
    }

    fn is_operator(c: char) -> bool {
        match c {
            '|' | '>' | '<' | '&' | '*' | '+' | '-' | '/' | '=' | '!' | '\\' | ':' => true,
            _ => false,
        }
    }

    fn add_token(&mut self, token: Token, span: Span) {
        self.tokens.push(TokenInfo {
            token: token,
            location: Location::new(self.line_index, span),
        });
    }

    fn collect(&mut self, filter_fn: fn(char) -> bool) -> Result<(String, Span), Error> {
        let start = self.line_offset;
        let mut token = String::new();
        while !self.is_done() {
            let c = self.peek()?;
            if (filter_fn)(c) {
                token.push(c);
                self.advance()?;
            } else {
                break;
            }
        }
        let span = Span {
            start: start,
            end: self.line_offset,
        };
        Ok((token, span))
    }

    fn collect_identifier(&mut self) -> Result<(), Error> {
        let (identifier, span) = self.collect(Lexer::is_identifier)?;

        let t = match identifier.as_ref() {
            "where" => Token::KeywordWhere,
            "data" => Token::KeywordData,
            "module" => Token::KeywordModule,
            "import" => Token::KeywordImport,
            "if" => Token::KeywordIf,
            "then" => Token::KeywordThen,
            "else" => Token::KeywordElse,
            "do" => Token::KeywordDo,
            "True" => Token::BoolLiteral(true),
            "False" => Token::BoolLiteral(false),
            "as" => Token::KeywordAs,
            "extern" => Token::KeywordExtern,
            _ => match identifier.parse::<i64>() {
                Ok(_) => Token::NumericLiteral(identifier),
                Err(_) => Token::Identifier(identifier),
            },
        };
        self.add_token(t, span);
        Ok(())
    }

    fn collect_operator(&mut self) -> Result<(), Error> {
        let (operator, span) = self.collect(Lexer::is_operator)?;
        let t = match operator.as_ref() {
            "+" => Token::Op(BuiltinOperator::Add),
            "-" => Token::Op(BuiltinOperator::Sub),
            "*" => Token::Op(BuiltinOperator::Mul),
            "/" => Token::Op(BuiltinOperator::Div),
            "|>" => Token::Op(BuiltinOperator::PipeForward),
            "&&" => Token::Op(BuiltinOperator::And),
            "||" => Token::Op(BuiltinOperator::Or),
            "=" => Token::Equal,
            "==" => Token::Op(BuiltinOperator::Equals),
            "!=" => Token::Op(BuiltinOperator::NotEquals),
            "<" => Token::Op(BuiltinOperator::LessThan),
            "<=" => Token::Op(BuiltinOperator::LessOrEqualThan),
            ">" => Token::Op(BuiltinOperator::GreaterThan),
            ">=" => Token::Op(BuiltinOperator::GreaterOrEqualThan),
            "|" => Token::Pipe,
            "\\" => Token::Lambda,
            "!" => Token::Op(BuiltinOperator::Not),
            "<-" => Token::Op(BuiltinOperator::Bind),
            "->" => Token::Op(BuiltinOperator::Arrow),
            "::" => Token::KeywordDoubleColon,
            _ => {
                return Err(Error::lexer_err(
                    format!("Unsupported operator {}", operator),
                    self.file_path.clone(),
                    Location::new(self.line_index, span),
                ))
            }
        };
        self.add_token(t, span);
        Ok(())
    }

    fn collect_string_literal(&mut self) -> Result<(), Error> {
        let start = self.line_offset;
        let mut prev_backslash = false;
        let mut literal = String::new();
        let mut inside = true;
        self.advance()?;
        while !self.is_done() {
            let c = self.peek()?;
            if c == '\\' {
                if prev_backslash {
                    prev_backslash = false;
                    literal.push(c);
                    self.advance()?;
                } else {
                    prev_backslash = true;
                    self.advance()?;
                }
            } else {
                if prev_backslash {
                    prev_backslash = false;
                    let special = match c {
                        'n' => '\n',
                        't' => '\t',
                        '"' => '"',
                        _ => {
                            return Err(Error::lexer_err(
                                format!("Invalid escape sequence \\{}", c),
                                self.file_path.clone(),
                                Location::new(self.line_index, Span::single(self.line_offset)),
                            ));
                        }
                    };
                    literal.push(special);
                    self.advance()?;
                } else {
                    if c == '"' {
                        inside = false;
                        self.advance()?;
                        break;
                    }
                    if c == '\n' {
                        break;
                    }
                    literal.push(c);
                    self.advance()?;
                }
            }
        }
        if inside {
            return Err(Error::lexer_err(
                format!("Unexpected end of string literal",),
                self.file_path.clone(),
                Location::new(self.line_index, Span::new(start, self.line_offset)),
            ));
        }
        let span = Span {
            start: start,
            end: self.line_offset,
        };
        self.add_token(Token::StringLiteral(literal), span);
        Ok(())
    }

    pub fn process(&mut self) -> Result<Vec<TokenInfo>, Error> {
        loop {
            if self.is_done() {
                break;
            }
            let c = self.peek()?;
            if Lexer::is_identifier(c) {
                self.collect_identifier()?;
            } else if Lexer::is_operator(c) {
                self.collect_operator()?;
            } else if c == '"' {
                self.collect_string_literal()?;
            } else {
                let span = Span::single(self.line_offset);
                let t = match c {
                    ' ' => {
                        self.advance()?;
                        continue;
                    }
                    '\n' => {
                        self.advance()?;
                        self.line_index += 1;
                        self.line_offset = 0;
                        continue;
                    }
                    '=' => Token::Equal,
                    ',' => Token::Comma,
                    '.' => Token::Dot,
                    '{' => Token::LCurly,
                    '}' => Token::RCurly,
                    '(' => Token::LParen,
                    ')' => Token::RParen,
                    ';' => Token::Semicolon,
                    _ => {
                        return Err(Error::lexer_err(
                            format!("Unsupported character: {}", c),
                            self.file_path.clone(),
                            Location::new(self.line_index, span),
                        ))
                    }
                };
                self.add_token(t, span);
                self.advance()?;
            }
        }
        let mut token_iterator = TokenIterator::new(self.tokens.clone());
        process_program(&mut token_iterator, &self.file_path)?;
        let mut result = token_iterator.result;
        result.pop();
        Ok(result)
    }
}

struct TokenIterator {
    tokens: Vec<TokenInfo>,
    result: Vec<TokenInfo>,
}

impl TokenIterator {
    fn new(tokens: Vec<TokenInfo>) -> TokenIterator {
        TokenIterator {
            tokens: tokens,
            result: Vec::new(),
        }
    }

    fn is_done(&self) -> bool {
        self.tokens.is_empty()
    }

    fn peek(&self) -> TokenInfo {
        self.tokens.first().expect("ran out of tokeninfo").clone()
    }

    fn advance(&mut self) -> TokenInfo {
        self.tokens.remove(0)
    }

    fn add_end(&mut self, token: Token) {
        let location = self.result.last().expect("empty iterator").location.clone();
        self.result.push(TokenInfo {
            token: token,
            location: location,
        });
    }
}

fn process_block(
    iterator: &mut TokenIterator,
    block_token: TokenInfo,
    module: bool,
    file_path: &FilePath,
) -> Result<(), Error> {
    if iterator.is_done() {
        return Err(Error::lexer_err(
            format!("Empty block"),
            file_path.clone(),
            block_token.location.clone(),
        ));
    }
    let first = iterator.peek();
    while !iterator.is_done() {
        let end_of_block = process_item(iterator, first.location.clone(), module, file_path)?;
        if end_of_block {
            break;
        }
    }
    if !module {
        iterator.add_end(Token::EndOfBlock);
    }
    Ok(())
}

fn process_program(iterator: &mut TokenIterator, file_path: &FilePath) -> Result<(), Error> {
    while !iterator.is_done() {
        let module_token = iterator.peek();
        if module_token.token.kind() != TokenKind::KeywordModule {
            return Err(Error::lexer_err(
                format!("Expected keyword module"),
                file_path.clone(),
                module_token.location.clone(),
            ));
        }
        let module = iterator.advance();
        iterator.result.push(module);
        if !iterator.is_done() {
            process_block(iterator, module_token, true, file_path)?;
            iterator.add_end(Token::EndOfModule);
        }
    }
    iterator.add_end(Token::EndOfModule);
    Ok(())
}

fn process_item(
    iterator: &mut TokenIterator,
    start: Location,
    module: bool,
    file_path: &FilePath,
) -> Result<bool, Error> {
    let mut first = true;
    let mut paren_level = 0;
    while !iterator.is_done() {
        let info = iterator.peek();
        if first {
            first = false;
        } else {
            if info.location.span.start <= start.span.start {
                if !module {
                    iterator.add_end(Token::EndOfItem);
                }
                return Ok(info.location.span.start < start.span.start);
            }
        }
        if info.token.kind() == TokenKind::KeywordModule {
            return Ok(true);
        }
        if info.token.kind() == TokenKind::LParen {
            paren_level += 1;
        }
        if info.token.kind() == TokenKind::RParen {
            paren_level -= 1;
            if paren_level < 0 {
                break;
            }
        }
        iterator.result.push(info.clone());
        if info.token.kind() == TokenKind::KeywordWhere || info.token.kind() == TokenKind::KeywordDo
        {
            iterator.advance();
            process_block(iterator, info, false, file_path)?;
        } else {
            iterator.advance();
        }
    }
    if !module {
        iterator.add_end(Token::EndOfItem);
    }
    Ok(true)
}
