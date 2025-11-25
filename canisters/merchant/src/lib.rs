use candid::{CandidType, Deserialize, Principal};
use ic_cdk::query;

#[derive(CandidType, Deserialize)]
struct PaymentInfo {
    scheme: String,
    token: TokenInfo,
    amount: u64,
    recipient: Principal,
    facilitator: Principal,
}

#[derive(CandidType, Deserialize)]
struct TokenInfo {
    chain: String,
    ledger_id: String,
    symbol: String,
}

#[derive(CandidType, Deserialize)]
struct ErrorResponse {
    code: String,
    message: String,
    x402: PaymentInfo,
}

// Premium data that costs money
static PREMIUM_DATA: &str = r#"
{
  "monopoly_status": "live",
  "agent_count": 402,
  "total_transactions": 1500000,
  "ckbtc_volume": "75.5 BTC",
  "active_merchants": 127,
  "network_health": "optimal"
}
"#;

// The 402 challenge endpoint - always returns payment required for demo
#[query]
fn get_premium_data() -> ErrorResponse {
    let canister_id = ic_cdk::caller(); // Using caller instead of canister_self for MVP

    let payment_info = PaymentInfo {
        scheme: "exact".to_string(),
        token: TokenInfo {
            chain: "ICP".to_string(),
            ledger_id: "mxzaz-hqmqe-cnarv-dqu6p-osn6o-42zio-q4u4l-q4k2l-q4n2l-q4h2l-76z".to_string(),
            symbol: "ckBTC".to_string(),
        },
        amount: 1000, // 1000 satoshis = 0.00001 BTC
        recipient: canister_id,
        facilitator: Principal::from_text("ogkpr-lyaaa-aaaap-an5fq-cai").unwrap(), // Anda facilitator
    };

    ErrorResponse {
        code: "402".to_string(),
        message: "Payment Required".to_string(),
        x402: payment_info,
    }
}

// This endpoint requires valid payment proof to access
#[query]
fn access_premium_data(payment_proof: String) -> String {
    // For MVP, we'll just accept any non-empty proof
    // In production, this would verify the payment with the facilitator
    if payment_proof.is_empty() {
        return "Invalid payment proof".to_string();
    }

    // TODO: Verify payment with facilitator canister
    // ic_cdk::call(facilitator, "verify_payment", (payment_proof,))

    PREMIUM_DATA.to_string()
}

// Health check for grant demo
#[query]
fn health() -> String {
    "Merchant canister is running".to_string()
}

#[query]
fn canister_info() -> (Principal, String) {
    (ic_cdk::caller(), "x402 Merchant Canister v0.1".to_string())
}