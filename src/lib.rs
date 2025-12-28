pub mod app;
mod runner;
pub mod seq;
mod vec_f64_aux;
pub mod alignment;
pub mod errors;
pub mod ui;

use crate::errors::TermalError;

pub fn run() -> Result<(), TermalError> {
    runner::run()
}
