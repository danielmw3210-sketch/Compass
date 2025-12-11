// Replace lines 774-776 in main.rs with this code:

                                 TransactionPayload::Mint(mint_params) => {
                                      println!("[EXEC] Processing Mint for {}", mint_params.owner);
                                      
                                      // Get Oracle keypair (Node acts as Oracle)
                                      let oracle_kp = w_guard.get_wallet("Daniel").and_then(|w| w.get_keypair());
                                      
                                      if let Some(kp) = oracle_kp {
                                          let oracle_pubkey = kp.public_key_hex();
                                          
                                          // Generate Oracle signature
                                          let oracle_msg = format!(
                                              "MINT:{}:{}:{}:{}:{}",
                                              mint_params.collateral_asset,
                                              mint_params.collateral_amount,
                                              mint_params.tx_proof,
                                              mint_params.mint_amount,
                                              mint_params.owner
                                          );
                                          let oracle_sig = kp.sign_hex(oracle_msg.as_bytes());
                                          
                                          // Construct BlockHeader
                                          let prev_hash = c_guard.head_hash().unwrap_or_default();
                                          let header = crate::block::BlockHeader {
                                              index: c_guard.height,
                                              timestamp: current_unix_timestamp_ms() as u64,
                                              prev_hash,
                                              hash: String::new(),
                                              proposer: "Validator1".to_string(),
                                              signature_hex: String::new(),
                                              block_type: crate::block::BlockType::Mint {
                                                  owner: mint_params.owner.clone(),
                                                  asset: mint_params.compass_asset.clone(),
                                                  amount: mint_params.mint_amount,
                                                  collateral_tx: mint_params.tx_proof.clone(),
                                                  collateral_amount: mint_params.collateral_amount,
                                                  oracle_signature: oracle_sig,
                                                  fee: mint_params.fee,
                                              }
                                          };
                                          
                                          // Calculate hash
                                          let mut header_final = header;
                                          header_final.hash = header_final.calculate_hash();
                                          
                                          // Append to Chain
                                          match c_guard.append_mint(header_final, &oracle_pubkey) {
                                              Ok(_) => println!("EXEC: Mined Mint Block - {} received {} {}", 
                                                  mint_params.owner, mint_params.mint_amount, mint_params.compass_asset),
                                              Err(e) => println!("EXEC: Mint Failed: {}", e),
                                          }
                                      } else {
                                          println!("EXEC: Mint Failed - Oracle wallet not found");
                                      }
                                  },
                                  TransactionPayload::Burn(_) => {
                                      println!("EXEC: Burn not yet implemented");
                                  }
