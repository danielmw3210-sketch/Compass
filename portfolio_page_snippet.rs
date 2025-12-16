    fn render_portfolio(&mut self, ui: &mut egui::Ui) {
        ui.heading("📂 Portfolio");
        ui.label(egui::RichText::new("Track your training progress, owned NFTs, and marketplace listings")
            .color(egui::Color32::GRAY));
        ui.add_space(20.0);
        
        // ===== TRAINING EPOCH PROGRESS =====
        ui.group(|ui| {
            ui.label(egui::RichText::new("🧠 Training Progress").size(18.0).strong());
            ui.separator();
            ui.label("Track your models\' progress towards NFT minting (requires 10 epochs with 75%+ accuracy)");
            ui.add_space(10.0);
            
            if self.epoch_stats.is_empty() {
                ui.label(egui::RichText::new("No active training sessions")
                    .color(egui::Color32::GRAY));
                ui.label("💡 Tip: Submitted training jobs will appear here once they start generating predictions");
            } else {
                for (key, stats) in &self.epoch_stats {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&stats.ticker).size(16.0).strong());
                            ui.label(egui::RichText::new(format!("Model: {}", stats.model_id))
                                .size(12.0)
                                .color(egui::Color32::GRAY));
                        });
                        
                        ui.add_space(8.0);
                        
                        // Progress bar to epoch 10
                        let progress = (stats.current_epoch as f32 / 10.0).min(1.0);
                        let accuracy_color = if stats.overall_accuracy >= 0.75 {
                            egui::Color32::from_rgb(34, 197, 94) // Green
                        } else {
                            egui::Color32::from_rgb(251, 146, 60) // Orange
                        };
                        
                        ui.horizontal(|ui| {
                            ui.label(format!("Epoch {}/10", stats.current_epoch));
                            ui.add(egui::ProgressBar::new(progress)
                                .desired_width(200.0)
                                .fill(accuracy_color));
                            ui.label(format!("{:.1}% accuracy", stats.overall_accuracy * 100.0));
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label(format!("Predictions: {}/{} in current epoch", 
                                stats.predictions_in_epoch, 
                                10)); // epochs have 10 predictions each
                            ui.label(format!("Total: {} ({} correct)", 
                                stats.total_predictions,
                                stats.total_correct));
                        });
                        
                        if stats.nft_minted {
                            ui.label(egui::RichText::new("✅ NFT MINTED!")
                                .color(egui::Color32::from_rgb(34, 197, 94))
                                .strong());
                        } else if stats.current_epoch >= 10 && stats.overall_accuracy >= 0.75 {
                            ui.label(egui::RichText::new("🎉 Ready to mint NFT!")
                                .color(egui::Color32::from_rgb(59, 130, 246))
                                .strong());
                        }
                    });
                    
                    ui.add_space(10.0);
                }
            }
        });
        
        ui.add_space(20.0);
        
        // ===== OWNED NFTs =====
        ui.group(|ui| {
            ui.label(egui::RichText::new("🎨 My NFTs").size(18.0).strong());
            ui.separator();
            ui.label("Your AI model NFTs - list them on the marketplace to earn from rentals and sales");
            ui.add_space(10.0);
            
            if self.my_models.is_empty() {
                ui.label(egui::RichText::new("You don't own any model NFTs yet")
                    .color(egui::Color32::GRAY));
                ui.label("💡 Tip: Train a model to 10 epochs with 75%+ accuracy to mint your first NFT");
            } else {
                egui::Grid::new("portfolio_nfts").striped(true).num_columns(6).show(ui, |ui| {
                    // Header
                    ui.label(egui::RichText::new("NFT").strong());
                    ui.label(egui::RichText::new("Name").strong());
                    ui.label(egui::RichText::new("Accuracy").strong());
                    ui.label(egui::RichText::new("List Price").strong());
                    ui.label(egui::RichText::new("Status").strong());
                    ui.label(egui::RichText::new("Actions").strong());
                    ui.end_row();
                    
                    for nft in &self.my_models.clone() {
                        // NFT Icon
                        ui.label("🤖");
                        
                        // Name
                        ui.label(&nft.name);
                        
                        // Accuracy
                        let acc_color = if nft.accuracy >= 0.80 {
                            egui::Color32::from_rgb(34, 197, 94)
                        } else if nft.accuracy >= 0.70 {
                            egui::Color32::from_rgb(251, 146, 60)
                        } else {
                            egui::Color32::from_rgb(239, 68, 68)
                        };
                        ui.label(egui::RichText::new(format!("{:.1}%", nft.accuracy * 100.0))
                            .color(acc_color));
                        
                        // List Price (editable)
                        let price_label = if nft.price > 0 {
                            format!("{} COMPASS", nft.price)
                        } else {
                            "Not Listed".to_string()
                        };
                        ui.label(price_label);
                        
                        // Status
                        let status = if nft.price > 0 {
                            egui::RichText::new("🟢 Listed").color(egui::Color32::from_rgb(34, 197, 94))
                        } else {
                            egui::RichText::new("⚫ Unlisted").color(egui::Color32::GRAY)
                        };
                        ui.label(status);
                        
                        // Actions
                        ui.horizontal(|ui| {
                            if nft.price > 0 {
                                if ui.button("📝 Update Price").clicked() {
                                    // TODO: Open price edit dialog
                                }
                                if ui.button("❌ Unlist").clicked() {
                                    let _ = self.rpc_tx.try_send(RpcCommand::BuyNFT {
                                        token_id: nft.token_id.clone(),
                                        buyer: "UNLIST".to_string(), // Special marker
                                    });
                                }
                            } else {
                                if ui.button("📤 List on Marketplace").clicked() {
                                    // TODO: Open listing dialog with price input
                                    // For now, list at default price
                                    let _ = self.rpc_tx.try_send(RpcCommand::BuyNFT {
                                        token_id: nft.token_id.clone(),
                                        buyer: "LIST:1000".to_string(), // price:amount format
                                    });
                                }
                            }
                        });
                        
                        ui.end_row();
                    }
                });
            }
        });
        
        ui.add_space(20.0);
        
        // ===== MARKETPLACE EARNINGS =====
        ui.group(|ui| {
            ui.label(egui::RichText::new("💰 Marketplace Earnings").size(18.0).strong());
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Total Royalties Earned:");
                ui.label(egui::RichText::new("0 COMPUTE").strong()); // TODO: Track this
            });
            
            ui.horizontal(|ui| {
                ui.label("Active Rentals:");
                ui.label(egui::RichText::new("0").strong()); // TODO: Track this
            });
            
            ui.add_space(10.0);
            ui.label(egui::RichText::new("Coming soon: Rental tracking and earnings dashboard")
                .color(egui::Color32::GRAY)
                .size(12.0));
        });
    }
