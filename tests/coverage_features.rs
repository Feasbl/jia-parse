use jia_parse::ast::{
    AssignOp, Condition, DurationConstraint, Effect, InitElement, MetricSpec, NumericExpr,
    Optimization, Requirement,
};
use jia_parse::jia::ast::{ArithOp, CmpOp, Domain, DomainStmt, ModelType, VarType};
use jia_parse::jia::lexer::tokenize;

const ADVANCED_DOMAIN: &str = include_str!("../examples/pddl/advanced/domain.pddl");
const ADVANCED_PROBLEM: &str = include_str!("../examples/pddl/advanced/problem.pddl");
const RESOURCE_SCHEDULE: &str = include_str!("../examples/jia/resource_schedule.jia");

#[test]
fn parses_advanced_pddl_domain_features() {
    let domain = jia_parse::pddl::parse_domain_str(ADVANCED_DOMAIN).unwrap();

    assert_eq!(domain.name, "advanced");
    assert!(domain.requirements.contains(&Requirement::Adl));
    assert!(domain
        .requirements
        .contains(&Requirement::DurationInequalities));
    assert_eq!(domain.constants.len(), 1);
    assert_eq!(domain.functions.len(), 3);
    assert!(domain.functions.iter().any(
        |function| function.name == "fuel" && function.return_type.as_deref() == Some("number")
    ));
    assert_eq!(domain.derived_predicates.len(), 1);
    assert_eq!(domain.actions.len(), 1);
    assert_eq!(domain.durative_actions.len(), 1);

    let load = &domain.actions[0];
    assert_eq!(load.name, "load");
    assert_eq!(load.parameters.len(), 4);
    assert!(matches!(load.precondition, Some(Condition::And(_))));

    let Some(Effect::And(effects)) = &load.effect else {
        panic!("expected load effect conjunction");
    };
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::NumericAssign {
            op: AssignOp::ScaleDown,
            ..
        }
    )));
    assert!(effects
        .iter()
        .any(|effect| matches!(effect, Effect::Forall { .. })));

    let drive = &domain.durative_actions[0];
    assert_eq!(drive.name, "drive");
    assert!(matches!(drive.duration, DurationConstraint::And(_)));
    assert!(matches!(drive.condition, Some(Condition::And(_))));
    assert!(matches!(drive.effect, Some(Effect::And(_))));
}

#[test]
fn parses_advanced_pddl_problem_features() {
    let problem = jia_parse::pddl::parse_problem_str(ADVANCED_PROBLEM).unwrap();

    assert_eq!(problem.name, "advanced-problem");
    assert_eq!(problem.domain_name, "advanced");
    assert_eq!(problem.requirements.len(), 4);
    assert_eq!(problem.objects.len(), 3);
    assert_eq!(problem.init.len(), 7);
    assert!(problem
        .init
        .iter()
        .any(|init| matches!(init, InitElement::NotPredicate(_))));
    assert!(problem
        .init
        .iter()
        .any(|init| matches!(init, InitElement::NumericAssignment(_, _))));
    assert!(problem
        .init
        .iter()
        .any(|init| matches!(init, InitElement::At(_, _))));
    assert!(matches!(problem.goal, Condition::And(_)));
    assert!(matches!(problem.constraints, Some(Condition::And(_))));

    let Some(MetricSpec { optimization, expr }) = &problem.metric else {
        panic!("expected metric");
    };
    assert_eq!(*optimization, Optimization::Maximize);
    assert!(matches!(expr, NumericExpr::BinaryOp { .. }));
}

#[test]
fn parses_jia_resource_schedule_example_and_analysis_helpers() {
    let model = jia_parse::jia::parse_model_str(RESOURCE_SCHEDULE).unwrap();

    assert_eq!(model.name, "resource_schedule");
    assert_eq!(model.model_type, Some(ModelType::Cp));
    assert_eq!(model.variables.len(), 5);
    assert_eq!(model.constraints.len(), 4);
    assert!(model.objective.is_some());
    assert!(model.domains.iter().any(|stmt| matches!(
        stmt,
        DomainStmt::IntervalStart {
            domain: Domain::RealRange { min, max },
            ..
        } if min.is_infinite() && min.is_sign_negative() && *max == 10.5
    )));
    assert!(model.domains.iter().any(|stmt| matches!(
        stmt,
        DomainStmt::IntegerDomain {
            name,
            domain: Domain::RealRange { min, max },
        } if name == "slack" && min.is_infinite() && max.is_infinite()
    )));
    assert!(model.domains.iter().any(|stmt| matches!(
        stmt,
        DomainStmt::IntervalDuration {
            domain: Domain::RealFixed(value),
            ..
        } if *value == 2.5
    )));

    let tokens = tokenize(RESOURCE_SCHEDULE).unwrap();
    let table = jia_parse::jia::analysis::build_symbol_table(&model, &tokens);
    let task_a = table.symbols.get("task_a").unwrap();
    let summary = task_a.domain_summary.as_ref().unwrap();
    assert!(summary.contains("duration = 2.5"));
    assert!(summary.contains("start in -inf..10.5"));
    assert!(summary.contains("demand(machine) = 2"));
    assert!(!task_a.ref_spans.is_empty());

    assert_eq!(VarType::Interval.to_string(), "Interval");
    assert_eq!(VarType::Integer.to_string(), "Integer");
    assert_eq!(VarType::Real.to_string(), "Real");
    assert_eq!(VarType::SetInterval.to_string(), "Set[Interval]");
    assert_eq!(VarType::SetInteger.to_string(), "Set[Integer]");
    assert_eq!(CmpOp::Lt.to_string(), "<");
    assert_eq!(CmpOp::Le.to_string(), "<=");
    assert_eq!(CmpOp::Gt.to_string(), ">");
    assert_eq!(CmpOp::Ge.to_string(), ">=");
    assert_eq!(CmpOp::Eq.to_string(), "==");
    assert_eq!(CmpOp::Ne.to_string(), "!=");
    assert_eq!(ArithOp::Add.to_string(), "+");
    assert_eq!(ArithOp::Sub.to_string(), "-");
    assert_eq!(ArithOp::Mul.to_string(), "*");
    assert_eq!(ArithOp::Div.to_string(), "/");
}
