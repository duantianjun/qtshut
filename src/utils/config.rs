//! 配置管理模块
//! 
//! 负责应用程序配置的加载、保存和管理

use std::path::{Path, PathBuf};
use std::fs;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use dirs::config_dir;

use crate::ui::theme::ThemeType;
use crate::core::types::ShutdownMethod;

/// 应用程序配置
/// 
/// 包含所有可配置的应用程序设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 应用程序设置
    pub app: AppSettings,
    /// UI设置
    pub ui: UISettings,
    /// 关机设置
    pub shutdown: ShutdownSettings,
    /// 高级设置
    pub advanced: AdvancedSettings,
}

/// 应用程序基本设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// 开机自动启动
    pub auto_start: bool,
    /// 启动时最小化到托盘
    pub start_minimized: bool,
    /// 关闭时最小化到托盘而不是退出
    pub minimize_on_close: bool,
    /// 语言设置
    pub language: String,
    /// 检查更新
    pub check_updates: bool,
}

/// UI界面设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UISettings {
    /// 主题类型
    pub theme_type: ThemeType,
    /// 自定义主题名称
    pub custom_theme: Option<String>,
    /// 窗口位置
    pub window_position: Option<(f32, f32)>,
    /// 窗口大小
    pub window_size: Option<(f32, f32)>,
    /// 总是置顶
    pub always_on_top: bool,
    /// 显示托盘通知
    pub show_tray_notifications: bool,
}

/// 关机相关设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShutdownSettings {
    /// 默认关机方法
    pub default_method: ShutdownMethod,
    /// 关机前确认
    pub confirm_before_shutdown: bool,
    /// 确认对话框超时时间（秒）
    pub confirmation_timeout: u32,
    /// 强制关机（忽略未保存的工作）
    pub force_shutdown: bool,
    /// 关机前警告时间（分钟）
    pub warning_time: u32,
}

/// 高级设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSettings {
    /// 日志级别
    pub log_level: String,
    /// 启用调试模式
    pub debug_mode: bool,
    /// 数据备份
    pub backup_data: bool,
    /// 最大备份文件数
    pub max_backup_files: u32,
    /// 性能监控
    pub performance_monitoring: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app: AppSettings::default(),
            ui: UISettings::default(),
            shutdown: ShutdownSettings::default(),
            advanced: AdvancedSettings::default(),
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_start: false,
            start_minimized: false,
            minimize_on_close: true,
            language: "zh-CN".to_string(),
            check_updates: true,
        }
    }
}

impl Default for UISettings {
    fn default() -> Self {
        Self {
            theme_type: ThemeType::Light,
            custom_theme: None,
            window_position: None,
            window_size: Some((400.0, 500.0)),
            always_on_top: false,
            show_tray_notifications: true,
        }
    }
}

impl Default for ShutdownSettings {
    fn default() -> Self {
        Self {
            default_method: ShutdownMethod::Command,
            confirm_before_shutdown: true,
            confirmation_timeout: 30,
            force_shutdown: false,
            warning_time: 5,
        }
    }
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            debug_mode: false,
            backup_data: true,
            max_backup_files: 5,
            performance_monitoring: false,
        }
    }
}

/// 配置管理器
/// 
/// 负责配置文件的加载、保存和管理
#[derive(Debug)]
pub struct ConfigManager {
    /// 配置文件路径
    config_path: PathBuf,
    /// 当前配置
    config: AppConfig,
}

impl ConfigManager {
    /// 创建新的配置管理器
    /// 
    /// # 返回值
    /// 
    /// 成功返回配置管理器，失败返回错误信息
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::get_config_file_path()?;
        let config = Self::load_config(&config_path)?;
        
