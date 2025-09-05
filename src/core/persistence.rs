//! 任务持久化模块
//! 
//! 负责将用户设置的定时任务保存到本地文件，确保应用重启后能恢复任务

use anyhow::{Result, anyhow};
use log::{info, warn, error};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::core::types::TaskData;

/// 持久化配置
#[derive(Debug, Clone)]
struct PersistenceConfig {
    /// 数据目录路径
    data_dir: PathBuf,
    /// 任务文件名
    task_file: String,
    /// 配置文件名
    config_file: String,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            data_dir: Self::get_default_data_dir(),
            task_file: "tasks.json".to_string(),
            config_file: "config.json".to_string(),
        }
    }
}

impl PersistenceConfig {
    /// 获取默认数据目录
    fn get_default_data_dir() -> PathBuf {
        if let Some(local_data) = dirs::data_local_dir() {
            local_data.join("QtShut")
        } else {
            // 备用方案：使用当前目录
            PathBuf::from(".qtshut")
        }
    }
    
    /// 获取任务文件完整路径
    fn get_task_file_path(&self) -> PathBuf {
        self.data_dir.join(&self.task_file)
    }
    
    /// 获取配置文件完整路径
    fn get_config_file_path(&self) -> PathBuf {
        self.data_dir.join(&self.config_file)
    }
}

/// 应用配置数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 是否开机自启动
    pub auto_start: bool,
    /// 是否最小化到托盘
    pub minimize_to_tray: bool,
    /// 关机前确认
    pub confirm_before_shutdown: bool,
    /// 界面主题
    pub theme: String,
    /// 语言设置
    pub language: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            auto_start: false,
            minimize_to_tray: true,
            confirm_before_shutdown: true,
            theme: "light".to_string(),
            language: "zh-CN".to_string(),
        }
    }
}

/// 任务持久化管理器
#[derive(Debug)]
pub struct TaskPersistence {
    /// 持久化配置
    config: PersistenceConfig,
}

impl TaskPersistence {
    /// 创建新的任务持久化管理器
    pub fn new() -> Result<Self> {
        let config = PersistenceConfig::default();
        
        // 确保数据目录存在
        if !config.data_dir.exists() {
            fs::create_dir_all(&config.data_dir)
                .map_err(|e| anyhow!("创建数据目录失败: {}", e))?;
            info!("创建数据目录: {:?}", config.data_dir);
        }
        
        Ok(Self { config })
    }
    
    /// 使用自定义数据目录创建持久化管理器
    /// 
    /// # 参数
    /// 
    /// * `data_dir` - 自定义数据目录路径
    pub fn with_data_dir<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let mut config = PersistenceConfig::default();
        config.data_dir = data_dir.as_ref().to_path_buf();
        
        // 确保数据目录存在
        if !config.data_dir.exists() {
            fs::create_dir_all(&config.data_dir)
                .map_err(|e| anyhow!("创建数据目录失败: {}", e))?;
            info!("创建数据目录: {:?}", config.data_dir);
        }
        
