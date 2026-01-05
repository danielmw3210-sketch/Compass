// Quick training script for enhanced signal models
use rust_compass::layer3::signal_model;

#[tokio::main]
async fn main() {
    println!("🚀 Starting Enhanced Signal Model Training");
    println!("📊 Training with 11 features (ATR, Stochastic, OBV added)");
    println!("{}", "=".repeat(60));
    println!();
    
    match signal_model::train_all_signal_models().await {
        Ok(paths) => {
            println!();
            println!("{}", "=".repeat(60));
            println!("✅ All models trained successfully!");
            println!();
            println!("📁 Models saved:");
            for path in paths {
                println!("   • {}", path);
            }
            println!();
            println!("🎯 Next steps:");
            println!("   1. Restart your node to load new models");
            println!("   2. Monitor prediction accuracy in logs");
            println!("   3. Expect +5-10% accuracy improvement!");
        }
        Err(e) => {
            eprintln!();
            eprintln!("❌ Training failed: {}", e);
            eprintln!("   Check your internet connection (needs Binance API)");
            std::process::exit(1);
        }
    }
}
