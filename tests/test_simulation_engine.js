/**
 * This file tests transactions on the Simulate Engine
 */

const {
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
  VersionedTransaction,
  TransactionMessage,
} = require("@solana/web3.js");

const { BASE_URL, makeRequest, validateApiResponse } = require("./api_test.js");

// Create a transaction
async function createRealTransaction() {
  const fromKeypair = Keypair.generate();
  const toKeypair = Keypair.generate();

  console.log("Sender Public Key:", fromKeypair.publicKey.toBase58());
  console.log("Receiver Public Key:", toKeypair.publicKey.toBase58());

  const transferAmount = 0.1 * LAMPORTS_PER_SOL;

  const transferInstruction = SystemProgram.transfer({
    fromPubkey: fromKeypair.publicKey,
    toPubkey: toKeypair.publicKey,
    lamports: transferAmount,
  });

  const messageV0 = new TransactionMessage({
    payerKey: fromKeypair.publicKey,
    recentBlockhash: "J5Wey6iC8xGVh86fDpFGmRmzxWKCw5KC9A7Dd8Z5KRZC",
    instructions: [transferInstruction],
  }).compileToV0Message();

  const transaction = new VersionedTransaction(messageV0);

  transaction.sign([fromKeypair]);
  const serializedTransaction = Buffer.from(transaction.serialize()).toString(
    "base64"
  );

  return {
    transaction: serializedTransaction,
    fromKeypair,
    toKeypair,
    transferAmount,
    blockhash: "J5Wey6iC8xGVh86fDpFGmRmzxWKCw5KC9A7Dd8Z5KRZC",
  };
}

// Test simulation and execution with real transaction
async function testRealTransactionSimulationAndExecution() {
  console.log("Testing simulation and execution...");

  let forkId;
  try {
    console.log("Creating fork...");
    const forkResponse = await makeRequest("POST", `${BASE_URL}/forks`);
    const forkData = validateApiResponse(forkResponse);
    forkId = forkData.data;
    console.log(`Fork created: ${forkId}\n`);
    const txData = await createRealTransaction();

    console.log("Funding sender account...");
    const fundingAmount = 2 * LAMPORTS_PER_SOL;
    await makeRequest("POST", `${BASE_URL}/forks/${forkId}/set_lamports`, {
      pubkey: txData.fromKeypair.publicKey.toString(),
      lamports: fundingAmount,
    });

    // Simulate the transaction
    console.log("Simulating transaction...");
    const simulationResponse = await makeRequest(
      "POST",
      `${BASE_URL}/forks/${forkId}/simulate`,
      {
        tx_base64: txData.transaction,
      }
    );

    if (simulationResponse.data.success) {
      console.log("Transaction simulation succeeded!");
      console.log(
        "metadata:",
        JSON.stringify(simulationResponse.data.data, null, 2)
      );
    } else {
      console.log(
        "Transaction simulation failed:",
        simulationResponse.data.error
      );
      return;
    }

    // Execute the transaction
    console.log("\nExecuting transaction...");
    const executionResponse = await makeRequest(
      "POST",
      `${BASE_URL}/forks/${forkId}/execute`,
      {
        tx_base64: txData.transaction,
      }
    );

    if (executionResponse.data.success) {
      console.log("Transaction execution succeeded!");
      console.log(
        "metadata:",
        JSON.stringify(executionResponse.data.data, null, 2)
      );

    } else {
      console.log(
        "Transaction execution failed:",
        executionResponse.data.error
      );
    }
  } catch (error) {
    console.error("Test failed:", error.message);
  }
}

// Test multiple real transactions
async function testMultipleRealTransactions() {
  console.log("\n\nTesting multiple transactions...\n");

  let forkId = null;
  try {
    const forkResponse = await makeRequest("POST", `${BASE_URL}/forks`);
    const forkData = validateApiResponse(forkResponse);
    forkId = forkData.data;
    console.log(`Fork created: ${forkId}\n`);

    const sender = Keypair.generate();
    const receiver1 = Keypair.generate();
    const receiver2 = Keypair.generate();

    await makeRequest("POST", `${BASE_URL}/forks/${forkId}/set_lamports`, {
      pubkey: sender.publicKey.toString(),
      lamports: 5 * LAMPORTS_PER_SOL,
    });
    console.log("Sender funded with 5 SOL\n");

    console.log("Creating first transaction (1 SOL)...");
    const tx1Message = new TransactionMessage({
      payerKey: sender.publicKey,
      recentBlockhash: "J5Wey6iC8xGVh86fDpFGmRmzxWKCw5KC9A7Dd8Z5KRZC",
      instructions: [
        SystemProgram.transfer({
          fromPubkey: sender.publicKey,
          toPubkey: receiver1.publicKey,
          lamports: 1 * LAMPORTS_PER_SOL,
        }),
      ],
    }).compileToV0Message();

    const tx1 = new VersionedTransaction(tx1Message);
    tx1.sign([sender]);
    const tx1Base64 = Buffer.from(tx1.serialize()).toString("base64");

    const result1 = await makeRequest(
      "POST",
      `${BASE_URL}/forks/${forkId}/execute`,
      {
        tx_base64: tx1Base64,
      }
    );
    console.log(
      "First transaction result:",
      result1.data.success ? "SUCCESS" : "FAILED"
    );

    console.log("Creating second transaction...");
    const tx2Message = new TransactionMessage({
      payerKey: sender.publicKey,
      recentBlockhash: "J5Wey6iC8xGVh86fDpFGmRmzxWKCw5KC9A7Dd8Z5KRZC",
      instructions: [
        SystemProgram.transfer({
          fromPubkey: sender.publicKey,
          toPubkey: receiver2.publicKey,
          lamports: 2 * LAMPORTS_PER_SOL,
        }),
      ],
    }).compileToV0Message();

    const tx2 = new VersionedTransaction(tx2Message);
    tx2.sign([sender]);
    const tx2Base64 = Buffer.from(tx2.serialize()).toString("base64");

    const result2 = await makeRequest(
      "POST",
      `${BASE_URL}/forks/${forkId}/execute`,
      {
        tx_base64: tx2Base64,
      }
    );
    console.log(
      "Second transaction result:",
      result2.data.success ? "SUCCESS" : "FAILED"
    );

  } catch (error) {
    console.error("Multiple transactions test failed:", error.message);
  }
}

// Main test runner
async function runRealTransactionTests() {
  console.log(
    "Starting tests for Simulation Engine...\n"
  );

  try {
    await testRealTransactionSimulationAndExecution();

    console.log("\n-----------------------------------------------------------");

    await testMultipleRealTransactions();

    console.log("\n-----------------------------------------------------------");
  } catch (error) {
    console.error("failed:", error.message);
  }
}

runRealTransactionTests().catch(console.error);
