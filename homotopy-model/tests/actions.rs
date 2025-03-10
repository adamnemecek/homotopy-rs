pub use homotopy_model::{history::Proof, proof::Action, serialize};

fn actions_test_helper(json: &str) -> Proof {
    let (_safe, actions): (bool, Vec<Action>) = serde_json::from_str(json).unwrap();
    let mut proof = Proof::default();

    for a in &actions {
        proof
            .update(a)
            .expect("Actions should replay without errors.");
    }

    proof
}

#[test]
fn construct_associator() {
    let action_dump = include_str!("examples/associator.json");

    let _proof = actions_test_helper(action_dump);
}

#[test]
fn construct_braiding_half_bubbles() {
    let action_dump = include_str!("examples/braiding_half_bubbles.json");

    let _proof = actions_test_helper(action_dump);
}

#[test]
fn construct_symmetric_monoidal() {
    let action_dump = include_str!("examples/symmetric_monoidal_abelianize.json");

    let _proof = actions_test_helper(action_dump);
}

#[test]
fn construct_adjoint_equivalence() {
    let action_dump = include_str!("examples/equivalence_to_adjoint_equivalence.json");

    let _proof = actions_test_helper(action_dump);
}

#[test]
fn merge_contract_r1() {
    let action_dump = include_str!("examples/merge_contract_RI.json");

    let _proof = actions_test_helper(action_dump);
}
