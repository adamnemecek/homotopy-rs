use std::{fs::read, path::PathBuf};

pub use history::Proof;
use homotopy_core::common::Mode;
pub use homotopy_model::model::{history, migration, proof, serialize};
// CLI option parsing
use structopt::StructOpt;

// Struct for CLI options
#[derive(Debug, StructOpt)]
#[structopt(
    name = "homotopy-cli",
    about = "Handy tool to debug proofs! Made by yours truly."
)]
struct Opt {
    #[structopt(short, long, parse(from_os_str))]
    input_hom: Option<PathBuf>,
}

fn import_hom(path: &PathBuf) -> Option<Proof> {
    let data = read(path).ok()?;
    let (signature, workspace) = match serialize::deserialize(&data) {
        Some(res) => res,
        None => migration::deserialize(&data)?,
    };

    for g in signature.iter() {
        g.diagram.check(Mode::Deep).ok()?;
    }
    if let Some(w) = workspace.as_ref() {
        w.diagram.check(Mode::Deep).ok()?;
    }
    let mut proof: Proof = Default::default();
    proof.signature = signature;
    proof.workspace = workspace;
    Some(proof)
}

fn main() {
    // Give me options.
    let opt = Opt::from_args();
    if let Some(path) = opt.input_hom {
        let _proof = import_hom(&path).expect("Could not import .hom file.");
    }
}
