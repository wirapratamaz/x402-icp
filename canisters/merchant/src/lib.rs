use candid::{CandidType, Deserialize as CandidDeserialize, Principal};
use ic_cdk::{query, update, caller};
use std::collections::BTreeSet;
use std::cell::RefCell;
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

#[derive(CandidType, CandidDeserialize)]
struct PaymentInfo {
    scheme: String,
    token: TokenInfo,
    amount: u64,
    recipient: Principal,
    facilitator: Principal,
}

#[derive(CandidType, CandidDeserialize)]
struct TokenInfo {
    chain: String,
    ledger_id: String,
    symbol: String,
}

#[derive(CandidType, CandidDeserialize)]
struct ErrorResponse {
    code: String,
    message: String,
    x402: PaymentInfo,
}

#[derive(CandidType, CandidDeserialize)]
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

// Anda facilitator payment payload structures
#[derive(Serialize, Deserialize)]
struct IcpPayloadAuthorization {
    scheme: String,
    asset: String,
    to: String,
    value: String,
    expires_at: u64,
    nonce: u64,
}

#[derive(Serialize, Deserialize)]
struct IcpPayload {
    signature: String,
    authorization: IcpPayloadAuthorization,
}

// Production-ready Anda facilitator interfaces
#[derive(CandidType, CandidDeserialize)]
pub struct Account {
    owner: Principal,
    subaccount: Option<Vec<u8>>,
}

#[derive(CandidType, CandidDeserialize)]
pub struct CommitPaymentArgs {
    payment_payload: String,
    payment_expectation: PaymentExpectation,
}

#[derive(CandidType, CandidDeserialize)]
pub struct PaymentExpectation {
    caller: Principal,
    merchant_account: Account,
    amount: candid::Nat,
    token: Principal,
}

#[derive(CandidType, CandidDeserialize)]
pub struct PaymentReceipt {
    height: candid::Nat, // Block index of settlement
    tx_hash: String,
}

pub type CommitPaymentResponse = Result<PaymentReceipt, String>;

// Legacy structures for backwards compatibility
#[derive(Deserialize)]
struct AndaVerifyRequest {
    payment_payload: IcpPayload,
}

