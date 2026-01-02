
use serde::{Deserialize, Serialize};
use weil_macros::{constructor, mutate, query, smart_contract, WeilType};
use weil_rs::{collections::{WeilId, WeilIdGenerator, map::WeilMap, vec::WeilVec}, runtime::Runtime};
use weil_rs::webserver::WebServer;

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

    // webserver specific functions
    fn start_file_upload(&mut self, path: String, total_chunks: u32) -> Result<(), String>;
    fn add_path_content(
        &mut self,
        path: String,
        chunk: Vec<u8>,
        index: u32,
    ) -> Result<(), String>;
    fn finish_upload(&mut self, path: String, size_bytes: u32) -> Result<(), String>;
    fn total_chunks(&self, path: String) -> Result<u32, String>;
    fn http_content(
        &self,
        path: String,
        index: u32,
        method: String,
    ) -> (u16, std::collections::HashMap<String, String>, Vec<u8>);
    fn size_bytes(&self, path: String) -> Result<u32, String>;
    fn get_chunk_size(&self) -> u32;
}

#[derive(Serialize, Deserialize, WeilType)]
pub struct KarmaContractState {
    // define your contract state here!
    counter: u64,
    users: WeilMap<String, User>, // user_id -> user
    markets: WeilMap<String, Market>, // market_id -> market
    user_ids: WeilVec<String>,
    market_ids: WeilVec<String>,

    server: WebServer,
    weil_id_generator: WeilIdGenerator
}

#[smart_contract]
impl Karma for KarmaContractState {
    #[constructor]
    fn new() -> Result<Self, String>
    where
        Self: Sized,
    {
        Ok(Self{
            counter: 0,
            users: WeilMap::new(WeilId(1)),
            markets: WeilMap::new(WeilId(2)),
            user_ids: WeilVec::new(WeilId(3)),
            market_ids: WeilVec::new(WeilId(4)),
            server: WebServer::new(WeilId(5), None),
            weil_id_generator: WeilIdGenerator::new(WeilId(6))
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
        let counter = self.counter;
        let market = Market::new(counter.to_string(), question, liquidity);
        let market_id = market.id.clone();
        self.counter+=1;

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
        // TODO: implement a way for users to 
        // convert their tokens on weilchain to 
        // balance in karma
    }

    #[query]
    async fn get_cost(&self, market_id: String) -> (f64, f64) {
        let Some(market) = self.markets.get(&market_id) else {
            return (0.0, 0.0);
        };
        let (cost_per_yes, cost_per_no) = lmsr_price(market.num_yes, market.num_no, market.liquidity);
        (cost_per_yes, cost_per_no)
    }

    #[mutate]
    fn start_file_upload(&mut self, path: String, total_chunks: u32) -> Result<(), String> {
        self.server.start_file_upload(self.weil_id_generator.next_id(), path, total_chunks)
    }

    #[query]
    fn total_chunks(&self, path: String) -> Result<u32, String> {
        self.server.total_chunks(path)
    }

    #[mutate]
    fn add_path_content(
        &mut self,
        path: String,
        chunk: Vec<u8>,
        index: u32,
    ) -> Result<(), String> {
        self.server.add_path_content(path, chunk, index)
    }

    #[mutate]
    fn finish_upload(&mut self, path: String, size_bytes: u32) -> Result<(), String> {
        self.server.finish_upload(path, size_bytes)
    }

    #[query]
    fn http_content(
        &self,
        path: String,
        index: u32,
        method: String,
    ) -> (u16, std::collections::HashMap<String, String>, Vec<u8>) {
        self.server.http_content(path, index, method)
    }

    #[query]
    fn size_bytes(&self, path: String) -> Result<u32, String> {
        self.server.size_bytes(path)
    }

    #[query]
    fn get_chunk_size(&self) -> u32 {
        self.server.get_chunk_size()
    }
}

