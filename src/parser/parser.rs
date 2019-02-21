use super::expr::parse_ops;
use super::util::parse_parens;
use super::util::report_unexpected_token;
use super::util::to_string_list;
use super::util::ParenParseResult;
use crate::constants::BuiltinOperator;
use crate::error::Error;
use crate::location_info::filepath::FilePath;
use crate::location_info::location_info::Expr as LIExpr;
use crate::location_info::location_info::Function as LIFunction;
use crate::location_info::location_info::Import as LIImport;
use crate::location_info::location_info::LocationInfo;
use crate::location_info::location_info::Module as LIModule;
use crate::location_info::location_info::TypeSignature as LITypeSignature;
use crate::location_info::location_set::LocationSet;
use crate::syntax::expr::Expr;
use crate::syntax::expr::ExprId;
use crate::syntax::function::Function;
use crate::syntax::function::FunctionBody;
use crate::syntax::function::FunctionId;
use crate::syntax::function::FunctionType;
use crate::syntax::import::Import;
use crate::syntax::import::ImportId;
use crate::syntax::item_path::ItemPath;
use crate::syntax::module::Module;
use crate::syntax::module::ModuleId;
use crate::syntax::program::Program;
use crate::syntax::types::TypeSignature;
use crate::syntax::types::TypeSignatureId;
use crate::token::Token;
use crate::token::TokenInfo;
use crate::token::TokenKind;

pub struct Parser<'a> {
    file_path: FilePath,
    tokens: &'a [TokenInfo],
    index: usize,
    program: &'a mut Program,
    location_info: &'a mut LocationInfo,
}

