use near_contract_standards::non_fungible_token::{Token, TokenId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::*;
use near_sdk::{
    env, ext_contract, json_types::U128, log, near_bindgen, AccountId, PanicOnDefault, Promise,
    PromiseOrValue,
};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CrossContract {
    nft_account: AccountId,
    ft_account: AccountId,
    staked: UnorderedMap<AccountId, Vector<Stake>>,
    unstaked: UnorderedMap<AccountId, Vector<u128>>, //  What is this?
}

#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct Stake {
    timestamp: u64,
    staked_id: TokenId,
    owner_id: AccountId,
}

//
// No documenatation.
//
// Why does Stake have an owner_id field?  From what I can tell stake is mapped owner_id --> Vector<Stake>, so owner is redundant
//
// You cannot iterate over an entire Vector.  You will run out of gas.
//

// What happens if you stake the same token_id twice?

// What is the unstaked collection for?

// One can provide a name, e.g. `ext` to use for generated methods.
#[ext_contract(nftext)]
pub trait NFTCrossContract {
    fn nft_transfer(
        &self,
        sender_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    ) -> (AccountId, Option<HashMap<AccountId, u64>>);
}

#[ext_contract(ftext)]
pub trait FTCrossContract {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

#[near_bindgen]
impl CrossContract {
    // Default Constructor
    #[init]
    pub fn new(ft_account: AccountId, nft_account: AccountId) -> Self {
        Self {
            ft_account,
            nft_account,
            staked: UnorderedMap::new(b"staked".to_vec()),
            unstaked: UnorderedMap::new(b"unstaked".to_vec()),
        }
    }


    pub fn stake(&mut self, token_id: TokenId) /*  -> PromiseOrValue<TokenId>  */
    {
        let caller = env::predecessor_account_id();
        let current_timestamp = env::block_timestamp();
        let mut staked = self
            .staked
            .get(&caller)
            .unwrap_or_else(|| Vector::new(b"new_vec".to_vec()));

        staked.push(&Stake {
            timestamp: current_timestamp,
            staked_id: token_id.clone(),
            owner_id: caller.clone(),
        });
        self.staked.insert(&caller, &staked);
        // ------------------------------------------------------

        match self.unstaked.get(&caller) {
            // What is this doing?
            Some(mut _unstaked) => {
                _unstaked.push(&0); // Why does this add zero?
            }
            None => {
                let new_vec: Vector<u128> = Vector::new(b"new_vec".to_vec());
                self.unstaked.insert(&caller, &new_vec);
            }
        }
        nftext::nft_transfer(
            caller.clone(),
            env::current_account_id(),
            token_id,
            Some(1u64),
            Some(String::from("memo")),
            self.nft_account.clone(), // contract account id
            1,                        // yocto NEAR to attach



            // Where did this number come from? is too low.
            near_sdk::Gas(20000),     // gas to attach 
        );
    }

    /* nftext::nft_transfer_call(
        owner,
        caller,
        ele.staked_id,
        String::from("unstake"),
        String::from("unstake"),
    ); */

    #[result_serializer(borsh)]
    pub fn unstake(&mut self) {
        let owner = env::current_account_id();
        let caller = env::predecessor_account_id();
        self.staked.get(&caller).map_or_else(
            || log!("You didn't stake any token at all."),
            |_staked| {
                _staked.iter().for_each(|ele| {
                    if ele.owner_id == caller {
                        nftext::nft_transfer(
                            owner.clone(),
                            caller.clone(),
                            ele.staked_id,
                            Some(1u64),
                            Some(String::from("memo")),
                            self.nft_account.clone(), // contract account id
                            0,                        // yocto NEAR to attach

                            // You cannot attached prepaid_gas since you have already burned some.  
                            // Furthermore, this code currently creates multiple promises so even if you 
                            // had attached gas left over, there wouldn't be for the next call.
                            env::prepaid_gas(),       // gas to attach
                        );
                    }
                })
            },
        );
    }

    /* nftext::nft_transfer_call(
        owner,
        caller,
        ele.staked_id,
        String::from("unstake"),
        String::from("unstake"),
    ); */
    
    pub fn claim(&mut self, token_id: TokenId) {  // token_id is never used
        self.callers_stake().map_or_else(
            || log!("You are not valid claimer."),
            |staked| {
                staked.iter().for_each(|ele| { // ele not used
                    ftext::ft_transfer(
                        env::predecessor_account_id(),
                        1_000_000_000_000_000_000u128.into(),
                        Some("claim".into()),
                        self.nft_account.clone(), // contract account id
                        1,                        // yocto NEAR to attach
                        // See comment starting on line 138
                        env::prepaid_gas(),       // gas to attach
                    );
                })
            },
        );
    }

    fn callers_stake(&mut self) -> Option<Vector<Stake>> {
        self.staked.get(&env::predecessor_account_id())
    }

    // #[result_serializer(borsh)]
    pub fn get_claimable(&self, token_id: TokenId) -> u128 {
        let caller = env::predecessor_account_id();
        let current_timestamp = env::block_timestamp();
        let mut staked_timestamp = 0;
        self.staked.get(&caller).map_or_else(
            || {
                log!("{}", "Cannot get claimable amount");
                0
            },
            |_staked| {
                _staked.iter().for_each(|ele| {
                    if ele.staked_id == token_id {
                        staked_timestamp = ele.timestamp;
                    }
                });
                // if no token found this would make staked_timestamp 0
                (current_timestamp - staked_timestamp).into()
            },
        )
    }

    pub fn transfer_money(&mut self, account_id: AccountId, amount: u64) {
        Promise::new(account_id).transfer(amount as u128);
    }
}
