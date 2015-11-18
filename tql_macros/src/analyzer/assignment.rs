/*
 * Copyright (C) 2015  Boucher, Antoni <bouanto@zoho.com>
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

/// Argument to assignment converter.

use syntax::ast::BinOp_;
use syntax::ast::Expr_::{ExprAssign, ExprAssignOp};
use syntax::codemap::Spanned;

use ast::{Assignment, AssignementOperator, Expression, FilterValue};
use error::{SqlError, SqlResult, res};
use plugin::number_literal;
use state::SqlTable;
use super::{check_field, check_field_type, path_expr_to_identifier};

/// Analyze the types of the `Assignment`s.
pub fn analyze_assignments_types(assignments: &[Assignment], table_name: &str, errors: &mut Vec<SqlError>) {
    for assignment in assignments {
        check_field_type(table_name, &FilterValue::Identifier(assignment.identifier.clone()), &assignment.value, errors);
    }
}

/// Convert an `Expression` to an `Assignment`.
pub fn argument_to_assignment(arg: &Expression, table: &SqlTable) -> SqlResult<Assignment> {
    fn assign_values(assignment: &mut Assignment, expr1: &Expression, expr2: &Expression, table: &SqlTable, errors: &mut Vec<SqlError>) {
        assignment.value = expr2.clone();
        if let Some(identifier) = path_expr_to_identifier(expr1, errors) {
            assignment.identifier = identifier;
            check_field(&assignment.identifier, expr1.span, table, errors);
        }
    }

    let mut errors = vec![];
    let mut assignment = Assignment {
        identifier: "".to_owned(),
        operator: Spanned {
            node: AssignementOperator::Equal,
            span: arg.span,
        },
        value: number_literal(0),
    };
    match arg.node {
        ExprAssign(ref expr1, ref expr2) => {
            assign_values(&mut assignment, expr1, expr2, table, &mut errors);
        },
        ExprAssignOp(ref binop, ref expr1, ref expr2) => {
            assignment.operator = Spanned {
                node: binop_to_assignment_operator(binop.node),
                span: binop.span,
            };
            assign_values(&mut assignment, expr1, expr2, table, &mut errors);
        },
        _ => {
            errors.push(SqlError::new(
                "Expected assignment", // TODO: improve this message.
                arg.span,
            ));
        },
    }
    res(assignment, errors)
}

/// Convert a `BinOp_` to an SQL `AssignmentOperator`.
fn binop_to_assignment_operator(binop: BinOp_) -> AssignementOperator {
    match binop {
        BinOp_::BiAdd => AssignementOperator::Add,
        BinOp_::BiSub => AssignementOperator::Sub,
        BinOp_::BiMul => AssignementOperator::Mul,
        BinOp_::BiDiv => AssignementOperator::Divide,
        BinOp_::BiRem => AssignementOperator::Modulo,
        BinOp_::BiAnd => unreachable!(),
        BinOp_::BiOr => unreachable!(),
        BinOp_::BiBitXor => unreachable!(),
        BinOp_::BiBitAnd => unreachable!(),
        BinOp_::BiBitOr => unreachable!(),
        BinOp_::BiShl => unreachable!(),
        BinOp_::BiShr => unreachable!(),
        BinOp_::BiEq => AssignementOperator::Equal,
        BinOp_::BiLt => unreachable!(),
        BinOp_::BiLe => unreachable!(),
        BinOp_::BiNe => unreachable!(),
        BinOp_::BiGe => unreachable!(),
        BinOp_::BiGt => unreachable!(),
    }
}
