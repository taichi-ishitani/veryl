use crate::analyzer_error::AnalyzerError;
use crate::evaluator::{EvaluatedError, Evaluator};
use crate::symbol::{Direction, GenericBoundKind, SymbolId, SymbolKind};
use crate::symbol_table;
use veryl_parser::veryl_grammar_trait::*;
use veryl_parser::veryl_token::TokenRange;
use veryl_parser::veryl_walker::{Handler, HandlerPoint};
use veryl_parser::ParolError;

#[derive(Default)]
pub struct CheckExpression<'a> {
    pub errors: Vec<AnalyzerError>,
    text: &'a str,
    point: HandlerPoint,
    evaluator: Evaluator,
    in_inst_declaration: bool,
    port_direction: Option<Direction>,
    in_input_port_default_value: bool,
}

impl<'a> CheckExpression<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            ..Default::default()
        }
    }

    fn evaluated_error(&mut self, errors: &[EvaluatedError]) {
        for e in errors {
            self.errors
                .push(AnalyzerError::evaluated_error(self.text, e));
        }
    }
}

impl Handler for CheckExpression<'_> {
    fn set_point(&mut self, p: HandlerPoint) {
        self.point = p;
    }
}

fn is_defined_in_package(full_path: &[SymbolId]) -> bool {
    for path in full_path {
        let symbol = symbol_table::get(*path).unwrap();
        if matches!(symbol.kind, SymbolKind::Package(_)) {
            return true;
        }
    }

    let symbol = symbol_table::get(*full_path.last().unwrap()).unwrap();
    if let Some(parent) = symbol.get_parent() {
        if matches!(parent.kind, SymbolKind::Package(_)) {
            return true;
        } else {
            return is_defined_in_package(&[parent.id]);
        }
    }

    false
}

