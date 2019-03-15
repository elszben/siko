use super::expr::parse_ops;
use super::util::parse_parens;
use super::util::report_unexpected_token;
use super::util::to_string_list;
use super::util::ParenParseResult;
use crate::constants::BuiltinOperator;
use crate::error::Error;
use crate::location_info::filepath::FilePath;
use crate::location_info::item::Item;
use crate::location_info::item::LocationId;
use crate::location_info::location_info::Expr as LIExpr;
use crate::location_info::location_info::Function as LIFunction;
use crate::location_info::location_info::Import as LIImport;
use crate::location_info::location_info::LocationInfo;
use crate::location_info::location_info::TypeSignature as LITypeSignature;
use crate::location_info::location_set::LocationSet;
use crate::syntax::data::Adt;
use crate::syntax::data::Data;
use crate::syntax::data::Record;
use crate::syntax::data::RecordItem;
use crate::syntax::data::RecordOrVariant;
use crate::syntax::data::Variant;
use crate::syntax::expr::Expr;
use crate::syntax::expr::ExprId;
use crate::syntax::function::Function;
use crate::syntax::function::FunctionBody;
use crate::syntax::function::FunctionId;
use crate::syntax::function::FunctionType;
use crate::syntax::import::DataConstructor;
use crate::syntax::import::HiddenItem;
use crate::syntax::import::Import;
use crate::syntax::import::ImportId;
use crate::syntax::import::ImportKind;
use crate::syntax::import::ImportList;
use crate::syntax::import::ImportedItem;
use crate::syntax::import::TypeConstructor;
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

    pub fn get_last(&self) -> TokenInfo {
        self.tokens[self.tokens.len() - 1].clone()
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
            return report_unexpected_token(self, msg);
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
    ) -> Result<Vec<TokenInfo>, Error> {
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
            return report_unexpected_token(self, &format!("Expected {:?}", item_kind));
        }
        assert!(!items.is_empty());
        Ok(items)
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

    pub fn parse_expr(&mut self) -> Result<ExprId, Error> {
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

    fn parse_tuple_type(&mut self) -> Result<TypeSignatureId, Error> {
        let start_index = self.get_index();
        let res = parse_parens(self, |p| p.parse_function_type(), "<type>")?;
        match res {
            ParenParseResult::Single(t) => {
                return Ok(t);
            }
            ParenParseResult::Tuple(ts) => {
                let type_signature = TypeSignature::Tuple(ts);
                let id = self.add_type_signature(type_signature, start_index);
                return Ok(id);
            }
        }
    }

    fn parse_function_type(&mut self) -> Result<TypeSignatureId, Error> {
        let start_index = self.get_index();
        let mut parts = Vec::new();
        loop {
            let part = self.parse_type_part()?;
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
        }
        let id: TypeSignatureId = match parts.len() {
            0 => unreachable!(),
            1 => parts.pop().unwrap(),
            _ => {
                let type_signature = TypeSignature::Function(parts);
                let id = self.add_type_signature(type_signature, start_index);
                id
            }
        };
        Ok(id)
    }

    fn parse_type_part(&mut self) -> Result<TypeSignatureId, Error> {
        let start_index = self.get_index();
        match self.peek() {
            Some(token_info) => match token_info.token {
                Token::LParen => {
                    return self.parse_tuple_type();
                }
                Token::Op(BuiltinOperator::Not) => {
                    self.advance()?;
                    let id = self.add_type_signature(TypeSignature::Nothing, start_index);
                    return Ok(id);
                }
                Token::Identifier(i) => {
                    self.advance()?;
                    let mut args = Vec::new();
                    loop {
                        match self.current_kind() {
                            TokenKind::Identifier => {
                                let arg_start_index = self.get_index();
                                let arg =
                                    self.identifier("Expected identifier as type argument")?;
                                let arg = self.add_type_signature(
                                    TypeSignature::TypeArgument(arg),
                                    arg_start_index,
                                );
                                args.push(arg);
                            }
                            _ => {
                                break;
                            }
                        }
                    }
                    let id = self.add_type_signature(TypeSignature::Named(i, args), start_index);
                    return Ok(id);
                }
                _ => {
                    return report_unexpected_token(self, "Expected type signature");
                }
            },
            None => {
                return report_unexpected_token(self, "Expected type signature");
            }
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
                let type_signature_id = self.parse_function_type()?;
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
                let body = self.parse_expr()?;
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

    pub fn parse_item_path(&mut self) -> Result<ItemPath, Error> {
        let name_parts = self.parse_list1(TokenKind::Identifier, TokenKind::Dot)?;
        let name_parts: Vec<_> = to_string_list(name_parts);
        let path = ItemPath { path: name_parts };
        Ok(path)
    }

    fn parse_imported_item(&mut self) -> Result<ImportedItem, Error> {
        let name = self.identifier("Expected identifier as imported item")?;
        if self.current(TokenKind::LParen) {
            let mut type_ctor = TypeConstructor {
                name: name,
                data_constructors: Vec::new(),
            };
            self.expect(TokenKind::LParen)?;
            if self.current(TokenKind::Dot) {
                self.expect(TokenKind::Dot)?;
                self.expect(TokenKind::Dot)?;
            } else {
                loop {
                    if self.current(TokenKind::RParen) {
                        break;
                    }
                    let name = self.identifier("Expected identifier as data constructor")?;
                    let data_constructor = DataConstructor { name: name };
                    type_ctor.data_constructors.push(data_constructor);
                    if self.current(TokenKind::Comma) {
                        self.expect(TokenKind::Comma)?;
                    } else {
                        break;
                    }
                }
            }
            self.expect(TokenKind::RParen)?;
            Ok(ImportedItem::TypeConstructor(type_ctor))
        } else {
            Ok(ImportedItem::FunctionOrRecord(name))
        }
    }

    fn parse_import(&mut self, id: ImportId) -> Result<Import, Error> {
        let start_index = self.get_index();
        self.expect(TokenKind::KeywordImport)?;
        let name = self.parse_item_path()?;
        let import_kind = if self.current(TokenKind::KeywordHiding) {
            self.expect(TokenKind::LParen)?;
            let symbol_names = self.parse_seq0(TokenKind::Identifier)?;
            let symbols = to_string_list(symbol_names);
            let items: Vec<_> = symbols
                .into_iter()
                .map(|s| HiddenItem { name: s })
                .collect();
            self.expect(TokenKind::RParen)?;
            ImportKind::Hiding(items)
        } else {
            let import_list = if self.current(TokenKind::LParen) {
                self.advance()?;
                let symbol_names = self.parse_seq0(TokenKind::Identifier)?;
                let symbols = to_string_list(symbol_names);
                let symbols: Vec<_> = symbols
                    .into_iter()
                    .map(|s| ImportedItem::FunctionOrRecord(s))
                    .collect();
                self.expect(TokenKind::RParen)?;
                ImportList::Explicit(symbols)
            } else {
                ImportList::ImplicitAll
            };
            let mut alternative_name = None;
            if let Some(as_token) = self.peek() {
                if let Token::KeywordAs = as_token.token {
                    self.advance()?;
                    let name = self.identifier("Expected identifier after as in import")?;
                    alternative_name = Some(name);
                }
            }
            ImportKind::ImportList {
                items: import_list,
                alternative_name: alternative_name,
            }
        };
        let import = Import {
            id: id.clone(),
            module_path: name,
            kind: import_kind,
        };
        let end_index = self.get_index();
        let location_set = self.get_location_set(start_index, end_index);
        let li_import = LIImport::new(location_set);
        self.location_info.add_import(id, li_import);
        self.expect(TokenKind::EndOfItem)?;
        Ok(import)
    }

    fn parse_record_item(&mut self) -> Result<RecordItem, Error> {
        let name = self.identifier("Expected identifier as record item name")?;
        self.expect(TokenKind::KeywordDoubleColon)?;
        let type_signature_id = self.parse_function_type()?;
        let item = RecordItem {
            name: name,
            id: self.program.get_record_item_id(),
            type_signature_id: type_signature_id,
        };
        Ok(item)
    }

    fn parse_record(&mut self, name: String) -> Result<Record, Error> {
        let mut items = Vec::new();
        loop {
            let record_item = self.parse_record_item()?;
            items.push(record_item);
            if self.current(TokenKind::Comma) {
                self.expect(TokenKind::Comma)?;
            }
            if self.current(TokenKind::RCurly) {
                self.expect(TokenKind::RCurly)?;
                break;
            }
        }
        let record = Record {
            name: name,
            id: self.program.get_record_id(),
            items: items,
        };
        Ok(record)
    }

    fn parse_variant(&mut self) -> Result<RecordOrVariant, Error> {
        let name = self.identifier("Expected identifier as variant")?;
        if self.current(TokenKind::LCurly) {
            self.expect(TokenKind::LCurly)?;
            let record = self.parse_record(name)?;
            Ok(RecordOrVariant::Record(record))
        } else {
            let mut items = Vec::new();
            loop {
                match self.current_kind() {
                    TokenKind::LParen | TokenKind::Identifier => {
                        let variant_item = self.parse_function_type()?;
                        items.push(variant_item);
                    }
                    _ => {
                        break;
                    }
                }
            }
            let variant = Variant {
                id: self.program.get_variant_id(),
                name: name,
                items: items,
            };
            Ok(RecordOrVariant::Variant(variant))
        }
    }

    fn parse_data(&mut self) -> Result<Data, Error> {
        let mut start_index = self.get_index();
        self.expect(TokenKind::KeywordData)?;
        let name = self.identifier("Expected identifier as data name")?;
        let args = self.parse_seq0(TokenKind::Identifier)?;
        let mut end_index = self.get_index();
        let args: Vec<_> = to_string_list(args);
        self.expect(TokenKind::Equal)?;
        let mut variants = Vec::new();
        loop {
            let variant_token = self.peek().expect("Variant location error");
            let variant_location = variant_token.location;
            let record_or_variant = self.parse_variant()?;
            match record_or_variant {
                RecordOrVariant::Record(record) => {
                    if !variants.is_empty() {
                        return Err(Error::parse_err(
                            format!("Record cannot appear as a variant in data definition"),
                            self.get_file_path(),
                            variant_location,
                        ));
                    }
                    self.expect(TokenKind::EndOfItem)?;
                    return Ok(Data::Record(record));
                }
                RecordOrVariant::Variant(variant) => {
                    variants.push(variant);
                }
            }
            if self.current(TokenKind::Pipe) {
                self.expect(TokenKind::Pipe)?;
            } else {
                break;
            }
        }
        self.expect(TokenKind::EndOfItem)?;
        let adt = Adt {
            name: name,
            id: self.program.get_adt_id(),
            type_args: args,
            variants: variants,
        };
        Ok(Data::Adt(adt))
    }

    fn parse_module(&mut self, id: ModuleId) -> Result<Module, Error> {
        self.expect(TokenKind::KeywordModule)?;
        let start_index = self.get_index();
        let name = self.parse_item_path()?;
        let end_index = self.get_index();
        let location_set = self.get_location_set(start_index, end_index);
        let li_item = Item::new(location_set);
        let location_id = self.location_info.add_item(li_item);
        let mut module = Module::new(name, id, location_id);
        self.expect(TokenKind::KeywordWhere)?;
        loop {
            if let Some(token) = self.peek() {
                match token.token.kind() {
                    TokenKind::KeywordImport => {
                        let import_id = self.program.get_import_id();
                        let import = self.parse_import(import_id)?;
                        module.add_import(import_id, import);
                    }
                    TokenKind::KeywordData => {
                        self.parse_data()?;
                    }
                    TokenKind::EndOfBlock => {
                        break;
                    }
                    _ => {
                        let function_id = self.program.get_function_id();
                        if let Some(function) = self.parse_fn(function_id)? {
                            module.add_function(function_id, function);
                        } else {
                            return report_unexpected_token(self, "Expected function");
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