        Ok(Self {
            config_path,
            config,
        })
    }
    
    /// 获取配置文件路径
    /// 
    /// # 返回值
    /// 
    /// 成功返回配置文件路径，失败返回错误信息
    fn get_config_file_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config_dir = config_dir()
            .ok_or("无法获取配置目录")?;
        
        let app_config_dir = config_dir.join("QtShut");
        
        // 确保配置目录存在
        if !app_config_dir.exists() {
            fs::create_dir_all(&app_config_dir)?;
            info!("创建配置目录: {:?}", app_config_dir);
        }
        
        Ok(app_config_dir.join("config.json"))
    }
    
    /// 加载配置文件
    /// 
    /// # 参数
    /// 
    /// * `path` - 配置文件路径
    /// 
    /// # 返回值
    /// 
    /// 成功返回配置对象，失败返回错误信息
    fn load_config(path: &Path) -> Result<AppConfig, Box<dyn std::error::Error>> {
        if !path.exists() {
            info!("配置文件不存在，使用默认配置: {:?}", path);
            let default_config = AppConfig::default();
            
            // 保存默认配置
            Self::save_config_to_file(&default_config, path)?;
            
            return Ok(default_config);
        }
        
        info!("加载配置文件: {:?}", path);
        
        let config_content = fs::read_to_string(path)?;
        
        match serde_json::from_str::<AppConfig>(&config_content) {
            Ok(config) => {
                info!("配置文件加载成功");
                Ok(config)
            },
            Err(e) => {
                warn!("配置文件格式错误: {}, 使用默认配置", e);
                
                // 备份损坏的配置文件
                let backup_path = path.with_extension("json.backup");
                if let Err(backup_err) = fs::copy(path, &backup_path) {
                    warn!("备份损坏的配置文件失败: {}", backup_err);
                }
                
                let default_config = AppConfig::default();
                Self::save_config_to_file(&default_config, path)?;
                
                Ok(default_config)
            }
        }
    }
    
    /// 保存配置到文件
    /// 
    /// # 参数
    /// 
    /// * `config` - 配置对象
    /// * `path` - 文件路径
    /// 
    /// # 返回值
    /// 
    /// 成功返回Ok(())，失败返回错误信息
    fn save_config_to_file(config: &AppConfig, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let config_json = serde_json::to_string_pretty(config)?;
        fs::write(path, config_json)?;
        info!("配置文件保存成功: {:?}", path);
        Ok(())
    }
    
    /// 获取当前配置
    pub fn get_config(&self) -> &AppConfig {
        &self.config
    }
    
    /// 获取可变配置引用
    pub fn get_config_mut(&mut self) -> &mut AppConfig {
        &mut self.config
    }
    
    /// 保存配置
    /// 
    /// # 返回值
    /// 
    /// 成功返回Ok(())，失败返回错误信息
    pub fn save_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        Self::save_config_to_file(&self.config, &self.config_path)
    }
    
    /// 重新加载配置
    /// 
    /// # 返回值
    /// 
    /// 成功返回Ok(())，失败返回错误信息
    pub fn reload_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.config = Self::load_config(&self.config_path)?;
        Ok(())
    }
    
    /// 重置为默认配置
    /// 
    /// # 返回值
    /// 
    /// 成功返回Ok(())，失败返回错误信息
    pub fn reset_to_default(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.config = AppConfig::default();
        self.save_config()?;
        info!("配置已重置为默认值");
        Ok(())
    }
    
    /// 导出配置到指定路径
    /// 
    /// # 参数
    /// 
    /// * `export_path` - 导出路径
    /// 
    /// # 返回值
    /// 
    /// 成功返回Ok(())，失败返回错误信息
    pub fn export_config(&self, export_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        Self::save_config_to_file(&self.config, export_path)?;
        info!("配置已导出到: {:?}", export_path);
        Ok(())
    }
    
    /// 从指定路径导入配置
    /// 
    /// # 参数
    /// 
    /// * `import_path` - 导入路径
    /// 
    /// # 返回值
    /// 
    /// 成功返回Ok(())，失败返回错误信息
    pub fn import_config(&mut self, import_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if !import_path.exists() {
            return Err(format!("导入文件不存在: {:?}", import_path).into());
        }
        
        let imported_config = Self::load_config(import_path)?;
        self.config = imported_config;
        self.save_config()?;
        
        info!("配置已从 {:?} 导入", import_path);
        Ok(())
    }
    
    /// 验证配置有效性
    /// 
    /// # 返回值
    /// 
    /// 配置是否有效
    pub fn validate_config(&self) -> bool {
        // 验证基本设置
        if self.config.app.language.is_empty() {
            return false;
        }
        
        // 验证UI设置
        if let Some((width, height)) = self.config.ui.window_size {
            if width <= 0.0 || height <= 0.0 {
                return false;
            }
        }
        
        // 验证关机设置
        if self.config.shutdown.confirmation_timeout == 0 {
            return false;
        }
        
        // 验证高级设置
        if self.config.advanced.max_backup_files == 0 {
            return false;
        }
        
        true
    }
    
    /// 获取配置文件路径
    pub fn get_config_path(&self) -> &Path {
        &self.config_path
    }
}

