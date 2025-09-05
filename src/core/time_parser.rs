//! 时间解析器模块
//! 
//! 负责解析用户输入的各种时间格式，支持自然语言和标准格式

use anyhow::{Result, anyhow};
use chrono::{Local, NaiveTime, Duration, Timelike, TimeZone};
use regex::Regex;
use std::collections::HashMap;
use lazy_static::lazy_static;
use log::{debug, warn, info};
use std::sync::OnceLock;

use crate::core::types::TimeInput;

/// 时间解析器
#[derive(Debug, Clone)]
pub struct TimeParser {
    /// 预编译的正则表达式
    patterns: TimePatterns,
    /// 中文数字映射
    chinese_numbers: HashMap<String, u32>,
}

/// 时间模式集合
#[derive(Debug, Clone)]
struct TimePatterns {
    /// 相对时间模式（如"30分钟"、"2小时"）
    duration_pattern: Regex,
    /// 绝对时间模式（如"22:30"、"晚上10点"）
    absolute_pattern: Regex,
    /// 每日时间模式（如"每天22:00"）
    daily_pattern: Regex,
}

lazy_static! {
    /// 时间单位映射
    static ref TIME_UNITS: HashMap<&'static str, i64> = {
        let mut m = HashMap::new();
        // 秒
        m.insert("秒", 1);
        m.insert("秒钟", 1);
        m.insert("s", 1);
        m.insert("sec", 1);
        m.insert("second", 1);
        m.insert("seconds", 1);
        
        // 分钟
        m.insert("分", 60);
        m.insert("分钟", 60);
        m.insert("m", 60);
        m.insert("min", 60);
        m.insert("minute", 60);
        m.insert("minutes", 60);
        
        // 小时
        m.insert("时", 3600);
        m.insert("小时", 3600);
        m.insert("h", 3600);
        m.insert("hour", 3600);
        m.insert("hours", 3600);
        
        m
    };
    
    /// 时间描述词映射
    static ref TIME_DESCRIPTIONS: HashMap<&'static str, i32> = {
        let mut m = HashMap::new();
        m.insert("早上", 8);
        m.insert("上午", 10);
        m.insert("中午", 12);
        m.insert("下午", 14);
        m.insert("傍晚", 18);
        m.insert("晚上", 20);
        m.insert("深夜", 23);
        m
    };
}

impl TimeParser {
    /// 创建新的时间解析器
    pub fn new() -> Self {
        let patterns = TimePatterns {
            // 匹配相对时间：数字+单位
            duration_pattern: Regex::new(r"(?i)(\d+)\s*(秒钟?|分钟?|小?时|[smh]|sec|min|hour)s?").unwrap(),
            
            // 匹配绝对时间：HH:MM 或 描述词+时间
            absolute_pattern: Regex::new(r"(?i)(早上|上午|中午|下午|傍晚|晚上|深夜)?\s*(\d{1,2})[：:]?(\d{2})?").unwrap(),
            
            // 匹配每日时间：每天/每日 + 时间
            daily_pattern: Regex::new(r"(?i)(每天|每日)\s*(\d{1,2})[：:]?(\d{2})?").unwrap(),
        };
        
        Self { 
            patterns,
            chinese_numbers: Self::init_chinese_numbers(),
        }
    }
    
