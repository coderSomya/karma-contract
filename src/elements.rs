use serde::{Deserialize, Serialize};
use weil_macros::WeilType;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, WeilType, PartialEq, Eq)]
pub enum Outcome {
    YES,
    NO,
}

#[derive(Debug, Serialize, Deserialize, WeilType)]
pub struct Bet{
    pub side: Outcome,
    pub quantity: u64
}

pub fn lmsr_price(q_yes: u64, q_no: u64, b: f64) -> (f64, f64) {
    let exp_yes = (q_yes as f64 / b).exp();
    let exp_no = (q_no as f64 / b).exp();

    let sum = exp_yes + exp_no;
    (exp_yes / sum, exp_no / sum)
}