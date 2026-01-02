use serde::{Deserialize, Serialize};
use weil_macros::WeilType;
use weil_rs::runtime::Runtime;

#[derive(Debug, Serialize, Deserialize, WeilType)]
pub struct User {
    pub id: String,
    bio: String,
    balance: f64,
    history: Vec<String>,
}


impl User{
    pub fn new(bio: String) -> Self{
        let user = User{
            id: Runtime::sender(),
            bio: bio,
            balance: 100.0,
            history: Vec::new()
        };

        user
    }

    pub fn balance(&self) -> f64{
        self.balance
    }

    pub fn deposit(&mut self, amount: f64){
        self.balance+=amount
    }

    pub fn withdraw(&mut self, amount: f64){
        self.balance-=amount
    }

    pub fn add_market(&mut self, market_id: String){
        self.history.push(market_id)
    }
}