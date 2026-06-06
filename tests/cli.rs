use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

const PDDL_DOMAIN: &str = "examples/pddl/delivery/domain.pddl";
const PDDL_PROBLEM: &str = "examples/pddl/delivery/problem.pddl";
const BAD_PDDL_DOMAIN: &str = "tests/fixtures/pddl/malformed-domain.pddl";
const JIA_CP: &str = "examples/jia/job_shop.jia";
const JIA_LP: &str = "examples/jia/production_lp.jia";
const JIA_RESOURCE: &str = "examples/jia/resource_schedule.jia";
const BAD_JIA: &str = "tests/fixtures/jia/malformed.jia";

#[test]
fn pddl_success_output_parses_full_domain_and_problem() {
    let mut cmd = Command::cargo_bin("jia-parse").unwrap();
    cmd.args(["pddl", "--domain", PDDL_DOMAIN, "--problem", PDDL_PROBLEM])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Domain 'delivery' parsed successfully.",
        ))
        .stdout(predicate::str::contains(
            "Problem 'delivery-1' parsed successfully.",
        ));
}

#[test]
fn pddl_json_output_is_single_object_with_domain_and_problem() {
    let output = Command::cargo_bin("jia-parse")
        .unwrap()
        .args([
            "pddl",
            "--domain",
            PDDL_DOMAIN,
            "--problem",
            PDDL_PROBLEM,
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(json["domain"]["name"], "delivery");
    assert_eq!(json["problem"]["name"], "delivery-1");
    assert_eq!(json["problem"]["domain_name"], "delivery");
    assert!(json.get("domain").is_some());
    assert!(json.get("problem").is_some());
}

#[test]
fn pddl_domain_only_json_omits_problem() {
    let output = Command::cargo_bin("jia-parse")
        .unwrap()
        .args(["pddl", "--domain", PDDL_DOMAIN, "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(json["domain"]["name"], "delivery");
    assert!(json.get("problem").is_none());
}

#[test]
fn pddl_problem_only_json_omits_domain() {
    let output = Command::cargo_bin("jia-parse")
        .unwrap()
        .args(["pddl", "--problem", PDDL_PROBLEM, "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(json.get("domain").is_none());
    assert_eq!(json["problem"]["name"], "delivery-1");
}

#[test]
fn pddl_stats_and_validate_modes_work() {
    Command::cargo_bin("jia-parse")
        .unwrap()
        .args(["pddl", "--domain", PDDL_DOMAIN, "--stats"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Domain: delivery"))
        .stdout(predicate::str::contains("Predicates: 3"))
        .stdout(predicate::str::contains("Actions: 1"));

    Command::cargo_bin("jia-parse")
        .unwrap()
        .args([
            "pddl",
            "--domain",
            PDDL_DOMAIN,
            "--problem",
            PDDL_PROBLEM,
            "--validate",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("OK  {PDDL_DOMAIN}")))
        .stdout(predicate::str::contains(format!("OK  {PDDL_PROBLEM}")));
}

#[test]
fn pddl_problem_stats_report_absent_metric_and_constraints() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(
        &mut file,
        br#"(define (problem noextras)
  (:domain d)
  (:init)
  (:goal (and))
)"#,
    )
    .unwrap();

    Command::cargo_bin("jia-parse")
        .unwrap()
        .args([
            "pddl",
            "--problem",
            file.path().to_str().unwrap(),
            "--stats",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Problem: noextras"))
        .stdout(predicate::str::contains("Has metric: no"))
        .stdout(predicate::str::contains("Has constraints: no"));
}

#[test]
fn pddl_missing_inputs_and_malformed_file_fail() {
    Command::cargo_bin("jia-parse")
        .unwrap()
        .arg("pddl")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No input files specified"));

    Command::cargo_bin("jia-parse")
        .unwrap()
        .args(["pddl", "--domain", BAD_PDDL_DOMAIN])
        .assert()
        .failure()
        .stderr(predicate::str::contains("parse error"));

    let mut bad_problem = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(
        &mut bad_problem,
        b"(define (problem bad) (:domain d) (:goal (",
    )
    .unwrap();
    Command::cargo_bin("jia-parse")
        .unwrap()
        .args(["pddl", "--problem", bad_problem.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("parse error"));

    Command::cargo_bin("jia-parse")
        .unwrap()
        .args([
            "pddl",
            "--domain",
            "tests/fixtures/pddl/missing-domain.pddl",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("reading"));

    Command::cargo_bin("jia-parse")
        .unwrap()
        .args([
            "pddl",
            "--problem",
            "tests/fixtures/pddl/missing-problem.pddl",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("reading"));
}

#[test]
fn jia_success_stats_and_json_work_for_full_files() {
    Command::cargo_bin("jia-parse")
        .unwrap()
        .args(["jia", JIA_CP])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Model 'job_shop' parsed successfully.",
        ));

    Command::cargo_bin("jia-parse")
        .unwrap()
        .args(["jia", JIA_LP, "--stats"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Model: production"))
        .stdout(predicate::str::contains("Type: Some(Lp)"))
        .stdout(predicate::str::contains("Variables: 1"))
        .stdout(predicate::str::contains("Constraints: 2"));

    let output = Command::cargo_bin("jia-parse")
        .unwrap()
        .args(["jia", JIA_CP, "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["name"], "job_shop");
    assert_eq!(json["variables"].as_array().unwrap().len(), 2);
    assert_eq!(json["constraints"].as_array().unwrap().len(), 4);
}

#[test]
fn jia_stats_reports_model_without_objective() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(
        &mut file,
        b"model no_objective\nvariables { Integer: x }\ndomains { x in 0..1 }\n",
    )
    .unwrap();

    Command::cargo_bin("jia-parse")
        .unwrap()
        .args(["jia", file.path().to_str().unwrap(), "--stats"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Has objective: no"));
}

#[test]
fn advanced_examples_are_accepted_by_cli() {
    Command::cargo_bin("jia-parse")
        .unwrap()
        .args(["jia", JIA_RESOURCE, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("resource_schedule"));

    Command::cargo_bin("jia-parse")
        .unwrap()
        .args([
            "pddl",
            "--domain",
            "examples/pddl/advanced/domain.pddl",
            "--problem",
            "examples/pddl/advanced/problem.pddl",
            "--stats",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Problem: advanced-problem"))
        .stdout(predicate::str::contains("Has constraints: yes"));
}

#[test]
fn jia_validate_and_malformed_file_behave_correctly() {
    Command::cargo_bin("jia-parse")
        .unwrap()
        .args(["jia", JIA_CP, "--validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("OK  {JIA_CP}")));

    Command::cargo_bin("jia-parse")
        .unwrap()
        .args(["jia", BAD_JIA])
        .assert()
        .failure()
        .stderr(predicate::str::contains("parse error"));

    Command::cargo_bin("jia-parse")
        .unwrap()
        .args(["jia", "tests/fixtures/jia/missing.jia"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("reading"));
}
