use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use weil_macros::WeilType;
use weil_rs::runtime::Runtime;
use crate::elements::{Bet, Outcome};

#[derive(Debug, Serialize, Deserialize, WeilType)]
pub struct Market {
    creator: String,
    pub id: String,
    question: String,
    pub num_yes: u64,
    pub num_no: u64,
    pub liquidity: f64,
    resolved: bool,
    pub outcome: Option<Outcome>,
    pub voters: BTreeMap<String, Bet>,
}

impl Market{
    pub fn new(counter: String, question: String, liquidity: f64) -> Self{
        let market_id = format!("market_{}", counter);
        
        let market = Market{
            creator: Runtime::sender(),
            id: market_id,
            question: question,
            num_yes: 0,
            num_no: 0,
            liquidity: liquidity,
            resolved: false,
            outcome: None,
            voters: BTreeMap::new(),
        };

        market
    }

    pub fn add_bet(&mut self, bet: Bet){
        let better = Runtime::sender();

        match bet.side{
            Outcome::YES => {
                self.num_yes+=1;
            },
            Outcome::NO  => {
                self.num_no+=1;
            }
        };

        self.voters.insert(better, bet);
    }

    pub fn resolve(&mut self) -> Result<(), String>{
        if self.creator != Runtime::sender(){
            return Err("You are not the creator".into());
        }
        self.resolved = true;
        if self.num_no > self.num_yes {
            self.outcome = Some(Outcome::NO)
        } else{
            self.outcome = Some(Outcome::YES)
        }
        Ok(())
    }

    pub fn is_resolved(&self) -> bool {
        self.resolved
    }

    pub fn has_already_voted(&self, user_id: String) -> bool {
        self.voters.get(&user_id).is_some()
    }
}