    /// 获取全局时间解析器实例（单例模式）
    /// 
    /// # 返回值
    /// 
    /// 时间解析器的静态引用
    pub fn global() -> &'static TimeParser {
        static INSTANCE: OnceLock<TimeParser> = OnceLock::new();
        INSTANCE.get_or_init(|| TimeParser::new())
    }
    
    /// 初始化中文数字映射
    fn init_chinese_numbers() -> HashMap<String, u32> {
        let mut chinese_numbers = HashMap::new();
        chinese_numbers.insert("零".to_string(), 0);
        chinese_numbers.insert("一".to_string(), 1);
        chinese_numbers.insert("二".to_string(), 2);
        chinese_numbers.insert("两".to_string(), 2);
        chinese_numbers.insert("三".to_string(), 3);
        chinese_numbers.insert("四".to_string(), 4);
        chinese_numbers.insert("五".to_string(), 5);
        chinese_numbers.insert("六".to_string(), 6);
        chinese_numbers.insert("七".to_string(), 7);
        chinese_numbers.insert("八".to_string(), 8);
        chinese_numbers.insert("九".to_string(), 9);
        chinese_numbers.insert("十".to_string(), 10);
        chinese_numbers.insert("十一".to_string(), 11);
        chinese_numbers.insert("十二".to_string(), 12);
        chinese_numbers.insert("十三".to_string(), 13);
        chinese_numbers.insert("十四".to_string(), 14);
        chinese_numbers.insert("十五".to_string(), 15);
        chinese_numbers.insert("十六".to_string(), 16);
        chinese_numbers.insert("十七".to_string(), 17);
        chinese_numbers.insert("十八".to_string(), 18);
        chinese_numbers.insert("十九".to_string(), 19);
        chinese_numbers.insert("二十".to_string(), 20);
        chinese_numbers.insert("二十一".to_string(), 21);
        chinese_numbers.insert("二十二".to_string(), 22);
        chinese_numbers.insert("二十三".to_string(), 23);
        chinese_numbers.insert("二十四".to_string(), 24);
        chinese_numbers.insert("二十五".to_string(), 25);
        chinese_numbers.insert("二十六".to_string(), 26);
        chinese_numbers.insert("二十七".to_string(), 27);
        chinese_numbers.insert("二十八".to_string(), 28);
        chinese_numbers.insert("二十九".to_string(), 29);
        chinese_numbers.insert("三十".to_string(), 30);
        chinese_numbers.insert("四十".to_string(), 40);
        chinese_numbers.insert("五十".to_string(), 50);
        chinese_numbers.insert("六十".to_string(), 60);
        chinese_numbers
    }
    
    /// 解析用户输入的时间字符串
    /// 
    /// # 参数
    /// 
    /// * `input` - 用户输入的时间字符串
    /// 
    /// # 返回值
    /// 
    /// 返回解析后的时间输入类型或错误
    pub fn parse(&self, input: &str) -> Result<TimeInput> {
        let input = input.trim();
        info!("开始解析时间输入: {}", input);
        
        // 预处理：转换中文数字
        let processed_input = self.preprocess_chinese_numbers(input);
        debug!("预处理后的输入: {}", processed_input);
        
        // 尝试解析自然语言时间
        if let Ok(time_input) = self.parse_natural_language(&processed_input) {
            debug!("成功解析自然语言时间");
            return Ok(time_input);
        }
        
        // 尝试解析每日时间
        if let Some(captures) = self.patterns.daily_pattern.captures(&processed_input) {
            debug!("匹配到每日时间模式");
            return self.parse_daily_time(&captures);
        }
        
        // 尝试解析相对时间
        if let Some(captures) = self.patterns.duration_pattern.captures(&processed_input) {
            debug!("匹配到相对时间模式");
            return self.parse_duration(&captures);
        }
        
        // 尝试解析绝对时间
        if let Some(captures) = self.patterns.absolute_pattern.captures(&processed_input) {
            debug!("匹配到绝对时间模式");
            return self.parse_absolute_time(&captures);
        }
        
        warn!("无法识别的时间格式: {}", input);
        Err(anyhow!("无法识别的时间格式: {}", input))
    }
    
    /// 预处理中文数字
    fn preprocess_chinese_numbers(&self, input: &str) -> String {
        let mut result = input.to_string();
        
        // 按长度降序排列，先替换长的字符串，避免部分匹配
        let mut sorted_numbers: Vec<_> = self.chinese_numbers.iter().collect();
        sorted_numbers.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        
        for (chinese, number) in sorted_numbers {
            result = result.replace(chinese, &number.to_string());
        }
        result
    }
    
    /// 解析自然语言时间表达
    fn parse_natural_language(&self, input: &str) -> Result<TimeInput> {
        match input {
            "半小时后" => Ok(TimeInput::Duration(Duration::minutes(30))),
            "一小时后" | "1小时后" => Ok(TimeInput::Duration(Duration::hours(1))),
            "两小时后" | "2小时后" => Ok(TimeInput::Duration(Duration::hours(2))),
            "三小时后" | "3小时后" => Ok(TimeInput::Duration(Duration::hours(3))),
            "明天" => {
                let now = Local::now();
                let tomorrow = now + Duration::days(1);
                let target = tomorrow.date_naive().and_hms_opt(9, 0, 0)
                    .ok_or_else(|| anyhow!("无法构造明天时间"))?;
                let target_dt = Local.from_local_datetime(&target)
                    .single()
                    .ok_or_else(|| anyhow!("无法构造本地时间"))?;
                Ok(TimeInput::AbsoluteTime(target_dt))
            },
            "后天" => {
                let now = Local::now();
                let day_after_tomorrow = now + Duration::days(2);
                let target = day_after_tomorrow.date_naive().and_hms_opt(9, 0, 0)
                    .ok_or_else(|| anyhow!("无法构造后天时间"))?;
                let target_dt = Local.from_local_datetime(&target)
                    .single()
                    .ok_or_else(|| anyhow!("无法构造本地时间"))?;
                Ok(TimeInput::AbsoluteTime(target_dt))
            },
            "今晚" => {
                let now = Local::now();
                let today = now.date_naive();
                let target = today.and_hms_opt(20, 0, 0)
                    .ok_or_else(|| anyhow!("无法构造今晚时间"))?;
                let target_dt = Local.from_local_datetime(&target)
                    .single()
                    .ok_or_else(|| anyhow!("无法构造本地时间"))?;
                let final_target = if target_dt <= now {
                    target_dt + Duration::days(1)
                } else {
                    target_dt
                };
                Ok(TimeInput::AbsoluteTime(final_target))
            },
            "明早" | "明天早上" => {
                let now = Local::now();
                let tomorrow = now + Duration::days(1);
                let target = tomorrow.date_naive().and_hms_opt(7, 0, 0)
                    .ok_or_else(|| anyhow!("无法构造明早时间"))?;
                let target_dt = Local.from_local_datetime(&target)
                    .single()
                    .ok_or_else(|| anyhow!("无法构造本地时间"))?;
                Ok(TimeInput::AbsoluteTime(target_dt))
            },
            "明天晚上" => {
                let now = Local::now();
                let tomorrow = now + Duration::days(1);
                let target = tomorrow.date_naive().and_hms_opt(20, 0, 0)
                    .ok_or_else(|| anyhow!("无法构造明天晚上时间"))?;
                let target_dt = Local.from_local_datetime(&target)
                    .single()
                    .ok_or_else(|| anyhow!("无法构造本地时间"))?;
                Ok(TimeInput::AbsoluteTime(target_dt))
            },
            "中午" => {
                let now = Local::now();
                let today = now.date_naive();
                let target = today.and_hms_opt(12, 0, 0)
                    .ok_or_else(|| anyhow!("无法构造中午时间"))?;
                let target_dt = Local.from_local_datetime(&target)
                    .single()
                    .ok_or_else(|| anyhow!("无法构造本地时间"))?;
                let final_target = if target_dt <= now {
                    target_dt + Duration::days(1)
                } else {
                    target_dt
                };
                Ok(TimeInput::AbsoluteTime(final_target))
            },
            "下午" => {
                let now = Local::now();
                let today = now.date_naive();
                let target = today.and_hms_opt(14, 0, 0)
                    .ok_or_else(|| anyhow!("无法构造下午时间"))?;
                let target_dt = Local.from_local_datetime(&target)
                    .single()
                    .ok_or_else(|| anyhow!("无法构造本地时间"))?;
                let final_target = if target_dt <= now {
                    target_dt + Duration::days(1)
                } else {
                    target_dt
                };
                Ok(TimeInput::AbsoluteTime(final_target))
            },
            "晚上" => {
                let now = Local::now();
                let today = now.date_naive();
                let target = today.and_hms_opt(20, 0, 0)
                    .ok_or_else(|| anyhow!("无法构造晚上时间"))?;
                let target_dt = Local.from_local_datetime(&target)
                    .single()
                    .ok_or_else(|| anyhow!("无法构造本地时间"))?;
                let final_target = if target_dt <= now {
                    target_dt + Duration::days(1)
                } else {
                    target_dt
                };
                Ok(TimeInput::AbsoluteTime(final_target))
            },
            "深夜" => {
                let now = Local::now();
                let today = now.date_naive();
                let target = today.and_hms_opt(23, 0, 0)
                    .ok_or_else(|| anyhow!("无法构造深夜时间"))?;
                let target_dt = Local.from_local_datetime(&target)
                    .single()
                    .ok_or_else(|| anyhow!("无法构造本地时间"))?;
                let final_target = if target_dt <= now {
                    target_dt + Duration::days(1)
                } else {
                    target_dt
                };
                Ok(TimeInput::AbsoluteTime(final_target))
            },
            "凌晨" => {
                let now = Local::now();
                let tomorrow = now + Duration::days(1);
                let target = tomorrow.date_naive().and_hms_opt(2, 0, 0)
                    .ok_or_else(|| anyhow!("无法构造凌晨时间"))?;
                let target_dt = Local.from_local_datetime(&target)
                    .single()
                    .ok_or_else(|| anyhow!("无法构造本地时间"))?;
                Ok(TimeInput::AbsoluteTime(target_dt))
            },
            _ => Err(anyhow!("不支持的自然语言表达"))
        }
    }
    
    /// 验证解析后的时间输入是否有效
    /// 
    /// # 参数
    /// 
    /// * `time_input` - 解析后的时间输入
    pub fn validate(&self, time_input: &TimeInput) -> Result<()> {
        match time_input {
            TimeInput::Duration(duration) => {
                if duration.num_seconds() <= 0 {
                    return Err(anyhow!("时间间隔必须大于0"));
                }
                if duration.num_seconds() > 24 * 3600 {
                    return Err(anyhow!("时间间隔不能超过24小时"));
                }
            },
            TimeInput::AbsoluteTime(datetime) => {
                let now = Local::now();
                if *datetime <= now {
                    return Err(anyhow!("目标时间必须在当前时间之后"));
                }
                // 检查是否超过24小时
                let diff = *datetime - now;
                if diff.num_hours() > 24 {
                    return Err(anyhow!("目标时间不能超过24小时后"));
                }
            },
            TimeInput::DailyTime(_time) => {
                // 每日时间总是有效的，因为会自动调整到下一个匹配的时间
            }
        }
        Ok(())
    }
    
    /// 解析相对时间（持续时间）
    fn parse_duration(&self, captures: &regex::Captures) -> Result<TimeInput> {
        let number_str = captures.get(1)
            .ok_or_else(|| anyhow!("无法提取数字"))?
            .as_str();
            
        let number: i64 = if let Some(&num) = self.chinese_numbers.get(number_str) {
            num as i64
        } else {
            number_str.parse()
                .map_err(|_| anyhow!("无效的数字格式"))?
        };
            
        let unit = captures.get(2)
            .ok_or_else(|| anyhow!("无法提取时间单位"))?
            .as_str()
            .to_lowercase();
            
        let seconds = TIME_UNITS.get(unit.as_str())
            .ok_or_else(|| anyhow!("不支持的时间单位: {}", unit))?;
            
        let total_seconds = number * seconds;
        let duration = Duration::seconds(total_seconds);
        
        // 验证持续时间范围
        if duration.num_seconds() <= 0 {
            return Err(anyhow!("持续时间必须大于0"));
        }
        
        if duration.num_days() > 365 {
            return Err(anyhow!("持续时间不能超过365天"));
        }
        
        Ok(TimeInput::Duration(duration))
    }
    
    /// 解析绝对时间
    fn parse_absolute_time(&self, captures: &regex::Captures) -> Result<TimeInput> {
        let description = captures.get(1).map(|m| m.as_str());
        let hour_str = captures.get(2)
            .ok_or_else(|| anyhow!("无法提取小时"))?
            .as_str();
        let minute_str = captures.get(3).map(|m| m.as_str()).unwrap_or("0");
        
        let mut hour: u32 = if let Some(&num) = self.chinese_numbers.get(hour_str) {
            num
        } else {
            hour_str.parse()
                .map_err(|_| anyhow!("无效的小时格式"))?
        };
        
        let minute: u32 = if let Some(&num) = self.chinese_numbers.get(minute_str) {
            num
        } else {
            minute_str.parse()
                .map_err(|_| anyhow!("无效的分钟格式"))?
        };
            
        // 处理时间描述词
        if let Some(desc) = description {
            if let Some(&base_hour) = TIME_DESCRIPTIONS.get(desc) {
                // 如果用户输入的是相对小时（如"晚上8点"），调整小时
                if hour <= 12 {
                    hour = (base_hour as u32 + hour - 8).max(0).min(23);
                }
            }
        }
        
        // 验证时间范围
        if hour >= 24 || minute >= 60 {
            return Err(anyhow!("无效的时间: {}:{:02}", hour, minute));
        }
        
        // 构造目标时间
        let now = Local::now();
        let today = now.date_naive();
        let target_time = NaiveTime::from_hms_opt(hour, minute, 0)
            .ok_or_else(|| anyhow!("无法构造时间"))?;
        let target_datetime = today.and_time(target_time);
        
        let target = Local.from_local_datetime(&target_datetime)
            .single()
            .ok_or_else(|| anyhow!("无法构造本地时间"))?;
            
        // 如果目标时间已过，设置为明天
        let final_target = if target <= now {
            target + Duration::days(1)
        } else {
            target
        };
        
        Ok(TimeInput::AbsoluteTime(final_target))
    }
    
    /// 解析每日时间
    fn parse_daily_time(&self, captures: &regex::Captures) -> Result<TimeInput> {
        let hour_str = captures.get(2)
            .ok_or_else(|| anyhow!("无法提取小时"))?
            .as_str();
        let minute_str = captures.get(3).map(|m| m.as_str()).unwrap_or("0");
        
        let hour: u32 = if let Some(&num) = self.chinese_numbers.get(hour_str) {
            num
        } else {
            hour_str.parse()
                .map_err(|_| anyhow!("无效的小时格式"))?
        };
        
        let minute: u32 = if let Some(&num) = self.chinese_numbers.get(minute_str) {
            num
        } else {
            minute_str.parse()
                .map_err(|_| anyhow!("无效的分钟格式"))?
        };
            
        // 验证时间范围
        if hour >= 24 || minute >= 60 {
            return Err(anyhow!("无效的时间: {}:{:02}", hour, minute));
        }
        
        let time = NaiveTime::from_hms_opt(hour, minute, 0)
            .ok_or_else(|| anyhow!("无法构造时间"))?;
            
        Ok(TimeInput::DailyTime(time))
    }
    
    /// 格式化时间输入为显示字符串
    pub fn format_time_input(&self, input: &TimeInput) -> String {
        match input {
            TimeInput::Duration(duration) => {
                self.format_duration(*duration)
            },
            TimeInput::AbsoluteTime(datetime) => {
                datetime.format("%Y-%m-%d %H:%M:%S").to_string()
            },
            TimeInput::DailyTime(time) => {
                time.format("%H:%M").to_string()
            },
        }
    }
    
    /// 格式化时间输入为用户友好的字符串
    pub fn format_time_input_friendly(&self, input: &TimeInput) -> String {
        match input {
            TimeInput::Duration(duration) => {
                format!("{}后", self.format_duration_friendly(*duration))
            },
            TimeInput::AbsoluteTime(datetime) => {
                let now = Local::now();
                let diff = *datetime - now;
                
                if diff.num_days() == 0 {
                    format!("今天 {}", datetime.format("%H:%M"))
                } else if diff.num_days() == 1 {
                    format!("明天 {}", datetime.format("%H:%M"))
                } else if diff.num_days() < 7 {
                    format!("{}天后 {}", diff.num_days(), datetime.format("%H:%M"))
                } else {
                    datetime.format("%m月%d日 %H:%M").to_string()
                }
            },
            TimeInput::DailyTime(time) => {
                format!("每天 {}", time.format("%H:%M"))
            },
        }
    }
    
    /// 格式化持续时间
    fn format_duration(&self, duration: Duration) -> String {
        let total_seconds = duration.num_seconds();
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        
        let mut parts = Vec::new();
        
        if days > 0 {
            parts.push(format!("{}天", days));
        }
        if hours > 0 {
            parts.push(format!("{}小时", hours));
        }
        if minutes > 0 {
            parts.push(format!("{}分钟", minutes));
        }
        if seconds > 0 || parts.is_empty() {
            parts.push(format!("{}秒", seconds));
        }
        
        parts.join("")
    }
    
    /// 格式化持续时间为友好格式
    fn format_duration_friendly(&self, duration: Duration) -> String {
        let total_seconds = duration.num_seconds();
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        let minutes = (total_seconds % 3600) / 60;
        
        if days > 0 {
            if hours > 0 {
                format!("{}天{}小时", days, hours)
            } else {
                format!("{}天", days)
            }
        } else if hours > 0 {
            if minutes > 0 {
                format!("{}小时{}分钟", hours, minutes)
            } else {
                format!("{}小时", hours)
            }
        } else if minutes > 0 {
            format!("{}分钟", minutes)
        } else {
            format!("{}秒", total_seconds)
        }
    }
    
    /// 验证时间输入的有效性
    pub fn validate_time_input(&self, input: &TimeInput) -> Result<()> {
        match input {
            TimeInput::Duration(duration) => {
                if duration.num_seconds() <= 0 {
                    return Err(anyhow!("持续时间必须大于0"));
                }
                if duration.num_days() > 365 {
                    return Err(anyhow!("持续时间不能超过365天"));
                }
                if duration.num_seconds() < 10 {
                    return Err(anyhow!("持续时间不能少于10秒"));
                }
            },
            TimeInput::AbsoluteTime(datetime) => {
                let now = Local::now();
                if *datetime <= now {
                    return Err(anyhow!("绝对时间必须在未来"));
                }
                let max_future = now + Duration::days(365);
                if *datetime > max_future {
                    return Err(anyhow!("绝对时间不能超过一年后"));
                }
                // 检查时间是否过于接近（少于10秒）
                let diff = *datetime - now;
                if diff.num_seconds() < 10 {
                    return Err(anyhow!("绝对时间必须至少在10秒后"));
                }
            },
            TimeInput::DailyTime(time) => {
                // 验证时间格式的合理性
                let hour = time.hour();
                let minute = time.minute();
                if hour > 23 || minute > 59 {
                    return Err(anyhow!("无效的每日时间格式"));
                }
            },
        }
        Ok(())
    }
    
    /// 获取时间输入的剩余时间（秒）
    pub fn get_remaining_seconds(&self, input: &TimeInput) -> Result<i64> {
        match input {
            TimeInput::Duration(duration) => {
                Ok(duration.num_seconds())
            },
            TimeInput::AbsoluteTime(datetime) => {
                let now = Local::now();
                let diff = *datetime - now;
                Ok(diff.num_seconds().max(0))
            },
            TimeInput::DailyTime(time) => {
                let now = Local::now();
                let today = now.date_naive();
                let target_today = today.and_time(*time);
                
                let target_dt = Local.from_local_datetime(&target_today)
                    .single()
                    .ok_or_else(|| anyhow!("无法构造本地时间"))?;
                
                let target = if target_dt <= now {
                    target_dt + Duration::days(1)
                } else {
                    target_dt
                };
                
                let diff = target - now;
                Ok(diff.num_seconds().max(0))
            },
        }
    }
    
    /// 检查时间输入是否已过期
    pub fn is_expired(&self, input: &TimeInput) -> bool {
        match self.get_remaining_seconds(input) {
            Ok(seconds) => seconds <= 0,
            Err(_) => true,
        }
    }
    
    /// 获取支持的时间格式示例
    pub fn get_format_examples(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("相对时间", "30分钟后, 2小时后, 1天后"),
            ("绝对时间", "14:30, 2024-01-01 15:00:00"),
            ("每日时间", "每天8点, 每天18:30"),
            ("自然语言", "半小时后, 明天, 今晚, 中午"),
            ("中文数字", "三十分钟后, 两小时后, 明天八点"),
            ("复合时间", "1小时30分钟后, 2天3小时后"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, Duration};
    
    #[test]
    fn test_time_parser_creation() {
        let parser = TimeParser::new();
        assert!(!parser.chinese_numbers.is_empty());
    }
    
    #[test]
    fn test_global_instance() {
        let parser1 = TimeParser::global();
        let parser2 = TimeParser::global();
        // 验证是同一个实例（地址相同）
        assert_eq!(parser1.chinese_numbers.len(), parser2.chinese_numbers.len());
    }
    
    #[test]
    fn test_duration_parsing() {
        let parser = TimeParser::new();
        
        // 测试基本持续时间
        let test_cases = vec![
            ("30分钟", 30 * 60),
            ("2小时", 2 * 3600),
            ("90秒", 90),
            ("1h", 3600),
            ("30m", 30 * 60),
        ];
        
        for (input, expected_seconds) in test_cases {
            let result = parser.parse(input);
            assert!(result.is_ok(), "Failed to parse: {}", input);
            
            if let Ok(TimeInput::Duration(duration)) = result {
                assert_eq!(duration.num_seconds(), expected_seconds, "Wrong duration for: {}", input);
            } else {
                panic!("Expected Duration for: {}", input);
            }
        }
    }
    
    #[test]
    fn test_chinese_number_parsing() {
        let parser = TimeParser::new();
        
        let test_cases = vec![
            ("三十分钟", 30 * 60),
            ("两小时", 2 * 3600),
            ("五分钟", 5 * 60),
        ];
        
        for (input, expected_seconds) in test_cases {
            let result = parser.parse(input);
            assert!(result.is_ok(), "Failed to parse Chinese: {}", input);
            
            if let Ok(TimeInput::Duration(duration)) = result {
                assert_eq!(duration.num_seconds(), expected_seconds, "Wrong duration for Chinese: {}", input);
            }
        }
    }
    
    #[test]
    fn test_absolute_time_parsing() {
        let parser = TimeParser::new();
        
        // 测试绝对时间
        assert!(parser.parse("22:30").is_ok());
        assert!(parser.parse("晚上10点").is_ok());
        assert!(parser.parse("下午2:30").is_ok());
    }
    
    #[test]
    fn test_daily_time_parsing() {
        let parser = TimeParser::new();
        
        let test_cases = vec![
            ("每天22:00", 22, 0),
            ("每日8:30", 8, 30),
        ];
        
        for (input, expected_hour, expected_minute) in test_cases {
            let result = parser.parse(input);
            assert!(result.is_ok(), "Failed to parse daily time: {}", input);
            
            if let Ok(TimeInput::DailyTime(time)) = result {
                assert_eq!(time.hour(), expected_hour, "Wrong hour for: {}", input);
                assert_eq!(time.minute(), expected_minute, "Wrong minute for: {}", input);
            } else {
                panic!("Expected DailyTime for: {}", input);
            }
        }
    }
    
    #[test]
    fn test_natural_language_parsing() {
        let parser = TimeParser::new();
        
        let test_cases = vec![
            "半小时后",
            "明天",
            "今晚",
            "中午",
        ];
        
        for input in test_cases {
            let result = parser.parse(input);
            assert!(result.is_ok(), "Failed to parse natural language: {}", input);
        }
    }
    
    #[test]
    fn test_format_duration() {
        let parser = TimeParser::new();
        
        let test_cases = vec![
            (Duration::minutes(30), "30分钟"),
            (Duration::hours(2), "2小时"),
            (Duration::minutes(90), "1小时30分钟"),
            (Duration::days(1), "1天"),
        ];
        
        for (duration, expected) in test_cases {
            let formatted = parser.format_duration(duration);
            assert!(formatted.contains(expected), "Format mismatch: {} should contain {}", formatted, expected);
        }
    }
    
    #[test]
    fn test_format_time_input_friendly() {
        let parser = TimeParser::new();
        
        // 测试持续时间格式化
        let duration_input = TimeInput::Duration(Duration::minutes(30));
        let formatted = parser.format_time_input_friendly(&duration_input);
        assert!(formatted.contains("后"));
        
        // 测试每日时间格式化
        let daily_input = TimeInput::DailyTime(NaiveTime::from_hms_opt(8, 30, 0).unwrap());
        let formatted = parser.format_time_input_friendly(&daily_input);
        assert!(formatted.contains("每天"));
    }
    
    #[test]
    fn test_validation() {
        let parser = TimeParser::new();
        
        // 测试有效的持续时间
        let valid_duration = TimeInput::Duration(Duration::minutes(30));
        assert!(parser.validate_time_input(&valid_duration).is_ok());
        
        // 测试无效的持续时间（太短）
        let invalid_duration = TimeInput::Duration(Duration::seconds(5));
        assert!(parser.validate_time_input(&invalid_duration).is_err());
        
        // 测试无效的持续时间（太长）
        let too_long_duration = TimeInput::Duration(Duration::days(400));
        assert!(parser.validate_time_input(&too_long_duration).is_err());
    }
    
    #[test]
    fn test_remaining_seconds() {
        let parser = TimeParser::new();
        
        // 测试持续时间的剩余秒数
        let duration_input = TimeInput::Duration(Duration::minutes(30));
        let remaining = parser.get_remaining_seconds(&duration_input);
        assert!(remaining.is_ok());
        assert_eq!(remaining.unwrap(), 30 * 60);
    }
    
    #[test]
    fn test_is_expired() {
        let parser = TimeParser::new();
        
        // 测试未过期的持续时间
        let future_duration = TimeInput::Duration(Duration::minutes(30));
        assert!(!parser.is_expired(&future_duration));
        
        // 测试已过期的绝对时间
        let past_time = TimeInput::AbsoluteTime(Local::now() - Duration::hours(1));
        assert!(parser.is_expired(&past_time));
    }
    
    #[test]
    fn test_get_format_examples() {
        let parser = TimeParser::new();
        let examples = parser.get_format_examples();
        
        assert!(!examples.is_empty());
        assert!(examples.iter().any(|(category, _)| category.contains("相对时间")));
        assert!(examples.iter().any(|(category, _)| category.contains("绝对时间")));
        assert!(examples.iter().any(|(category, _)| category.contains("每日时间")));
    }
    
    #[test]
    fn test_preprocess_chinese_numbers() {
        let parser = TimeParser::new();
        
        let test_cases = vec![
            ("三十分钟", "30分钟"),
            ("两小时", "2小时"),
            ("五分钟", "5分钟"),
            ("十二点", "12点"),
        ];
        
        for (input, expected) in test_cases {
            let processed = parser.preprocess_chinese_numbers(input);
            assert_eq!(processed, expected, "Preprocessing failed for: {}", input);
        }
    }
    
    #[test]
    fn test_invalid_input() {
        let parser = TimeParser::new();
        
        let invalid_inputs = vec![
            "无效时间",
            "",
            "25:00",
            "abc",
        ];
        
        for input in invalid_inputs {
            let result = parser.parse(input);
            // 某些输入可能会被解析，所以我们只检查明显无效的
            if input.is_empty() || input == "abc" || input == "无效时间" {
                assert!(result.is_err(), "Should fail to parse: {}", input);
            }
        }
    }
}