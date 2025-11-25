#!/usr/bin/env node

const { Actor, HttpAgent } = require('@dfinity/agent');
const { Principal } = require('@dfinity/principal');

// Simple test to verify the x402 flow works
async function testX402Flow() {
  console.log('🚀 Testing x402 Payment Flow...\n');

  try {
    // Initialize agent for local network
    const agent = new HttpAgent({
      host: 'http://127.0.0.1:8000',
    });

    // Disable fetch root key for local development
    await agent.fetchRootKey();

    // Get canister IDs from dfx
    const merchantId = process.argv[2];
    const agentId = process.argv[3];

    if (!merchantId || !agentId) {
      console.error('❌ Usage: npm run test-flow <merchant-canister-id> <agent-canister-id>');
      process.exit(1);
    }

    const merchant = Actor.createActor(require('../canisters/merchant/merchant.did'), {
      agent,
      canisterId: Principal.fromText(merchantId),
    });

    const agentCanister = Actor.createActor(require('../canisters/agent/agent.did'), {
      agent,
      canisterId: Principal.fromText(agentId),
    });

    console.log('1️⃣ Testing Merchant Health Check...');
    const health = await merchant.health();
    console.log(`   ✅ Merchant: ${health}`);

    console.log('\n2️⃣ Testing Agent Status...');
    const agentStatus = await agentCanister.agent_status();
    console.log(`   ✅ Agent: ${agentStatus}`);

    console.log('\n3️⃣ Attempting to Access Premium Data (Should Trigger 402)...');
    const premiumDataResult = await merchant.get_premium_data();

    if ('Err' in premiumDataResult) {
      const paymentInfo = premiumDataResult.Err.error.x402;
      console.log(`   ✅ 402 Payment Required Triggered!`);
      console.log(`   💰 Amount: ${paymentInfo.amount} sats`);
      console.log(`   🪙 Token: ${paymentInfo.token.symbol}`);
      console.log(`   🏭 Facilitator: ${paymentInfo.facilitator}`);
    } else {
      console.log('   ❌ Expected 402 error, got data instead');
    }

    console.log('\n🎉 Basic x402 flow test completed!');
    console.log('\n📋 Next Steps:');
    console.log('   1. Implement real ckBTC approval in agent canister');
    console.log('   2. Add facilitator integration for payment verification');
    console.log('   3. Build frontend demo interface');

  } catch (error) {
    console.error('❌ Test failed:', error.message);
    process.exit(1);
  }
}

testX402Flow();