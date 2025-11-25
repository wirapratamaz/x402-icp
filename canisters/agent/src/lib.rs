use candid::{CandidType, Deserialize, Principal};
use ic_cdk::{query, update, call};
use std::cell::RefCell;

#[derive(CandidType, Deserialize)]
struct PaymentRequest {
    to: Principal,
    amount: u64,
    memo: Option<Vec<u8>>,
}

#[derive(CandidType, Deserialize)]
struct AgentConfig {
    owner: Principal,
    max_payment: u64,
}

thread_local! {
    static AGENT_CONFIG: RefCell<Option<AgentConfig>> = RefCell::new(None);
}

// Initialize the agent with owner configuration
#[update]
fn init_agent(owner: Principal, max_payment: u64) {
    AGENT_CONFIG.with(|c| {
        *c.borrow_mut() = Some(AgentConfig { owner, max_payment });
    });
}

#[derive(CandidType, Deserialize)]
struct FetchResult {
    success: bool,
    data: Option<String>,
    payment_required: Option<PaymentInfo>,
    error: Option<String>,
}

#[derive(CandidType, Deserialize, Clone)]
struct PaymentInfo {
    scheme: String,
    token: TokenInfo,
    amount: u64,
    recipient: Principal,
    facilitator: Principal,
}

#[derive(CandidType, Deserialize, Clone)]
struct TokenInfo {
    chain: String,
    ledger_id: String,
    symbol: String,
}

#[derive(CandidType, Deserialize)]
struct MerchantErrorResponse {
    code: String,
    message: String,
    x402: PaymentInfo,
}

#[derive(CandidType, Deserialize)]
struct PaymentProof {
    payment_id: String,
    nonce: u64,
    payer: Principal,
    amount: u64,
    token_ledger: Principal,
    facilitator_signature: Vec<u8>,
    expiry: u64,
    resource_id: String,
}

#[derive(CandidType)]
struct PaymentVerification {
    success: bool,
    block_index: u64,
    transaction_hash: String,
    error: Option<String>,
}

// Agent attempts to fetch premium data
#[update]
async fn fetch_premium_data(merchant_canister: Principal) -> FetchResult {
    let max_payment = AGENT_CONFIG.with(|c| {
        c.borrow()
            .as_ref()
            .map(|cfg| cfg.max_payment)
            .expect("Agent not initialized")
    });
    let merchant_resp = match call::<(), (MerchantErrorResponse,)>(merchant_canister, "get_premium_data", ()).await {
        Ok((resp,)) => resp,
        Err((_code, _msg)) => {
            return FetchResult {
                success: false,
                data: None,
                payment_required: None,
                error: Some("Merchant call failed".to_string()),
            }
        }
    };

    if merchant_resp.code == "402" {
        let pi = merchant_resp.x402.clone();
        if pi.amount > max_payment {
            return FetchResult {
                success: false,
                data: None,
                payment_required: Some(pi),
                error: Some("Payment amount exceeds configured maximum".to_string()),
            };
        }
        return FetchResult {
            success: false,
            data: None,
            payment_required: Some(pi),
            error: None,
        };
    }

    let data = match call::<(String,), (String,)>(merchant_canister, "access_premium_data", ("proof".to_string(),)).await {
        Ok((s,)) => s,
        Err((_code, _msg)) => {
            return FetchResult {
                success: false,
                data: None,
                payment_required: None,
                error: Some("Access call failed".to_string()),
            };
        }
    };

    FetchResult {
        success: true,
        data: Some(data),
        payment_required: None,
        error: None,
    }
}

// Get agent status
#[query]
fn agent_status() -> String {
    AGENT_CONFIG.with(|c| {
        match c.borrow().as_ref() {
            Some(config) => format!(
                "Agent initialized. Owner: {}. Max payment: {} sats",
                config.owner, config.max_payment
            ),
            None => "Agent not initialized".to_string(),
        }
    })
}

#[query]
fn get_balance(_token_ledger: Principal) -> u64 {
    // Mock balance for demo
    1000000 // 0.01 BTC in satoshis
}
