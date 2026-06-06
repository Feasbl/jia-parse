const PDDL_DOMAIN: &str = include_str!("../examples/pddl/delivery/domain.pddl");
const PDDL_PROBLEM: &str = include_str!("../examples/pddl/delivery/problem.pddl");
const JIA_CP: &str = include_str!("../examples/jia/job_shop.jia");
const JIA_LP: &str = include_str!("../examples/jia/production_lp.jia");

#[test]
fn parses_full_pddl_domain_fixture_with_public_api() {
    let domain = jia_parse::pddl::parse_domain_str(PDDL_DOMAIN).unwrap();

    assert_eq!(domain.name, "delivery");
    assert_eq!(domain.predicates.len(), 3);
    assert_eq!(domain.actions.len(), 1);
    assert_eq!(domain.actions[0].name, "deliver");
}

#[test]
fn parses_full_pddl_problem_fixture_with_public_api() {
    let problem = jia_parse::pddl::parse_problem_str(PDDL_PROBLEM).unwrap();

    assert_eq!(problem.name, "delivery-1");
    assert_eq!(problem.domain_name, "delivery");
    assert_eq!(problem.init.len(), 2);
}

#[test]
fn parses_full_jia_cp_fixture_with_public_api() {
    let model = jia_parse::jia::parse_model_str(JIA_CP).unwrap();

    assert_eq!(model.name, "job_shop");
    assert_eq!(model.variables.len(), 2);
    assert_eq!(model.domains.len(), 6);
    assert_eq!(model.constraints.len(), 4);
    assert!(model.objective.is_some());
}

#[test]
fn parses_full_jia_lp_fixture_with_public_api() {
    let model = jia_parse::jia::parse_model_str(JIA_LP).unwrap();

    assert_eq!(model.name, "production");
    assert_eq!(model.model_type, Some(jia_parse::jia::ast::ModelType::Lp));
    assert_eq!(model.variables.len(), 1);
    assert_eq!(model.constraints.len(), 2);
    assert!(model.objective.is_some());
}
