mod alignment;
mod app;
mod runner;
mod seq;
mod ui;
mod vec_f64_aux;
pub mod errors;

use crate::errors::TermalError;

pub fn run() -> Result<(), TermalError> {
    runner::run()
}
