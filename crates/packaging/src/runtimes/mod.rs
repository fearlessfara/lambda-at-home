use std::path::{PathBuf};
use lambda_models::Function;

mod node;
mod python;
mod rust_rt;

pub fn dockerfile_for(function: &Function) -> String {
    match function.runtime.as_str() {
        "nodejs18.x" | "nodejs22.x" => node::dockerfile(function),
        "python3.11" => python::dockerfile(function),
        "rust" => rust_rt::dockerfile(function),
        _ => unreachable!("unsupported runtime checked earlier"),
    }
}

/// Optional bootstrap source path relative to repo root that should be copied into build context.
/// Returns (relative_path_in_repo, dest_filename) where dest is placed in /var/runtime in the image.
pub fn bootstrap_source(function: &Function) -> Option<(PathBuf, &'static str)> {
    match function.runtime.as_str() {
        "nodejs18.x" => Some((PathBuf::from("runtimes/nodejs18/bootstrap.js"), "bootstrap.js")),
        "nodejs22.x" => Some((PathBuf::from("runtimes/nodejs22/bootstrap.js"), "bootstrap.js")),
        "python3.11" => Some((PathBuf::from("runtimes/python311/bootstrap.py"), "bootstrap.py")),
        _ => None,
    }
}

