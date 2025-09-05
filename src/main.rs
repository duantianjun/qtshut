//! QtShut - 轻量化Windows定时关机软件
//! 
//! 这是一个专为普通家庭用户设计的定时关机工具，
//! 提供极简的操作界面和可靠的定时关机功能。

use log::info;

mod app;
mod core;
mod ui;
mod utils;


/// 应用程序入口点
/// 
/// 初始化日志系统并启动GUI
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("QtShut 启动中...");

    // 创建并启动应用
    let app = app::App::new().await?;
    app.run().await?;
    
    Ok(())
}