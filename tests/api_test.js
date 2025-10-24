/**
 * This file is to test all the endpoints of the Simulation Engine
 */
const BASE_URL = "http://127.0.0.1:8080";

async function makeRequest(method, url, body = null) {
  const options = {
    method,
    headers: {
      "Content-Type": "application/json",
    },
  };

  if (body) {
    options.body = JSON.stringify(body);
  }

  try {
    const response = await fetch(url, options);
    const data = await response.json();

    console.log(`${method} ${url}`);
    console.log("Response:", JSON.stringify(data, null, 2));
    console.log("---");

    return { status: response.status, data };
  } catch (error) {
    console.error(`Error making request to ${url}:`, error.message);
    return { error: error.message };
  }
}

function validateApiResponse(response, expectSuccess = true) {
  if (response.error) {
    console.log("Response: ", response);
    throw new Error(`Request failed: ${response.error}`);
  }

  const { data } = response;

  if (!data.hasOwnProperty("success")) {
    throw new Error("Response missing success field");
  }

  if (expectSuccess && !data.success) {
    throw new Error(`Expected success but got error: ${data.error}`);
  }

  return data;
}

const SAMPLE_TX_BASE64 =
  "AXcHeh3VQ98L5NZef2XFN8RJ4tCfCFPs7p0moQBTZoDaQfAzmTgPGdoz2wHHHWhog0A15hoEoP/tXbzcaA+rgg4BAAIEfu9qh+um2yVbtcpTwBe72hQh8KRPu9Yo1vcyXgY9MvnuSZayjnEgc0XeoV+EFSzUmiVj+t9BH118eIOmeMuNvQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAwZGb+UhFzL/7K26csOb57yM5bvF9xJrLEObOkAAAADDNXfAd64TVOMzHI5/HU4geFP1xZAEPgVElEJkhrvvzwMDAAUCQJwAAAMACQMA4fUFAAAAAAICAAEMAgAAADA2vusLAAAA";

async function runTests() {
  let forkId = null;
  let testsPassed = 0;
  let testsFailed = 0;

  try {
    console.log("Test 1: Creating a fork...");
    const response = await makeRequest("POST", `${BASE_URL}/forks`);
    const apiResponse = validateApiResponse(response);

    if (apiResponse.data && typeof apiResponse.data === "string") {
      forkId = apiResponse.data;
      console.log(`Fork created successfully with ID: ${forkId}\n`);
      testsPassed++;
    } else {
      throw new Error("Fork ID not returned");
    }
  } catch (error) {
    console.log(`Test 1 failed: ${error.message}\n`);
    testsFailed++;
  }

  if (!forkId) {
    console.log("Cannot continue tests without a valid fork ID");
    return;
  }

  try {
    console.log("Test 2: Setting lamports for an account...");
    const testPubkey = "H3B7dM826FyyZe2ehuu6zzFEFvL1HdLvk994pzpfakJp";
    const response = await makeRequest(
      "POST",
      `${BASE_URL}/forks/${forkId}/set_lamports`,
      {
        pubkey: testPubkey,
        lamports: 1_000_000,
      }
    );

    const apiResponse = validateApiResponse(response);
    console.log(`Lamports set successfully for ${testPubkey}\n`);
    testsPassed++;
  } catch (error) {
    console.log(`Test 2 failed: ${error.message}\n`);
    testsFailed++;
  }

  try {
    console.log("Test 3: Getting account information...");
    const testPubkey = "H3B7dM826FyyZe2ehuu6zzFEFvL1HdLvk994pzpfakJp";
    const response = await makeRequest(
      "POST",
      `${BASE_URL}/forks/${forkId}/get_account`,
      {
        pubkey: testPubkey,
      }
    );

    const apiResponse = validateApiResponse(response);
    if (apiResponse.data && typeof apiResponse.data === "object") {
      console.log(`Account retrieved successfully for ${testPubkey}`);
      console.log(`   Lamports: ${apiResponse.data.lamports}`);
      console.log(`   Owner: ${apiResponse.data.owner}\n`);
      testsPassed++;
    } else {
      throw new Error("Account data not returned");
    }
  } catch (error) {
    console.log(`Test 3 failed: ${error.message}\n`);
    testsFailed++;
  }

  try {
    console.log("Test 4: Setting token balance...");
    const response = await makeRequest(
      "POST",
      `${BASE_URL}/forks/${forkId}/set_token_balance`,
      {
        token_account: "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
        mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        owner: "DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1",
        amount: 1000000,
      }
    );

    const apiResponse = validateApiResponse(response);
    console.log("Token balance set successfully\n");
    testsPassed++;
  } catch (error) {
    console.log(`Test 4 failed: ${error.message}\n`);
    testsFailed++;
  }

  try {
    console.log("Test 5: Testing with invalid fork ID...");
    const invalidForkId = "00000000-0000-0000-0000-000000000000";
    const response = await makeRequest(
      "POST",
      `${BASE_URL}/forks/${invalidForkId}/set_lamports`,
      {
        pubkey: "So11111111111111111111111111111111111111112",
        lamports: 1000000,
      }
    );

    const apiResponse = response.data;
    if (apiResponse && !apiResponse.success && apiResponse.error) {
      console.log("Invalid fork ID handled correctly\n");
      testsPassed++;
    } else {
      throw new Error("Invalid fork ID should have returned an error");
    }
  } catch (error) {
    console.log(`Test 5 failed: ${error.message}\n`);
    testsFailed++;
  }

  try {
    console.log("Test 6: Deleting fork...");
    const response = await makeRequest("DELETE", `${BASE_URL}/forks/${forkId}`);
    const apiResponse = validateApiResponse(response);

    console.log(`Fork ${forkId} deleted successfully\n`);
    testsPassed++;
  } catch (error) {
    console.log(`Test 6 failed: ${error.message}\n`);
    testsFailed++;
  }

  try {
    console.log("Test 7: Testing deleted fork usage...");
    const response = await makeRequest(
      "POST",
      `${BASE_URL}/forks/${forkId}/set_lamports`,
      {
        pubkey: "So11111111111111111111111111111111111111112",
        lamports: 1000000,
      }
    );

    const apiResponse = response.data;
    if (apiResponse && !apiResponse.success && apiResponse.error) {
      console.log("Deleted fork correctly returns error\n");
      testsPassed++;
    } else {
      throw new Error("Deleted fork should have returned an error");
    }
  } catch (error) {
    console.log(`Test 7 failed: ${error.message}\n`);
    testsFailed++;
  }

  console.log(
    "================================================================================="
  );
  console.log("TEST RESULTS");
  console.log(
    "================================================================================="
  );
  console.log(`Tests Passed: ${testsPassed}`);
  console.log(`Tests Failed: ${testsFailed}`);
  console.log(`Total Tests: ${testsPassed + testsFailed}`);

  if (testsFailed === 0) {
    console.log("All tests passed!");
  } else {
    console.log("Some tests failed. Check the output above for details.");
  }
}

if (require.main === module) {
  runTests().catch(console.error);
}

module.exports = {
  BASE_URL,
  makeRequest,
  validateApiResponse,
  SAMPLE_TX_BASE64,
};