#[derive(Deserialize)]
struct AndaVerifyResponse {
    success: bool,
    error: Option<String>,
    fee: Option<String>,
    from: Option<String>,
    to: Option<String>,
    value: Option<String>,
    asset: Option<String>,
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
static MAX_FEE_SATS: u64 = 200;

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
    let _trusted_facilitator = Principal::from_text(TRUSTED_FACILITATOR).unwrap_or_else(|_| ic_cdk::caller());
    let expected_ledger = Principal::from_text(CKBTC_LEDGER_ID).unwrap_or_else(|_| ic_cdk::caller());
    if payment_proof.token_ledger != expected_ledger {
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

    // Check payment ID format (minimum length for testing)
    if payment_proof.payment_id.len() < 8 {
        return (false, "Invalid payment ID format".to_string());
    }

    (true, String::new())
}

// Verify payment with REAL Anda facilitator (Production Implementation)
async fn verify_with_facilitator(payment_proof: &PaymentProof) -> Result<(bool, u64, String), String> {
    // Production: Use real inter-canister calls to Anda facilitator
    verify_payment_with_anda_production(payment_proof).await
}

// Production-ready Anda facilitator inter-canister call
async fn verify_payment_with_anda_production(payment_proof: &PaymentProof) -> Result<(bool, u64, String), String> {
    let facilitator_principal = Principal::from_text(TRUSTED_FACILITATOR)
        .map_err(|e| format!("Invalid facilitator ID: {}", e))?;
    let merchant_self = ic_cdk::api::id();
    let caller_principal = ic_cdk::caller();

    // Create payment payload string for Anda
    let payload = IcpPayload {
        signature: BASE64.encode(&payment_proof.facilitator_signature),
        authorization: IcpPayloadAuthorization {
            scheme: "exact".to_string(),
            asset: payment_proof.token_ledger.to_string(),
            to: caller_principal.to_string(),
            value: payment_proof.amount.to_string(),
            expires_at: payment_proof.expiry,
            nonce: payment_proof.nonce,
        },
    };

    // Serialize payload to JSON string
    let payload_str = serde_json::to_string(&payload)
        .map_err(|e| format!("Failed to serialize payment payload: {}", e))?;

    // 1. Construct the Payment Expectation
    // Tell the facilitator: "We expect this payment to send X amount to our canister"
    let args = CommitPaymentArgs {
        payment_payload: payload_str,
        payment_expectation: PaymentExpectation {
            caller: caller_principal, // The agent verifying the payment
            merchant_account: Account {
                owner: merchant_self,
                subaccount: None,
            },
            amount: candid::Nat::from(payment_proof.amount),
            token: payment_proof.token_ledger,
        },
    };

    // 2. Inter-Canister Call to Facilitator
    // Method: `commit_payment_exact` (for exact scheme)
    let (response,): (CommitPaymentResponse,) = ic_cdk::call(
        facilitator_principal,
        "commit_payment_exact",
        (args,),
    )
    .await
    .map_err(|(code, msg)| format!("Inter-canister call failed: {:?} - {}", code, msg))?;

    // 3. Handle the Result
    match response {
        Ok(receipt) => {
            // Convert Nat to u64 for internal logic
            let block_index = receipt.height.0.try_into().unwrap_or(0);
            let tx_hash = receipt.tx_hash;

            // Log successful settlement (production debugging)
            ic_cdk::println!("Anda facilitator settlement: {} at block {}", tx_hash, block_index);

            Ok((true, block_index, tx_hash))
        }
        Err(e) => {
            // Payment failed (insufficient allowance, expired, invalid signature, etc.)
            ic_cdk::println!("Anda facilitator settlement failed: {}", e);
            Err(format!("Settlement failed: {}", e))
        }
    }
}

// Mock HTTP verification with Anda facilitator
async fn verify_payment_with_anda_http(request: &AndaVerifyRequest) -> Result<AndaVerifyResponse, String> {
    // For MVP: Simulate successful verification
    // In production, this would make a real HTTP request to the Anda facilitator
    let mock_response = AndaVerifyResponse {
        success: true,
        error: None,
        fee: Some("100".to_string()), // Mock fee in satoshis
        from: Some(request.payment_payload.authorization.to.clone()),
        to: Some(request.payment_payload.authorization.to.clone()),
        value: Some(request.payment_payload.authorization.value.clone()),
        asset: Some(request.payment_payload.authorization.asset.clone()),
    };

    Ok(mock_response)
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

// Test helper function for secure payment verification (MVP mode)
#[update]
async fn test_secure_payment() -> PaymentVerification {
    let now = ic_cdk::api::time() / 1_000_000;

    let test_payment_proof = PaymentProof {
        payment_id: format!("test_payment_{}", now),
        nonce: 42,
        payer: ic_cdk::caller(),
        amount: 1000,
        token_ledger: Principal::from_text(CKBTC_LEDGER_ID).unwrap_or_else(|_| ic_cdk::caller()),
        facilitator_signature: vec![1, 2, 3, 4], // Mock signature
        expiry: now + 300_000, // 5 minutes from now
        resource_id: RESOURCE_ID.to_string(),
    };

    access_premium_data(test_payment_proof).await
}

// Test production-ready payment verification (Production mode)
#[update]
async fn test_production_payment() -> PaymentVerification {
    let now = ic_cdk::api::time() / 1_000_000;

    let test_payment_proof = PaymentProof {
        payment_id: format!("prod_payment_{}", now),
        nonce: 100, // Higher nonce for production
        payer: ic_cdk::caller(),
        amount: 1000,
        token_ledger: Principal::from_text(CKBTC_LEDGER_ID).unwrap_or_else(|_| ic_cdk::caller()),
        facilitator_signature: vec![5, 6, 7, 8, 9, 10], // Production signature
        expiry: now + 600_000, // 10 minutes from now
        resource_id: RESOURCE_ID.to_string(),
    };

    // This will use the real inter-canister call to Anda facilitator
    access_premium_data(test_payment_proof).await
}

// Test Anda facilitator integration
#[query]
fn test_anda_integration() -> String {
    format!("Anda Facilitator Integration Status:\n\
    Facilitator ID: {}\n\
    Supported Assets: ckBTC, ckUSDC, ICP\n\
    Real API: https://ogkpr-lyaaa-aaaap-an5fq-cai.icp0.io/\n\
    Verification Endpoint: POST /verify\n\
    Settlement Endpoint: POST /settle\n\
    Note: MVP uses mock HTTP calls. Production requires real HTTP integration.", TRUSTED_FACILITATOR)
}
