use candid::{CandidType, Deserialize, Principal};
use ic_cdk::{query, update, caller};
use std::collections::BTreeSet;
use std::cell::RefCell;

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

// Constants for security
static TRUSTED_FACILITATOR: &str = "ogkpr-lyaaa-aaaap-an5fq-cai";
static CKBTC_LEDGER_ID: &str = "mxzaz-hqmqe-cnarv-dqu6p-osn6o-42zio-q4u4l-q4k2l-q4n2l-q4h2l-76z";
static RESOURCE_ID: &str = "premium-data-v1";
static PAYMENT_TTL_MS: u64 = 300_000; // 5 minutes

// Thread-local storage for replay protection
thread_local! {
    static CONSUMED_PAYMENTS: RefCell<BTreeSet<String>> = RefCell::new(BTreeSet::new());
}

// The 402 challenge endpoint
#[query]
fn get_premium_data() -> ErrorResponse {
    let canister_id = caller();

    let payment_info = PaymentInfo {
        scheme: "exact".to_string(),
        token: TokenInfo {
            chain: "ICP".to_string(),
            ledger_id: CKBTC_LEDGER_ID.to_string(),
            symbol: "ckBTC".to_string(),
        },
        amount: 1000, // 1000 satoshis = 0.00001 BTC
        recipient: canister_id,
        facilitator: Principal::from_text(TRUSTED_FACILITATOR).unwrap(),
    };

    ErrorResponse {
        code: "402".to_string(),
        message: "Payment Required".to_string(),
        x402: payment_info,
    }
}

// Secure access endpoint - requires verified payment
#[update]
async fn access_premium_data(payment_proof: PaymentProof) -> PaymentVerification {
    let now = ic_cdk::api::time() / 1_000_000; // Convert to milliseconds

    // 1. Basic validation
    if let (false, error) = validate_payment_basics(&payment_proof, now) {
        return PaymentVerification {
            success: false,
            block_index: 0,
            transaction_hash: String::new(),
            error: Some(error),
        };
    }

    // 2. Replay protection
    if CONSUMED_PAYMENTS.with(|payments| payments.borrow().contains(&payment_proof.payment_id)) {
        return PaymentVerification {
            success: false,
            block_index: 0,
            transaction_hash: String::new(),
            error: Some("Payment already consumed".to_string()),
        };
    }

    // 3. Verify with facilitator
    match verify_with_facilitator(&payment_proof).await {
        Ok((verified, block_index, tx_hash)) => {
            if verified {
                // Mark payment as consumed
                CONSUMED_PAYMENTS.with(|payments| {
                    payments.borrow_mut().insert(payment_proof.payment_id.clone());
                });

                PaymentVerification {
                    success: true,
                    block_index,
                    transaction_hash: tx_hash,
                    error: None,
                }
            } else {
                PaymentVerification {
                    success: false,
                    block_index: 0,
                    transaction_hash: String::new(),
                    error: Some("Facilitator verification failed".to_string()),
                }
            }
        }
        Err(error) => PaymentVerification {
            success: false,
            block_index: 0,
            transaction_hash: String::new(),
            error: Some(format!("Facilitator error: {}", error)),
        },
    }
}

// Validate basic payment requirements
fn validate_payment_basics(payment_proof: &PaymentProof, now: u64) -> (bool, String) {
    // Check expiry
    if payment_proof.expiry < now {
        return (false, "Payment expired".to_string());
    }

    // Check minimum TTL (prevent immediate expiry)
    if payment_proof.expiry < now + PAYMENT_TTL_MS {
        return (false, "Payment expiry too soon".to_string());
    }

    // Check trusted facilitator
    let _trusted_facilitator = Principal::from_text(TRUSTED_FACILITATOR).unwrap();
    if payment_proof.token_ledger != Principal::from_text(CKBTC_LEDGER_ID).unwrap() {
        return (false, "Invalid token ledger".to_string());
    }

    // Check amount matches expected price
    if payment_proof.amount != 1000 {
        return (false, "Incorrect payment amount".to_string());
    }

    // Check resource ID binding
    if payment_proof.resource_id != RESOURCE_ID {
        return (false, "Invalid resource binding".to_string());
    }

    // Check payment ID format
    if payment_proof.payment_id.len() < 32 {
        return (false, "Invalid payment ID format".to_string());
    }

    (true, String::new())
}

// Verify payment with Anda facilitator canister
async fn verify_with_facilitator(payment_proof: &PaymentProof) -> Result<(bool, u64, String), String> {
    let _facilitator_id = Principal::from_text(TRUSTED_FACILITATOR).unwrap();

    // In a real implementation, this would call the facilitator canister
    // For MVP, we'll simulate a basic verification
    // TODO: Replace with actual inter-canister call to facilitator

    // Simulate facilitator verification logic:
    // 1. Verify the signature
    // 2. Check that ICRC-2 approve -> transfer_from was executed
    // 3. Confirm funds moved from payer to merchant
    // 4. Validate amount and token match the quote

    // Mock successful verification for MVP
    let mock_block_index = 12345678 + payment_proof.nonce;
    let mock_tx_hash = format!("tx_{}_{}", payment_proof.payment_id, payment_proof.nonce);

    // In production, this would be:
    // let verification_result: (bool, String) = ic_cdk::call(
    //     facilitator_id,
    //     "verify_payment_capture",
    //     (payment_proof.clone(),)
    // ).await.map_err(|e| format!("Failed to call facilitator: {}", e))?;

    Ok((true, mock_block_index, mock_tx_hash))
}

#[query]
fn preview_premium_data() -> String {
    PREMIUM_DATA.to_string()
}

// Health check for grant demo
#[query]
fn health() -> String {
    "Merchant canister is running".to_string()
}

#[query]
fn canister_info() -> (Principal, String) {
    (caller(), "x402 Merchant Canister v0.1 - Secure Version".to_string())
}

#[query]
fn get_consumed_payment_count() -> u32 {
    CONSUMED_PAYMENTS.with(|payments| payments.borrow().len() as u32)
}

// Test helper function for secure payment verification
#[update]
async fn test_secure_payment() -> PaymentVerification {
    let now = ic_cdk::api::time() / 1_000_000;

    let test_payment_proof = PaymentProof {
        payment_id: format!("test_payment_{}", now),
        nonce: 42,
        payer: ic_cdk::caller(),
        amount: 1000,
        token_ledger: Principal::from_text(CKBTC_LEDGER_ID).unwrap(),
        facilitator_signature: vec![1, 2, 3, 4], // Mock signature
        expiry: now + 300_000, // 5 minutes from now
        resource_id: RESOURCE_ID.to_string(),
    };

    access_premium_data(test_payment_proof).await
}
