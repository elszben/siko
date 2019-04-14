use super::expr::parse_ops;
use super::util::parse_parens;
use super::util::report_parser_error;
use super::util::report_unexpected_token;
use super::util::ParenParseResult;
use super::util::ParserErrorReason;
use crate::constants::BuiltinOperator;
use crate::constants::PRELUDE_NAME;
use crate::error::Error;
use crate::location_info::filepath::FilePath;
use crate::location_info::item::Item;
use crate::location_info::item::LocationId;
use crate::location_info::location_info::LocationInfo;
use crate::location_info::location_set::LocationSet;
use crate::parser::token::Token;
use crate::parser::token::TokenInfo;
use crate::parser::token::TokenKind;
use crate::syntax::data::Adt;
use crate::syntax::data::Data;
use crate::syntax::data::Record;
use crate::syntax::data::RecordField;
use crate::syntax::data::Variant;
use crate::syntax::data::VariantId;
use crate::syntax::export_import::EIGroup;
use crate::syntax::export_import::EIItem;
use crate::syntax::export_import::EIItemInfo;
use crate::syntax::export_import::EIList;
use crate::syntax::export_import::EIMember;
use crate::syntax::export_import::EIMemberInfo;
use crate::syntax::expr::Expr;
use crate::syntax::expr::ExprId;
use crate::syntax::function::Function;
use crate::syntax::function::FunctionBody;
use crate::syntax::function::FunctionId;
use crate::syntax::function::FunctionType;
use crate::syntax::import::HiddenItem;
use crate::syntax::import::Import;
use crate::syntax::import::ImportId;
use crate::syntax::import::ImportKind;
use crate::syntax::module::Module;
use crate::syntax::module::ModuleId;
use crate::syntax::program::Program;
use crate::syntax::types::TypeSignature;
use crate::syntax::types::TypeSignatureId;

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

    pub fn get_location_id(&mut self, start: usize, end: usize) -> LocationId {
        let mut set = LocationSet::new(self.file_path.clone());
        for token in &self.tokens[start..end] {
            set.add(token.location.clone());
        }
        let li_item = Item::new(set);
        let location_id = self.location_info.add_item(li_item);
        location_id
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

    fn restore(&mut self, index: usize) {
        self.index = index;
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

    pub fn identifier(&mut self, item: &str, simple: bool) -> Result<String, Error> {
        let token_info = self.peek().expect("Ran out of tokens");
        if let Token::Identifier(i) = token_info.token {
            if simple && i.contains(".") {
                let reason = ParserErrorReason::Custom {
                    msg: format!("invalid identifier as {}", item),
                };
                return report_parser_error(self, reason)?;
            }
            self.advance()?;
            return Ok(i);
        } else {
            return report_unexpected_token(self, item.to_string());
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
        let t = self.current_kind();
        if t == token {
            let t = self.advance()?;
            return Ok(t);
        } else {
            if token == TokenKind::EndOfItem {
                let reason = ParserErrorReason::Custom {
                    msg: format!("unexpected {}", t.nice_name()),
                };
                return report_parser_error(self, reason);
            }
            return report_unexpected_token(self, token.nice_name());
        }
    }

    pub fn parse_lambda_args(&mut self) -> Result<Vec<(String, LocationId)>, Error> {
        let mut args = Vec::new();
        loop {
            if let Some(item) = self.peek() {
                if item.token.kind() == TokenKind::Identifier {
                    let start_index = self.get_index();
                    let arg = self.identifier("lambda arg", true)?;
                    let end_index = self.get_index();
                    let location_id = self.get_location_id(start_index, end_index);
                    args.push((arg, location_id));
                    if !self.current(TokenKind::Comma) {
                        break;
                    } else {
                        self.expect(TokenKind::Comma)?;
                        continue;
                    }
                }
            }
            return report_unexpected_token(self, format!("lambda arg"));
        }
        assert!(!args.is_empty());
        Ok(args)
    }

    fn parse_list1_in_parens<T>(
        &mut self,
        parse_fn: fn(&mut Parser) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        self.expect(TokenKind::LParen)?;
        let mut items = Vec::new();
        loop {
            let item = parse_fn(self)?;
            items.push(item);
            let comma = if self.current(TokenKind::Comma) {
                self.expect(TokenKind::Comma)?;
                true
            } else {
                false
            };
            if self.current(TokenKind::RParen) {
                break;
            } else {
                if !comma {
                    return report_unexpected_token(self, format!("comma"));
                }
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(items)
    }

    fn parse_list0_in_parens<T>(
        &mut self,
        parse_fn: fn(&mut Parser) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        self.expect(TokenKind::LParen)?;
        let mut items = Vec::new();
        loop {
            if self.current(TokenKind::RParen) {
                break;
            }
            let item = parse_fn(self)?;
            items.push(item);
            if self.current(TokenKind::Comma) {
                self.expect(TokenKind::Comma)?;
            } else {
                break;
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(items)
    }

    fn parse_args(&mut self) -> Result<Vec<(String, LocationId)>, Error> {
        let mut items = Vec::new();
        while !self.is_done() {
            if !self.current(TokenKind::Identifier) {
                break;
            }
            let start_index = self.get_index();
            let item = self.identifier("argument", true)?;
            let end_index = self.get_index();
            let location_id = self.get_location_id(start_index, end_index);
            items.push((item, location_id));
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
        let location_id = self.get_location_id(start_index, end_index);
        let id = self.program.get_expr_id();
        self.program.add_expr(id, expr, location_id);
        id
    }

    pub fn add_type_signature(
        &mut self,
        type_signature: TypeSignature,
        start_index: usize,
    ) -> TypeSignatureId {
        let end_index = self.get_index();
        let location_id = self.get_location_id(start_index, end_index);
        let id = self.program.get_type_signature_id();
        self.program
            .add_type_signature(id, type_signature, location_id);

        id
    }

    fn parse_tuple_type(&mut self) -> Result<TypeSignatureId, Error> {
        let start_index = self.get_index();
        let res = parse_parens(self, |p| p.parse_function_type(false), "<type>")?;
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

    fn parse_function_type(&mut self, parsing_variant: bool) -> Result<TypeSignatureId, Error> {
        let start_index = self.get_index();
        let mut from = self.parse_type_part(parsing_variant)?;
        if let Some(next) = self.peek() {
            match next.token {
                Token::Op(BuiltinOperator::Arrow) => {
                    self.advance()?;
                    let to = self.parse_function_type(parsing_variant)?;
                    let ty = TypeSignature::Function(from, to);
                    let ty = self.add_type_signature(ty, start_index);
                    from = ty;
                }
                _ => {}
            }
        }
        Ok(from)
    }

    fn parse_type_part(&mut self, parsing_variant: bool) -> Result<TypeSignatureId, Error> {
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
                Token::Identifier(_) => {
                    let name = self.identifier("type", false)?;
                    let mut args = Vec::new();
                    loop {
                        match self.current_kind() {
                            TokenKind::Identifier => {
                                let arg_start_index = self.get_index();
                                let arg = self.identifier("type", false)?;
                                let arg = self.add_type_signature(
                                    TypeSignature::Named(arg, Vec::new()),
                                    arg_start_index,
                                );
                                args.push(arg);
                            }
                            TokenKind::LParen => {
                                let arg = self.parse_tuple_type()?;
                                args.push(arg);
                            }
                            _ => {
                                break;
                            }
                        }
                    }
                    let ty = if parsing_variant {
                        TypeSignature::Variant(name, args)
                    } else {
                        TypeSignature::Named(name, args)
                    };
                    let id = self.add_type_signature(ty, start_index);
                    return Ok(id);
                }
                _ => {
                    return report_unexpected_token(self, format!("type signature"));
                }
            },
            None => {
                return report_unexpected_token(self, format!("type signature"));
            }
        }
    }

    fn parse_fn(&mut self, id: FunctionId) -> Result<Function, Error> {
        let mut start_index = self.get_index();
        let name = self.identifier("function name", true)?;
        let mut name = name;
        let mut args = self.parse_args()?;
        let mut function_type = None;
        if let Some(next) = self.peek() {
            if next.token.kind() == TokenKind::KeywordDoubleColon {
                self.advance()?;
                let type_signature_id = self.parse_function_type(false)?;
                let full_type_signature_id = self.program.get_type_signature_id();
                let end_index = self.get_index();
                let location_id = self.get_location_id(start_index, end_index);
                function_type = Some(FunctionType {
                    name: name,
                    type_args: args,
                    full_type_signature_id: full_type_signature_id,
                    type_signature_id: type_signature_id,
                    location_id: location_id,
                });
                self.expect(TokenKind::EndOfItem)?;
                start_index = self.get_index();
                name = self.identifier("function name", true)?;
                args = self.parse_args()?;
            }
        }
        let end_index = self.get_index();
        let location_id = self.get_location_id(start_index, end_index);
        self.expect(TokenKind::Equal)?;
        let body = if let Some(token) = self.peek() {
            if token.token.kind() == TokenKind::KeywordExtern {
                self.expect(TokenKind::KeywordExtern)?;
                FunctionBody::Extern
            } else {
                let body = self.parse_expr()?;
                FunctionBody::Expr(body)
            }
        } else {
            unreachable!()
        };
        self.expect(TokenKind::EndOfItem)?;
        let function = Function {
            id: id,
            name: name,
            args: args,
            body: body,
            func_type: function_type,
            location_id: location_id,
        };
        Ok(function)
    }

    fn parse_export_import_data_member(parser: &mut Parser) -> Result<EIMemberInfo, Error> {
        let start_index = parser.get_index();
        let member = if parser.current(TokenKind::DoubleDot) {
            parser.expect(TokenKind::DoubleDot)?;
            EIMember::All
        } else {
            let name = parser.identifier("variant", true)?;
            EIMember::Specific(name)
        };
        let end_index = parser.get_index();
        let location_id = parser.get_location_id(start_index, end_index);
        let info = EIMemberInfo {
            member: member,
            location_id: location_id,
        };
        Ok(info)
    }

    fn parse_export_import_item(parser: &mut Parser) -> Result<EIItemInfo, Error> {
        let start_index = parser.get_index();
        let name = parser.identifier("exported item", true)?;
        let item = if parser.current(TokenKind::LParen) {
            let members = parser.parse_list0_in_parens(Parser::parse_export_import_data_member)?;
            let group = EIGroup {
                name: name,
                members: members,
            };
            EIItem::Group(group)
        } else {
            EIItem::Named(name)
        };
        let end_index = parser.get_index();
        let location_id = parser.get_location_id(start_index, end_index);
        let info = EIItemInfo {
            item: item,
            location_id: location_id,
        };
        Ok(info)
    }

    fn parse_hidden_item(parser: &mut Parser) -> Result<HiddenItem, Error> {
        let start_index = parser.get_index();
        let name = parser.identifier("hidden item", true)?;
        let end_index = parser.get_index();
        let location_id = parser.get_location_id(start_index, end_index);
        Ok(HiddenItem {
            name: name,
            location_id: location_id,
        })
    }

    fn parse_import(&mut self, id: ImportId) -> Result<Import, Error> {
        let start_index = self.get_index();
        self.expect(TokenKind::KeywordImport)?;
        let name = self.identifier("module name", false)?;
        let import_kind = if self.current(TokenKind::KeywordHiding) {
            self.expect(TokenKind::KeywordHiding)?;
            let items = self.parse_list1_in_parens(Parser::parse_hidden_item)?;
            ImportKind::Hiding(items)
        } else {
            let import_list = self.parse_export_import_list()?;
            let mut alternative_name = None;
            if let Some(as_token) = self.peek() {
                if let Token::KeywordAs = as_token.token {
                    self.advance()?;
                    let name = self.identifier("alternative name", true)?;
                    alternative_name = Some(name);
                }
            }
            ImportKind::ImportList {
                items: import_list,
                alternative_name: alternative_name,
            }
        };
        let end_index = self.get_index();
        let location_id = self.get_location_id(start_index, end_index);
        self.expect(TokenKind::EndOfItem)?;
        let import = Import {
            id: id.clone(),
            module_path: name,
            kind: import_kind,
            location_id: Some(location_id),
        };
        Ok(import)
    }

    fn parse_record_field(&mut self) -> Result<RecordField, Error> {
        let start_index = self.get_index();
        let name = self.identifier("record field name", true)?;
        self.expect(TokenKind::KeywordDoubleColon)?;
        let type_signature_id = self.parse_function_type(false)?;
        let end_index = self.get_index();
        let location_id = self.get_location_id(start_index, end_index);
        let item = RecordField {
            name: name,
            id: self.program.get_record_field_id(),
            type_signature_id: type_signature_id,
            location_id: location_id,
        };
        Ok(item)
    }

    fn parse_record(
        &mut self,
        name: String,
        type_args: Vec<(String, LocationId)>,
        start_index: usize,
    ) -> Result<Record, Error> {
        let mut fields = Vec::new();
        loop {
            let record_field = self.parse_record_field()?;
            fields.push(record_field);
            let mut found = false;
            if self.current(TokenKind::Comma) {
                found = true;
                self.expect(TokenKind::Comma)?;
            }
            if self.current(TokenKind::RCurly) {
                self.expect(TokenKind::RCurly)?;
                break;
            }
            if !found {
                return report_unexpected_token(self, format!("comma or }}"));
            }
        }
        let end_index = self.get_index();
        let location_id = self.get_location_id(start_index, end_index);
        let record = Record {
            name: name,
            id: self.program.get_record_id(),
            type_args: type_args,
            fields: fields,
            location_id: location_id,
        };
        Ok(record)
    }

    fn parse_variant(&mut self) -> Result<VariantId, Error> {
        let variant_start_index = self.get_index();
        let name = self.identifier("variant", true)?;
        self.restore(variant_start_index);
        let type_signature_id = self.parse_function_type(true)?;
        let end_index = self.get_index();
        let location_id = self.get_location_id(variant_start_index, end_index);
        let id = self.program.get_variant_id();
        let variant = Variant {
            id: id,
            name: name,
            type_signature_id: type_signature_id,
            location_id: location_id,
        };
        self.program.variants.insert(id, variant);
        Ok(id)
    }

    fn parse_data(&mut self) -> Result<Data, Error> {
        let start_index = self.get_index();
        self.expect(TokenKind::KeywordData)?;
        let name = self.identifier("type", true)?;
        let args = self.parse_args()?;
        self.expect(TokenKind::Equal)?;
        if self.current(TokenKind::LCurly) {
            self.expect(TokenKind::LCurly)?;
            let record = self.parse_record(name, args, start_index)?;
            Ok(Data::Record(record))
        } else {
            let mut variants = Vec::new();
            loop {
                let variant = self.parse_variant()?;
                variants.push(variant);
                if self.current(TokenKind::Pipe) {
                    self.expect(TokenKind::Pipe)?;
                } else {
                    break;
                }
            }
            let end_index = self.get_index();
            let location_id = self.get_location_id(start_index, end_index);
            let adt = Adt {
                name: name,
                id: self.program.get_adt_id(),
                type_args: args,
                variants: variants,
                location_id: location_id,
            };
            Ok(Data::Adt(adt))
        }
    }

    fn parse_export_import_list(&mut self) -> Result<EIList, Error> {
        let export_list = if self.current(TokenKind::LParen) {
            let items = self.parse_list0_in_parens(Parser::parse_export_import_item)?;
            EIList::Explicit(items)
        } else {
            EIList::ImplicitAll
        };
        Ok(export_list)
    }

    fn parse_module(&mut self, id: ModuleId) -> Result<Module, Error> {
        self.expect(TokenKind::KeywordModule)?;
        let start_index = self.get_index();
        let name = self.identifier("module name", false)?;
        let end_index = self.get_index();
        let location_id = self.get_location_id(start_index, end_index);
        let export_list = self.parse_export_import_list()?;
        let mut module = Module::new(name, id, location_id, export_list);
        self.expect(TokenKind::KeywordWhere)?;
        loop {
            if let Some(token) = self.peek() {
                match token.token.kind() {
                    TokenKind::KeywordImport => {
                        let import_id = self.program.get_import_id();
                        let import = self.parse_import(import_id)?;
                        module.imports.insert(import_id, import);
                    }
                    TokenKind::KeywordData => {
                        let data = self.parse_data()?;
                        self.expect(TokenKind::EndOfItem)?;
                        match data {
                            Data::Record(record) => {
                                module.records.push(record.id);
                                self.program.records.insert(record.id, record);
                            }
                            Data::Adt(adt) => {
                                module.adts.push(adt.id);
                                self.program.adts.insert(adt.id, adt);
                            }
                        }
                    }
                    TokenKind::EndOfBlock => {
                        break;
                    }
                    _ => {
                        let function_id = self.program.get_function_id();
                        let function = self.parse_fn(function_id)?;
                        module.functions.push(function_id);
                        self.program.functions.insert(function_id, function);
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

        let mut prelude_exists = false;
        for (_, module) in self.program.modules.iter_mut() {
            if module.name == PRELUDE_NAME {
                prelude_exists = true;
                break;
            }
        }

        if prelude_exists {
            let mut modules_without_prelude = Vec::new();
            for (module_id, module) in &self.program.modules {
                let mut prelude_imported = false;
                if module.name == PRELUDE_NAME {
                    continue;
                }
                for (_, import) in &module.imports {
                    if import.module_path == PRELUDE_NAME {
                        prelude_imported = true;
                        break;
                    }
                }
                if !prelude_imported {
                    modules_without_prelude.push(*module_id);
                }
            }
            for module_id in modules_without_prelude {
                let import_id = self.program.get_import_id();
                let import = Import {
                    id: import_id,
                    module_path: PRELUDE_NAME.to_string(),
                    kind: ImportKind::ImportList {
                        items: EIList::ImplicitAll,
                        alternative_name: None,
                    },
                    location_id: None,
                };
                let module = self
                    .program
                    .modules
                    .get_mut(&module_id)
                    .expect("Module not found");
                module.imports.insert(import_id, import);
            }
        }

        Ok(())
    }
}