/// 配置更新事件
#[derive(Debug, Clone)]
pub enum ConfigUpdateEvent {
    /// 主题更改
    ThemeChanged(ThemeType),
    /// 语言更改
    LanguageChanged(String),
    /// 自动启动设置更改
    AutoStartChanged(bool),
    /// 关机方法更改
    ShutdownMethodChanged(ShutdownMethod),
    /// 其他配置更改
    Other(String),
}

/// 配置验证器
/// 
/// 用于验证配置的有效性
pub struct ConfigValidator;

impl ConfigValidator {
    /// 验证应用设置
    /// 
    /// # 参数
    /// 
    /// * `settings` - 应用设置
    /// 
    /// # 返回值
    /// 
    /// 验证结果和错误信息
    pub fn validate_app_settings(settings: &AppSettings) -> (bool, Vec<String>) {
        let mut errors = Vec::new();
        
        if settings.language.is_empty() {
            errors.push("语言设置不能为空".to_string());
        }
        
        (errors.is_empty(), errors)
    }
    
    /// 验证UI设置
    /// 
    /// # 参数
    /// 
    /// * `settings` - UI设置
    /// 
    /// # 返回值
    /// 
    /// 验证结果和错误信息
    pub fn validate_ui_settings(settings: &UISettings) -> (bool, Vec<String>) {
        let mut errors = Vec::new();
        
        if let Some((width, height)) = settings.window_size {
            if width <= 0.0 || width > 10000.0 {
                errors.push("窗口宽度无效".to_string());
            }
            if height <= 0.0 || height > 10000.0 {
                errors.push("窗口高度无效".to_string());
            }
        }
        
        (errors.is_empty(), errors)
    }
    
    /// 验证关机设置
    /// 
    /// # 参数
    /// 
    /// * `settings` - 关机设置
    /// 
    /// # 返回值
    /// 
    /// 验证结果和错误信息
    pub fn validate_shutdown_settings(settings: &ShutdownSettings) -> (bool, Vec<String>) {
        let mut errors = Vec::new();
        
        if settings.confirmation_timeout == 0 || settings.confirmation_timeout > 300 {
            errors.push("确认超时时间应在1-300秒之间".to_string());
        }
        
        if settings.warning_time > 60 {
            errors.push("警告时间不应超过60分钟".to_string());
        }
        
        (errors.is_empty(), errors)
    }
    
    /// 验证高级设置
    /// 
    /// # 参数
    /// 
    /// * `settings` - 高级设置
    /// 
    /// # 返回值
    /// 
    /// 验证结果和错误信息
    pub fn validate_advanced_settings(settings: &AdvancedSettings) -> (bool, Vec<String>) {
        let mut errors = Vec::new();
        
        let valid_log_levels = ["error", "warn", "info", "debug", "trace"];
        if !valid_log_levels.contains(&settings.log_level.as_str()) {
            errors.push("无效的日志级别".to_string());
        }
        
        if settings.max_backup_files == 0 || settings.max_backup_files > 100 {
            errors.push("备份文件数量应在1-100之间".to_string());
        }
        
        (errors.is_empty(), errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    
    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        
        assert!(!config.app.auto_start);
        assert_eq!(config.app.language, "zh-CN");
        assert_eq!(config.ui.theme_type, ThemeType::Light);
        assert!(config.shutdown.confirm_before_shutdown);
        assert_eq!(config.advanced.log_level, "info");
    }
    
    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        
        // 测试序列化
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.is_empty());
        
        // 测试反序列化
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.app.language, deserialized.app.language);
    }
    
    #[test]
    fn test_config_validation() {
        let config = AppConfig::default();
        let manager = ConfigManager {
            config_path: PathBuf::from("test.json"),
            config,
        };
        
        assert!(manager.validate_config());
    }
    
    #[test]
    fn test_config_validator() {
        let app_settings = AppSettings::default();
        let (valid, errors) = ConfigValidator::validate_app_settings(&app_settings);
        assert!(valid);
        assert!(errors.is_empty());
        
        let ui_settings = UISettings::default();
        let (valid, errors) = ConfigValidator::validate_ui_settings(&ui_settings);
        assert!(valid);
        assert!(errors.is_empty());
    }
    
    #[test]
    fn test_invalid_window_size() {
        let mut ui_settings = UISettings::default();
        ui_settings.window_size = Some((-100.0, 200.0)); // 无效宽度
        
        let (valid, errors) = ConfigValidator::validate_ui_settings(&ui_settings);
        assert!(!valid);
        assert!(!errors.is_empty());
    }
}