        Ok(Self { config })
    }
    
    /// 保存任务数据
    /// 
    /// # 参数
    /// 
    /// * `task_data` - 要保存的任务数据
    pub fn save_task(&self, task_data: &TaskData) -> Result<()> {
        let task_file = self.config.get_task_file_path();
        
        let json_data = serde_json::to_string_pretty(task_data)
            .map_err(|e| anyhow!("序列化任务数据失败: {}", e))?;
            
        fs::write(&task_file, json_data)
            .map_err(|e| anyhow!("写入任务文件失败: {}", e))?;
            
        info!("任务数据已保存到: {:?}", task_file);
        Ok(())
    }
    
    /// 异步保存任务数据
    /// 
    /// # 参数
    /// 
    /// * `task_data` - 要保存的任务数据
    pub async fn save_task_async(&self, task_data: &TaskData) -> Result<()> {
        let task_file = self.config.get_task_file_path();
        
        let json_data = serde_json::to_string_pretty(task_data)
            .map_err(|e| anyhow!("序列化任务数据失败: {}", e))?;
            
        let mut file = async_fs::File::create(&task_file).await
            .map_err(|e| anyhow!("创建任务文件失败: {}", e))?;
            
        file.write_all(json_data.as_bytes()).await
            .map_err(|e| anyhow!("写入任务文件失败: {}", e))?;
            
        file.flush().await
            .map_err(|e| anyhow!("刷新任务文件失败: {}", e))?;
            
        info!("任务数据已异步保存到: {:?}", task_file);
        Ok(())
    }
    
    /// 加载任务数据
    /// 
    /// # 返回值
    /// 
    /// 返回加载的任务数据，如果文件不存在则返回None
    pub fn load_task(&self) -> Result<Option<TaskData>> {
        let task_file = self.config.get_task_file_path();
        
        if !task_file.exists() {
            info!("任务文件不存在: {:?}", task_file);
            return Ok(None);
        }
        
        let json_data = fs::read_to_string(&task_file)
            .map_err(|e| anyhow!("读取任务文件失败: {}", e))?;
            
        if json_data.trim().is_empty() {
            info!("任务文件为空");
            return Ok(None);
        }
        
        let task_data: TaskData = serde_json::from_str(&json_data)
            .map_err(|e| {
                error!("反序列化任务数据失败: {}", e);
                // 如果反序列化失败，备份损坏的文件并返回None
                if let Err(backup_err) = self.backup_corrupted_file(&task_file) {
                    warn!("备份损坏文件失败: {}", backup_err);
                }
                anyhow!("任务数据格式错误: {}", e)
            })?;
            
        info!("任务数据已加载: {:?}", task_data.task_type);
        Ok(Some(task_data))
    }
    
    /// 异步加载任务数据
    /// 
    /// # 返回值
    /// 
    /// 返回加载的任务数据，如果文件不存在则返回None
    pub async fn load_task_async(&self) -> Result<Option<TaskData>> {
        let task_file = self.config.get_task_file_path();
        
        if !task_file.exists() {
            info!("任务文件不存在: {:?}", task_file);
            return Ok(None);
        }
        
        let mut file = async_fs::File::open(&task_file).await
            .map_err(|e| anyhow!("打开任务文件失败: {}", e))?;
            
        let mut json_data = String::new();
        file.read_to_string(&mut json_data).await
            .map_err(|e| anyhow!("读取任务文件失败: {}", e))?;
            
        if json_data.trim().is_empty() {
            info!("任务文件为空");
            return Ok(None);
        }
        
        let task_data: TaskData = serde_json::from_str(&json_data)
            .map_err(|e| {
                error!("反序列化任务数据失败: {}", e);
                anyhow!("任务数据格式错误: {}", e)
            })?;
            
        info!("任务数据已异步加载: {:?}", task_data.task_type);
        Ok(Some(task_data))
    }
    
    /// 清除任务数据
    /// 
    /// 删除保存的任务文件
    pub fn clear_task(&self) -> Result<()> {
        let task_file = self.config.get_task_file_path();
        
        if task_file.exists() {
            fs::remove_file(&task_file)
                .map_err(|e| anyhow!("删除任务文件失败: {}", e))?;
            info!("任务文件已删除: {:?}", task_file);
        } else {
            info!("任务文件不存在，无需删除");
        }
        
        Ok(())
    }
    
    /// 异步清除任务数据
    pub async fn clear_task_async(&self) -> Result<()> {
        let task_file = self.config.get_task_file_path();
        
        if task_file.exists() {
            async_fs::remove_file(&task_file).await
                .map_err(|e| anyhow!("删除任务文件失败: {}", e))?;
            info!("任务文件已异步删除: {:?}", task_file);
        } else {
            info!("任务文件不存在，无需删除");
        }
        
        Ok(())
    }
    
    /// 保存应用配置
    /// 
    /// # 参数
    /// 
    /// * `config` - 应用配置数据
    pub fn save_config(&self, config: &AppConfig) -> Result<()> {
        let config_file = self.config.get_config_file_path();
        
        let json_data = serde_json::to_string_pretty(config)
            .map_err(|e| anyhow!("序列化配置数据失败: {}", e))?;
            
        fs::write(&config_file, json_data)
            .map_err(|e| anyhow!("写入配置文件失败: {}", e))?;
            
        info!("配置数据已保存到: {:?}", config_file);
        Ok(())
    }
    
    /// 加载应用配置
    /// 
    /// # 返回值
    /// 
    /// 返回加载的配置数据，如果文件不存在则返回默认配置
    pub fn load_config(&self) -> Result<AppConfig> {
        let config_file = self.config.get_config_file_path();
        
        if !config_file.exists() {
            info!("配置文件不存在，使用默认配置");
            let default_config = AppConfig::default();
            // 保存默认配置
            if let Err(e) = self.save_config(&default_config) {
                warn!("保存默认配置失败: {}", e);
            }
            return Ok(default_config);
        }
        
        let json_data = fs::read_to_string(&config_file)
            .map_err(|e| anyhow!("读取配置文件失败: {}", e))?;
            
        let config: AppConfig = serde_json::from_str(&json_data)
            .map_err(|e| {
                warn!("配置文件格式错误，使用默认配置: {}", e);
                // 备份损坏的配置文件
                if let Err(backup_err) = self.backup_corrupted_file(&config_file) {
                    warn!("备份损坏配置文件失败: {}", backup_err);
                }
                anyhow!("配置数据格式错误: {}", e)
            })
            .unwrap_or_else(|_| AppConfig::default());
            
        info!("配置数据已加载");
        Ok(config)
    }
    
    /// 检查数据目录是否存在且可写
    pub fn validate_data_directory(&self) -> Result<()> {
        let data_dir = &self.config.data_dir;
        
        if !data_dir.exists() {
            return Err(anyhow!("数据目录不存在: {:?}", data_dir));
        }
        
        if !data_dir.is_dir() {
            return Err(anyhow!("数据路径不是目录: {:?}", data_dir));
        }
        
        // 测试写入权限
        let test_file = data_dir.join(".write_test");
        match fs::write(&test_file, "test") {
            Ok(_) => {
                // 清理测试文件
                let _ = fs::remove_file(&test_file);
                Ok(())
            },
            Err(e) => Err(anyhow!("数据目录不可写: {}", e))
        }
    }
    
    /// 获取数据目录路径
    pub fn get_data_dir(&self) -> &Path {
        &self.config.data_dir
    }
    
    /// 获取数据目录大小（字节）
    pub fn get_data_size(&self) -> Result<u64> {
        let mut total_size = 0u64;
        
        if let Ok(entries) = fs::read_dir(&self.config.data_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_file() {
                            total_size += metadata.len();
                        }
                    }
                }
            }
        }
        
        Ok(total_size)
    }
    
    /// 清理所有数据
    /// 
    /// 删除所有保存的数据文件
    pub fn clear_all_data(&self) -> Result<()> {
        info!("清理所有数据...");
        
        // 删除任务文件
        let _ = self.clear_task();
        
        // 删除配置文件
        let config_file = self.config.get_config_file_path();
        if config_file.exists() {
            fs::remove_file(&config_file)
                .map_err(|e| anyhow!("删除配置文件失败: {}", e))?;
            info!("配置文件已删除: {:?}", config_file);
        }
        
        info!("所有数据已清理");
        Ok(())
    }
    
    /// 备份损坏的文件
    fn backup_corrupted_file(&self, file_path: &Path) -> Result<()> {
        let backup_path = file_path.with_extension("corrupted.bak");
        fs::copy(file_path, &backup_path)
            .map_err(|e| anyhow!("备份文件失败: {}", e))?;
        info!("损坏文件已备份到: {:?}", backup_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use chrono::Local;
    use crate::core::types::TaskType;
    
    fn create_test_task_data() -> TaskData {
        TaskData {
            task_type: TaskType::Once,
            target_time: Some(Local::now() + chrono::Duration::hours(1)),
            daily_time: None,
            enabled: true,
            created_at: Local::now(),
        }
    }
    
    #[test]
    fn test_persistence_creation() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = TaskPersistence::with_data_dir(temp_dir.path()).unwrap();
        
        assert!(persistence.validate_data_directory().is_ok());
    }
    
    #[test]
    fn test_save_and_load_task() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = TaskPersistence::with_data_dir(temp_dir.path()).unwrap();
        
        let task_data = create_test_task_data();
        
        // 保存任务
        assert!(persistence.save_task(&task_data).is_ok());
        
        // 加载任务
        let loaded_task = persistence.load_task().unwrap();
        assert!(loaded_task.is_some());
        
        let loaded_task = loaded_task.unwrap();
        assert_eq!(loaded_task.task_type, task_data.task_type);
        assert_eq!(loaded_task.enabled, task_data.enabled);
    }
    
    #[test]
    fn test_clear_task() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = TaskPersistence::with_data_dir(temp_dir.path()).unwrap();
        
        let task_data = create_test_task_data();
        
        // 保存任务
        persistence.save_task(&task_data).unwrap();
        
        // 确认任务存在
        assert!(persistence.load_task().unwrap().is_some());
        
        // 清除任务
        persistence.clear_task().unwrap();
        
        // 确认任务已清除
        assert!(persistence.load_task().unwrap().is_none());
    }
    
    #[test]
    fn test_save_and_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = TaskPersistence::with_data_dir(temp_dir.path()).unwrap();
        
        let mut config = AppConfig::default();
        config.auto_start = true;
        config.theme = "dark".to_string();
        
        // 保存配置
        assert!(persistence.save_config(&config).is_ok());
        
        // 加载配置
        let loaded_config = persistence.load_config().unwrap();
        assert_eq!(loaded_config.auto_start, config.auto_start);
        assert_eq!(loaded_config.theme, config.theme);
    }
    
    #[tokio::test]
    async fn test_async_operations() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = TaskPersistence::with_data_dir(temp_dir.path()).unwrap();
        
        let task_data = create_test_task_data();
        
        // 异步保存任务
        assert!(persistence.save_task_async(&task_data).await.is_ok());
        
        // 异步加载任务
        let loaded_task = persistence.load_task_async().await.unwrap();
        assert!(loaded_task.is_some());
        
        // 异步清除任务
        assert!(persistence.clear_task_async().await.is_ok());
        
        // 确认任务已清除
        assert!(persistence.load_task_async().await.unwrap().is_none());
    }
}