impl<'a> Parser<'a> {
    pub fn new(
        file_path: FilePath,
        tokens: &'a [TokenInfo],
        program: &'a mut Program,
        location_info: &'a mut LocationInfo,
    ) -> Parser<'a> {
        Parser {
            file_path: file_path,
            tokens: tokens,
            index: 0,
            program: program,
            location_info: location_info,
        }
    }

    pub fn get_file_path(&self) -> FilePath {
        self.file_path.clone()
    }

    pub fn get_program(&mut self) -> &mut Program {
        &mut self.program
    }

    pub fn get_index(&self) -> usize {
        self.index
    }

    pub fn get_location_set(&self, start: usize, end: usize) -> LocationSet {
        let mut set = LocationSet::new(self.file_path.clone());
        for token in &self.tokens[start..end] {
            set.add(token.location.clone());
        }
        set
    }

    pub fn is_done(&self) -> bool {
        self.index >= self.tokens.len()
    }

    pub fn advance(&mut self) -> Result<TokenInfo, Error> {
        if self.is_done() {
            let last = &self.tokens[self.tokens.len() - 1];
            return Err(Error::parse_err(
                format!("Unexpected end of stream"),
                self.file_path.clone(),
                last.location.clone(),
            ));
        }
        let r = self.tokens[self.index].clone();
        self.index += 1;
        Ok(r)
    }

    pub fn prev(&self) -> TokenInfo {
        self.tokens[self.index - 1].clone()
    }

    pub fn peek(&self) -> Option<TokenInfo> {
        if self.is_done() {
            None
        } else {
            let r = self.tokens[self.index].clone();
            Some(r)
        }
    }

    pub fn lookahead(&self, offset: usize) -> Option<TokenInfo> {
        if self.index + offset >= self.tokens.len() {
            None
        } else {
            let r = self.tokens[self.index + offset].clone();
            Some(r)
        }
    }

    fn identifier(&mut self, msg: &str) -> Result<String, Error> {
        let token_info = self.peek().expect("Ran out of tokens");
        if let Token::Identifier(i) = token_info.token {
            self.advance()?;
            return Ok(i);
        } else {
            return report_unexpected_token(self.prev(), self, msg);
        }
    }

    pub fn current(&self, token: TokenKind) -> bool {
        if self.is_done() {
            return false;
        }
        self.tokens[self.index].token.kind() == token
    }

    pub fn current_kind(&self) -> TokenKind {
        self.tokens[self.index].token.kind()
    }

    pub fn expect(&mut self, token: TokenKind) -> Result<TokenInfo, Error> {
        let t = self.advance()?;
        if t.token.kind() == token {
            return Ok(t);
        } else {
            let found = t;
            return Err(Error::parse_err(
                format!("Expected {:?}, found {:?}", token, found.token.kind()),
                self.file_path.clone(),
                found.location,
            ));
        }
    }

    pub fn parse_list1(
        &mut self,
        item_kind: TokenKind,
        sep: TokenKind,
    ) -> Result<Option<Vec<TokenInfo>>, Error> {
        let mut items = Vec::new();
        loop {
            if let Some(item) = self.peek() {
                if item.token.kind() == item_kind {
                    items.push(item);
                    self.advance()?;
                    if !self.current(sep) {
                        break;
                    } else {
                        self.expect(sep)?;
                        continue;
                    }
                }
            }
            break;
        }
        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(items))
        }
    }

    fn parse_seq0(&mut self, item_kind: TokenKind) -> Result<Vec<TokenInfo>, Error> {
        let mut items = Vec::new();
        while !self.is_done() {
            if !self.current(item_kind) {
                break;
            }
            let item = self.expect(item_kind)?;
            items.push(item);
        }
        Ok(items)
    }

    pub fn consume_op(
        &mut self,
        op_kinds: &[BuiltinOperator],
    ) -> Option<(BuiltinOperator, TokenInfo)> {
        if let Some(op_token) = self.peek() {
            if let Some(op) = op_token.token.get_op() {
                if op_kinds.contains(&op) {
                    self.index += 1;
                    return Some((op, op_token));
                }
            }
        }
        None
    }

    pub fn parse_expr(&mut self) -> Result<Option<ExprId>, Error> {
        let id = parse_ops(self)?;
        Ok(id)
    }

    pub fn add_expr(&mut self, expr: Expr, start_index: usize) -> ExprId {
        let end_index = self.get_index();
        let location_set = self.get_location_set(start_index, end_index);
        let li_expr = LIExpr::new(location_set);
        let id = self.program.get_expr_id();
        self.program.add_expr(id, expr);
        self.location_info.add_expr(id, li_expr);
        id
    }

    pub fn add_type_signature(
        &mut self,
        type_signature: TypeSignature,
        start_index: usize,
    ) -> TypeSignatureId {
        let end_index = self.get_index();
        let location_set = self.get_location_set(start_index, end_index);
        let li_type_signature = LITypeSignature::new(location_set);
        let id = self.program.get_type_signature_id();
        self.program.add_type_signature(id, type_signature);
        self.location_info.add_type_signature(id, li_type_signature);
        id
    }

    fn parse_tuple_type(&mut self) -> Result<Option<TypeSignatureId>, Error> {
        let start_index = self.get_index();
        if let Some(res) = parse_parens(self, |p| p.parse_function_type())? {
            match res {
                ParenParseResult::Single(t) => {
                    return Ok(Some(t));
                }
                ParenParseResult::Tuple(ts) => {
                    let type_signature = TypeSignature::Tuple(ts);
                    let id = self.add_type_signature(type_signature, start_index);
                    return Ok(Some(id));
                }
            }
        } else {
            Ok(None)
        }
    }

    fn parse_function_type(&mut self) -> Result<Option<TypeSignatureId>, Error> {
        let start_index = self.get_index();
        let mut parts = Vec::new();
        loop {
            if let Some(part) = self.parse_type_part()? {
                parts.push(part);
                if let Some(next) = self.peek() {
                    match next.token {
                        Token::Op(BuiltinOperator::Arrow) => {
                            self.advance()?;
                        }
                        _ => {
                            break;
                        }
                    }
                }
            } else {
                break;
            }
        }
        let id: TypeSignatureId = match parts.len() {
            0 => {
                return Ok(None);
            }
            1 => parts.pop().unwrap(),
            _ => {
                let type_signature = TypeSignature::Function(parts);
                let id = self.add_type_signature(type_signature, start_index);
                id
            }
        };
        Ok(Some(id))
    }

    fn parse_type_part(&mut self) -> Result<Option<TypeSignatureId>, Error> {
        let start_index = self.get_index();
        match self.peek() {
            Some(token_info) => match token_info.token {
                Token::LParen => {
                    return self.parse_tuple_type();
                }
                Token::Op(BuiltinOperator::Not) => {
                    self.advance()?;
                    let id = self.add_type_signature(TypeSignature::Nothing, start_index);
                    return Ok(Some(id));
                }
                Token::Identifier(i) => {
                    self.advance()?;
                    let id = self.add_type_signature(TypeSignature::Named(i), start_index);
                    return Ok(Some(id));
                }
                _ => Ok(None),
            },
            None => Ok(None),
        }
    }

    fn parse_fn(&mut self, id: FunctionId) -> Result<Option<Function>, Error> {
        let mut start_index = self.get_index();
        let name = self.identifier("Expected identifier as function name")?;
        let mut name = name;
        let args = self.parse_seq0(TokenKind::Identifier)?;
        let mut end_index = self.get_index();
        let mut args: Vec<_> = to_string_list(args);
        let mut function_type = None;
        if let Some(next) = self.peek() {
            if next.token.kind() == TokenKind::KeywordDoubleColon {
                self.advance()?;
                if let Some(type_signature_id) = self.parse_function_type()? {
                    let full_type_signature_id = self.program.get_type_signature_id();
                    let location = self.get_location_set(start_index, self.get_index());
                    let li_full_type_signature = LITypeSignature::new(location);
                    self.location_info
                        .add_type_signature(full_type_signature_id, li_full_type_signature);
                    function_type = Some(FunctionType {
                        name: name,
                        type_args: args,
                        full_type_signature_id: full_type_signature_id,
                        type_signature_id: type_signature_id,
                    });
                    self.expect(TokenKind::EndOfItem)?;
                    start_index = self.get_index();
                    name = self.identifier("Expected identifier as function name")?;
                    let func_args = self.parse_seq0(TokenKind::Identifier)?;
                    end_index = self.get_index();
                    args = to_string_list(func_args);
                } else {
                    return Ok(None);
                }
            }
        }
        let location_set = self.get_location_set(start_index, end_index);
        let li_function = LIFunction::new(location_set);
        self.location_info.add_function(id, li_function);
        let equal = self.expect(TokenKind::Equal)?;
        let body = if let Some(token) = self.peek() {
            if token.token.kind() == TokenKind::KeywordExtern {
                self.expect(TokenKind::KeywordExtern)?;
                FunctionBody::Extern
            } else {
                let body = match self.parse_expr()? {
                    Some(body) => body,
                    None => {
                        if self.is_done() {
                            return Err(Error::parse_err(
                                format!("Expected expression as function body",),
                                self.file_path.clone(),
                                equal.location,
                            ));
                        } else {
                            let found = self.advance()?;
                            return Err(Error::parse_err(
                                format!(
                                    "Expected expression as function body, found {:?}",
                                    found.token.kind()
                                ),
                                self.file_path.clone(),
                                found.location,
                            ));
                        }
                    }
                };
                FunctionBody::Expr(body)
            }
        } else {
            return Err(Error::parse_err(
                format!("Expected expression as function body",),
                self.file_path.clone(),
                equal.location,
            ));
        };
        self.expect(TokenKind::EndOfItem)?;
        let function = Function {
            id: id,
            name: name,
            args: args,
            body: body,
            func_type: function_type,
        };
        Ok(Some(function))
    }

    pub fn parse_item_path(&mut self, msg: &str) -> Result<ItemPath, Error> {
        if let Some(name_parts) = self.parse_list1(TokenKind::Identifier, TokenKind::Dot)? {
            let name_parts: Vec<_> = to_string_list(name_parts);
            let path = ItemPath { path: name_parts };
            Ok(path)
        } else {
            return report_unexpected_token(self.prev(), self, msg);
        }
    }

    fn parse_import(&mut self, id: ImportId) -> Result<Import, Error> {
        let start_index = self.get_index();
        self.expect(TokenKind::KeywordImport)?;
        let name = self.parse_item_path("Expected import path")?;
        let mut alternative_name = None;
        let mut symbols = None;
        if let Some(token) = self.peek() {
            if let Token::LParen = token.token {
                self.advance()?;
                let symbol_names = self.parse_seq0(TokenKind::Identifier)?;
                symbols = Some(to_string_list(symbol_names));
                self.expect(TokenKind::RParen)?;
            }
        }
        if let Some(as_token) = self.peek() {
            if let Token::KeywordAs = as_token.token {
                self.advance()?;
                let name = self.identifier("Expected identifier after as in import")?;
                alternative_name = Some(name);
            }
        }
        let import = Import {
            id: id.clone(),
            module_path: name,
            alternative_name: alternative_name,
            symbols: symbols,
        };
        let end_index = self.get_index();
        let location_set = self.get_location_set(start_index, end_index);
        let li_import = LIImport::new(location_set);
        self.location_info.add_import(id, li_import);
        self.expect(TokenKind::EndOfItem)?;
        Ok(import)
    }

    fn parse_module(&mut self, id: ModuleId) -> Result<Module, Error> {
        self.expect(TokenKind::KeywordModule)?;
        let start_index = self.get_index();
        let name = self.parse_item_path("Expected module name")?;
        let end_index = self.get_index();
        let location_set = self.get_location_set(start_index, end_index);
        let li_module = LIModule::new(location_set);
        self.location_info.add_module(id, li_module);
        let mut module = Module::new(name, id);
        self.expect(TokenKind::KeywordWhere)?;
        loop {
            if let Some(token) = self.peek() {
                match token.token.kind() {
                    TokenKind::KeywordImport => {
                        let import_id = self.program.get_import_id();
                        let import = self.parse_import(import_id)?;
                        module.add_import(import_id, import);
                    }
                    TokenKind::EndOfBlock => {
                        break;
                    }
                    _ => {
                        let function_id = self.program.get_function_id();
                        if let Some(function) = self.parse_fn(function_id)? {
                            module.add_function(function_id, function);
                        } else {
                            return report_unexpected_token(token, self, "Expected function");
                        }
                    }
                }
            } else {
                break;
            }
        }
        self.expect(TokenKind::EndOfBlock)?;
        self.expect(TokenKind::EndOfModule)?;
        Ok(module)
    }

    pub fn parse(&mut self) -> Result<(), Error> {
        while !self.is_done() {
            let m_id = self.program.get_module_id();
            let module = self.parse_module(m_id)?;
            self.program.add_module(m_id, module);
        }

        Ok(())
    }
}
