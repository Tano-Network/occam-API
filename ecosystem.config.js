module.exports = {
  apps: [
    {
      name: "rust-server",
      script: "./target/release/evm",
      env: {
        NETWORK_PRIVATE_KEY: "9d2fe65604d872ea2b45f7dd48c49d4a83984f11e2f179275a65b98fe84d4899",
        SP1_PROVER: "network"
      }
    }
  ]
};
