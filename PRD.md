# PRD: x402 Agentic Economy Implementation on ICP

## 1. Executive Summary
This document outlines the architecture and implementation plan for adopting the **x402 Payment Protocol** on the Internet Computer (ICP). The goal is to enable an "Agentic Economy" where AI agents can autonomously pay for API resources using **ckBTC**. 

This implementation leverages the existing open-source work by `ldclabs` (Anda Cloud) and adapts it for our specific requirement: enabling **ckBTC** payments for resources.

## 2. Core Components

### 2.1 The Token (ckBTC)
* **Standard:** **ICRC-2** (Required).
* **Why ICRC-2?** Unlike ICRC-1 (simple transfer), ICRC-2 allows the `Approve -> TransferFrom` pattern. This allows the Agent to "authorize" a payment, and the Facilitator to "capture" it only upon successful verification, similar to a credit card hold or EIP-3009 on EVM.
* **Canister ID:** `mxzaz-hqmqe-cnarv-dqu6p-osn6o-42zio-q4u4l-q4k2l-q4n2l-q4h2l-76z` (Mainnet ckBTC).

### 2.2 The Facilitator (Verifier)
* **Reference Implementation:** `anda_x402_canister` (by ldclabs).
* **Role:** Trusted intermediary that verifies the agent's payment authorization and settles the transaction (moves funds from Agent -> Merchant).
* **Source:** [GitHub - ldclabs/anda-cloud](https://github.com/ldclabs/anda-cloud/tree/main/rs/anda_x402_canister)

### 2.3 The Actors
* **Merchant (Server):** The API or Canister selling data/services.
* **Agent (Client):** The AI/User wanting access.
* **Facilitator:** The smart contract handling the settlement.

## 3. The "Exact" Payment Scheme (Workflow)

We will implement the **x402 Exact Scheme** as defined in the research.

### Step 1: Resource Discovery (The 402 Challenge)
**Agent** requests `GET /premium-data`.  
**Merchant** returns `402 Payment Required` with headers/body:
```json
{
  "error": {
    "code": "402",
    "message": "Payment Required",
    "x402": {
      "scheme": "exact",
      "token": {
        "chain": "ICP",
        "ledger_id": "mxzaz-hqmqe-cnarv-dqu6p-osn6o-42zio-q4u4l-q4k2l-q4n2l-q4h2l-76z",
        "symbol": "ckBTC"
      },
      "amount": "1000",
      "recipient": "merchant-account-id",
      "facilitator": "anda-facilitator-canister-id"
    }
  }
}