impl VerylGrammarTrait for CheckExpression<'_> {
    fn identifier_factor(&mut self, arg: &IdentifierFactor) -> Result<(), ParolError> {
        if let HandlerPoint::Before = self.point {
            let expid = arg.expression_identifier.as_ref();
            if let Ok(rr) = symbol_table::resolve(expid) {
                // Only generic const or globally visible identifier can be used as port default value
                if self.in_input_port_default_value {
                    let port_default_available = match &rr.found.kind {
                        SymbolKind::SystemFunction => true,
                        SymbolKind::GenericParameter(x) => {
                            matches!(x.bound, GenericBoundKind::Const)
                        }
                        _ => is_defined_in_package(&rr.full_path),
                    };

                    if !port_default_available {
                        let identifier = rr.found.token.to_string();
                        let token: TokenRange = arg.expression_identifier.as_ref().into();
                        let kind_name = rr.found.kind.to_kind_name();

                        self.errors.push(AnalyzerError::invalid_factor(
                            &identifier,
                            &kind_name,
                            self.text,
                            &token,
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn let_statement(&mut self, arg: &LetStatement) -> Result<(), ParolError> {
        let exp = self.evaluator.expression(&arg.expression);
        self.evaluated_error(&exp.errors);

        // TODO type check

        Ok(())
    }

    fn identifier_statement(&mut self, arg: &IdentifierStatement) -> Result<(), ParolError> {
        match arg.identifier_statement_group.as_ref() {
            IdentifierStatementGroup::FunctionCall(_) => {
                // TODO function check
            }
            IdentifierStatementGroup::Assignment(x) => {
                let exp = self.evaluator.expression(&x.assignment.expression);
                self.evaluated_error(&exp.errors);

                // TODO type check
            }
        }
        Ok(())
    }

    fn if_statement(&mut self, arg: &IfStatement) -> Result<(), ParolError> {
        let exp = self.evaluator.expression(&arg.expression);
        self.evaluated_error(&exp.errors);

        // TODO type check

        for x in &arg.if_statement_list {
            let exp = self.evaluator.expression(&x.expression);
            self.evaluated_error(&exp.errors);

            // TODO type check
        }

        Ok(())
    }

    fn if_reset_statement(&mut self, arg: &IfResetStatement) -> Result<(), ParolError> {
        for x in &arg.if_reset_statement_list {
            let exp = self.evaluator.expression(&x.expression);
            self.evaluated_error(&exp.errors);

            // TODO type check
        }

        Ok(())
    }

    fn return_statement(&mut self, arg: &ReturnStatement) -> Result<(), ParolError> {
        let exp = self.evaluator.expression(&arg.expression);
        self.evaluated_error(&exp.errors);

        // TODO type check

        Ok(())
    }

    fn for_statement(&mut self, arg: &ForStatement) -> Result<(), ParolError> {
        let exp = self.evaluator.expression(&arg.range.expression);
        self.evaluated_error(&exp.errors);

        // TODO type check

        if let Some(x) = &arg.range.range_opt {
            let exp = self.evaluator.expression(&x.expression);
            self.evaluated_error(&exp.errors);

            // TODO type check
        }

        if let Some(x) = &arg.for_statement_opt {
            let exp = self.evaluator.expression(&x.expression);
            self.evaluated_error(&exp.errors);

            // TODO type check
        }

        Ok(())
    }

    fn case_statement(&mut self, arg: &CaseStatement) -> Result<(), ParolError> {
        let exp = self.evaluator.expression(&arg.expression);
        self.evaluated_error(&exp.errors);

        Ok(())
    }

    fn case_condition(&mut self, arg: &CaseCondition) -> Result<(), ParolError> {
        let range_items: Vec<RangeItem> = arg.into();

        for x in range_items {
            let exp = self.evaluator.expression(&x.range.expression);
            self.evaluated_error(&exp.errors);

            // TODO type check

            if !exp.is_known_static() {
                self.errors
                    .push(AnalyzerError::invalid_case_condition_non_elaborative(
                        self.text,
                        &x.range.expression.as_ref().into(),
                    ));
            }

            if let Some(x) = &x.range.range_opt {
                let exp = self.evaluator.expression(&x.expression);
                self.evaluated_error(&exp.errors);

                // TODO type check

                if !exp.is_known_static() {
                    self.errors
                        .push(AnalyzerError::invalid_case_condition_non_elaborative(
                            self.text,
                            &x.expression.as_ref().into(),
                        ));
                }
            }
        }

        Ok(())
    }

    fn switch_condition(&mut self, arg: &SwitchCondition) -> Result<(), ParolError> {
        let expressions: Vec<Expression> = arg.into();

        for x in expressions {
            let exp = self.evaluator.expression(&x);
            self.evaluated_error(&exp.errors);

            // TODO type check
        }

        Ok(())
    }

    fn let_declaration(&mut self, arg: &LetDeclaration) -> Result<(), ParolError> {
        let exp = self.evaluator.expression(&arg.expression);
        self.evaluated_error(&exp.errors);

        // TODO type check

        Ok(())
    }

    fn const_declaration(&mut self, arg: &ConstDeclaration) -> Result<(), ParolError> {
        let exp = self.evaluator.expression(&arg.expression);
        self.evaluated_error(&exp.errors);

        // TODO type check

        Ok(())
    }

    fn assign_declaration(&mut self, arg: &AssignDeclaration) -> Result<(), ParolError> {
        let exp = self.evaluator.expression(&arg.expression);
        self.evaluated_error(&exp.errors);

        // TODO type check

        Ok(())
    }

    fn enum_item(&mut self, arg: &EnumItem) -> Result<(), ParolError> {
        if let Some(x) = &arg.enum_item_opt {
            let exp = self.evaluator.expression(&x.expression);
            self.evaluated_error(&exp.errors);

            // TODO type check
        }
        Ok(())
    }

    fn inst_declaration(&mut self, _arg: &InstDeclaration) -> Result<(), ParolError> {
        match self.point {
            HandlerPoint::Before => {
                self.in_inst_declaration = true;

                // TODO check port connection
            }
            HandlerPoint::After => self.in_inst_declaration = false,
        }
        Ok(())
    }

    fn with_parameter_item(&mut self, arg: &WithParameterItem) -> Result<(), ParolError> {
        let exp = self.evaluator.expression(&arg.expression);
        self.evaluated_error(&exp.errors);

        // TODO type check

        Ok(())
    }

    fn port_type_concrete(&mut self, arg: &PortTypeConcrete) -> Result<(), ParolError> {
        match self.point {
            HandlerPoint::Before => {
                self.port_direction = Some(arg.direction.as_ref().into());

                if let Some(x) = &arg.port_type_concrete_opt0 {
                    let exp = self.evaluator.expression(&x.port_default_value.expression);
                    self.evaluated_error(&exp.errors);

                    // TODO type check
                }
            }
            HandlerPoint::After => self.port_direction = None,
        }
        Ok(())
    }

    fn port_default_value(&mut self, _arg: &PortDefaultValue) -> Result<(), ParolError> {
        match self.point {
            HandlerPoint::Before => {
                self.in_input_port_default_value =
                    matches!(self.port_direction.unwrap(), Direction::Input)
            }
            HandlerPoint::After => self.in_input_port_default_value = false,
        }
        Ok(())
    }

    fn generate_if_declaration(&mut self, arg: &GenerateIfDeclaration) -> Result<(), ParolError> {
        let exp = self.evaluator.expression(&arg.expression);
        self.evaluated_error(&exp.errors);

        // TODO type check

        for x in &arg.generate_if_declaration_list {
            let exp = self.evaluator.expression(&x.expression);
            self.evaluated_error(&exp.errors);

            // TODO type check
        }

        Ok(())
    }

    fn generate_for_declaration(&mut self, arg: &GenerateForDeclaration) -> Result<(), ParolError> {
        let exp = self.evaluator.expression(&arg.range.expression);
        self.evaluated_error(&exp.errors);

        // TODO type check

        if let Some(x) = &arg.range.range_opt {
            let exp = self.evaluator.expression(&x.expression);
            self.evaluated_error(&exp.errors);

            // TODO type check
        }

        if let Some(x) = &arg.generate_for_declaration_opt {
            let exp = self.evaluator.expression(&x.expression);
            self.evaluated_error(&exp.errors);

            // TODO type check
        }

        Ok(())
    }
}
