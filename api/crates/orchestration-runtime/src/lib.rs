extern crate self as orchestration_runtime;

pub mod answer_presentation;
pub mod binding_runtime;
pub mod code_executor_capability;
mod code_runtime;
pub mod compiled_plan;
pub mod compiler;
pub mod execution_engine;
pub mod execution_state;
pub mod node_errors;
pub mod payload_builder;
pub mod preview_executor;

pub fn crate_name() -> &'static str {
    "orchestration-runtime"
}

#[cfg(test)]
pub mod _tests;
