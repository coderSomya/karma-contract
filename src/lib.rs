
use serde::{Deserialize, Serialize};
use weil_macros::{constructor, mutate, query, smart_contract, WeilType};
use weil_rs::{collections::{WeilId, WeilIdGenerator, map::WeilMap, vec::WeilVec}, runtime::Runtime};

mod market;
use market::Market;

mod elements;
use elements::Outcome;

mod user;
use user::User;

use crate::elements::{Bet, lmsr_price};

trait Karma {
    fn new() -> Result<Self, String>
    where
        Self: Sized;
    async fn register_user(&mut self, bio: String);
    async fn add_market(&mut self, question: String, liquidity: f64);
    async fn get_users(&self) -> Vec<User>;
    async fn get_markets(&self) -> Vec<Market>;
    async fn get_user(&self, id: String) -> Option<User>;
    async fn get_market(&self, id: String) -> Option<Market>;
    async fn bet(&mut self, market_id: String, side: Outcome, quantity: u64) -> Result<(), String>;
    async fn resolve(&mut self, market_id: String) -> Result<(), String>;
    async fn deposit(&mut self, amount: f64);
    async fn get_cost(&self, market_id: String) -> (f64, f64);
}

#[derive(Serialize, Deserialize, WeilType)]
pub struct KarmaContractState {
    // define your contract state here!
    base_id: WeilIdGenerator,
    users: WeilMap<String, User>, // user_id -> user
    markets: WeilMap<String, Market>, // market_id -> market
    user_ids: WeilVec<String>,
    market_ids: WeilVec<String>
}

#[smart_contract]
impl Karma for KarmaContractState {
    #[constructor]
    fn new() -> Result<Self, String>
    where
        Self: Sized,
    {
        let mut base_id = WeilIdGenerator::new(WeilId(1));
        let id_1 = base_id.next_id();
        let id_2 = base_id.next_id();
        let id_3 = base_id.next_id();
        let id_4 = base_id.next_id();

        Ok(Self{
            base_id,
            users: WeilMap::new(id_1),
            markets: WeilMap::new(id_2),
            user_ids: WeilVec::new(id_3),
            market_ids: WeilVec::new(id_4)
        })
    }


    #[mutate]
    async fn register_user(&mut self, bio: String) {
        let user = User::new(bio);
        let user_id = user.id.clone();
        
        self.users.insert(user_id.clone(), user);
        self.user_ids.push(user_id);
    }

    #[mutate]
    async fn add_market(&mut self, question: String, liquidity: f64) {
        let market = Market::new(question, liquidity);
        let market_id = market.id.clone();

        self.markets.insert(market_id.clone(), market);
        self.market_ids.push(market_id)
    }

    #[query]
    async fn get_users(&self) -> Vec<User> {
        let mut all_users = Vec::new();

        for user_id in self.user_ids.iter(){
            all_users.push(self.users.get(&user_id).unwrap())
        };
        all_users
    }

    #[query]
    async fn get_markets(&self) -> Vec<Market> {
        let mut all_markets = Vec::new();

        for market_id in self.market_ids.iter(){
            all_markets.push(self.markets.get(&market_id).unwrap())
        };
        all_markets
    }

    #[query]
    async fn get_user(&self, id: String) -> Option<User> {
        self.users.get(&id)
    }

    #[query]
    async fn get_market(&self, id: String) -> Option<Market> {
        self.markets.get(&id)
    }

    #[mutate]
    async fn bet(&mut self, market_id: String, side: Outcome, quantity: u64) -> Result<(), String> {
        let Some(mut market) = self.markets.get(&market_id) else{
            return Err("No such market".into());
        };
        if market.is_resolved(){
            return Err("Market already resolved".into());
        }

        let user_id = Runtime::sender();
        let Some(mut user) = self.users.get(&user_id) else {
            return Err("User is not registered".into());
        };

        if market.has_already_voted(user_id.clone()){
            return Err("You have already voted for this market".into());
        }

        let (cost_per_yes, cost_per_no) = lmsr_price(market.num_yes, market.num_no, market.liquidity);

        let price = match side {
            Outcome::YES => {
                cost_per_yes*(quantity as f64)
            },
            Outcome::NO => {
                cost_per_no*(quantity as f64)
            }
        };

        if price > user.balance(){
            return Err("Insufficient balance".into());
        }
        user.add_market(market_id.clone());
        user.withdraw(price);
        let bet = Bet{
            side,
            quantity
        };
        market.add_bet(bet);
        
        self.users.insert(user_id, user);
        self.markets.insert(market_id, market);
        Ok(())
    }

    #[mutate]
    async fn resolve(&mut self, market_id: String) -> Result<(), String>{
        let Some(mut market) = self.markets.get(&market_id) else{
            return Err("No such market".into());
        };

        market.resolve()?;
        // SAFETY: we just added the outcome
        let outcome = market.outcome.unwrap();
        // distribute the money to all winners
        for (voter, bet) in market.voters.iter(){
            if bet.side == outcome {
                // SAFETY: if voted, then the user must be present
                let mut user = self.users.get(voter).unwrap();
                let user_id = user.id.clone();
                let amount = bet.quantity as f64; // 1 share = 1 dollar
                user.deposit(amount);
                self.users.insert(user_id, user);
            }
        }

        self.markets.insert(market_id, market);
        Ok(())
    }

    #[mutate]
    async fn deposit(&mut self, amount: f64) {
        let Some(mut user) = self.users.get(&Runtime::sender()) else {
            return;
        };

        user.deposit(amount);
        self.users.insert(Runtime::sender(), user);
    }

    #[query]
    async fn get_cost(&self, market_id: String) -> (f64, f64) {
        let Some(market) = self.markets.get(&market_id) else {
            return (0.0, 0.0);
        };
        let (cost_per_yes, cost_per_no) = lmsr_price(market.num_yes, market.num_no, market.liquidity);
        (cost_per_yes, cost_per_no)
    }
}

