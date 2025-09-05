//! UI主题模块
//! 
//! 定义应用程序的视觉主题，包括颜色、字体、样式等

use iced::{Color, Background};
use serde::{Deserialize, Serialize};

/// 应用主题
/// 
/// 定义应用程序的整体视觉风格
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    /// 主题名称
    pub name: String,
    /// 主题类型
    pub theme_type: ThemeType,
    /// 颜色配置
    pub colors: ThemeColors,
    /// 字体配置
    pub fonts: ThemeFonts,
    /// 间距配置
    pub spacing: ThemeSpacing,
    /// 圆角配置
    pub rounding: ThemeRounding,
}

/// 主题类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThemeType {
    /// 浅色主题
    Light,
    /// 深色主题
    Dark,
    /// 自动主题（跟随系统）
    Auto,
}

/// 主题颜色配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeColors {
    /// 主要背景色
    pub background: [u8; 3],
    /// 次要背景色
    pub background_secondary: [u8; 3],
    /// 主要文本色
    pub text: [u8; 3],
    /// 次要文本色
    pub text_secondary: [u8; 3],
    /// 主色调
    pub primary: [u8; 3],
    /// 次要色调
    pub secondary: [u8; 3],
    /// 成功色
    pub success: [u8; 3],
    /// 警告色
    pub warning: [u8; 3],
    /// 错误色
    pub error: [u8; 3],
    /// 边框色
    pub border: [u8; 3],
    /// 阴影色
    pub shadow: [u8; 3],
}

/// 字体配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeFonts {
    /// 默认字体大小
    pub default_size: f32,
    /// 标题字体大小
    pub heading_size: f32,
    /// 小字体大小
    pub small_size: f32,
    /// 字体族
    pub family: String,
}

/// 间距配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeSpacing {
    /// 小间距
    pub small: f32,
    /// 中等间距
    pub medium: f32,
    /// 大间距
    pub large: f32,
    /// 按钮内边距
    pub button_padding: f32,
    /// 输入框内边距
    pub input_padding: f32,
}

/// 圆角配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeRounding {
    /// 小圆角
    pub small: f32,
    /// 中等圆角
    pub medium: f32,
    /// 大圆角
    pub large: f32,
    /// 按钮圆角
    pub button: f32,
    /// 输入框圆角
    pub input: f32,
}

impl Default for Theme {
    /// 创建默认主题（浅色主题）
    fn default() -> Self {
        Self::light_theme()
    }
}

impl Theme {
    /// 创建浅色主题
    /// 
    /// # 返回值
    /// 
    /// 返回配置好的浅色主题
    pub fn light_theme() -> Self {
        Self {
            name: "Light".to_string(),
            theme_type: ThemeType::Light,
            colors: ThemeColors {
                background: [248, 249, 250],
                background_secondary: [255, 255, 255],
                text: [33, 37, 41],
                text_secondary: [108, 117, 125],
                primary: [13, 110, 253],
                secondary: [108, 117, 125],
                success: [25, 135, 84],
                warning: [255, 193, 7],
                error: [220, 53, 69],
                border: [222, 226, 230],
                shadow: [0, 0, 0],
            },
            fonts: ThemeFonts {
                default_size: 14.0,
                heading_size: 24.0,
                small_size: 12.0,
                family: "Microsoft YaHei".to_string(),
            },
            spacing: ThemeSpacing {
                small: 4.0,
                medium: 8.0,
                large: 16.0,
                button_padding: 10.0,
                input_padding: 8.0,
            },
            rounding: ThemeRounding {
                small: 2.0,
                medium: 4.0,
                large: 8.0,
                button: 4.0,
                input: 4.0,
            },
        }
    }

    /// 创建深色主题
    /// 
    /// # 返回值
    /// 
    /// 返回配置好的深色主题
    pub fn dark_theme() -> Self {
        Self {
            name: "Dark".to_string(),
            theme_type: ThemeType::Dark,
            colors: ThemeColors {
                background: [33, 37, 41],
                background_secondary: [52, 58, 64],
                text: [248, 249, 250],
                text_secondary: [173, 181, 189],
                primary: [13, 110, 253],
                secondary: [108, 117, 125],
                success: [25, 135, 84],
                warning: [255, 193, 7],
                error: [220, 53, 69],
                border: [73, 80, 87],
                shadow: [0, 0, 0],
            },
            fonts: ThemeFonts {
                default_size: 14.0,
                heading_size: 24.0,
                small_size: 12.0,
                family: "Microsoft YaHei".to_string(),
            },
            spacing: ThemeSpacing {
                small: 4.0,
                medium: 8.0,
                large: 16.0,
                button_padding: 10.0,
                input_padding: 8.0,
            },
            rounding: ThemeRounding {
                small: 2.0,
                medium: 4.0,
                large: 8.0,
                button: 4.0,
                input: 4.0,
            },
        }
    }

    /// 获取背景颜色
    /// 
    /// # 返回值
    /// 
    /// 返回iced Color格式的背景颜色
    pub fn background_color(&self) -> Color {
        let [r, g, b] = self.colors.background;
        Color::from_rgb8(r, g, b)
    }

    /// 获取文本颜色
    /// 
    /// # 返回值
    /// 
    /// 返回iced Color格式的文本颜色
    pub fn text_color(&self) -> Color {
        let [r, g, b] = self.colors.text;
        Color::from_rgb8(r, g, b)
    }

    /// 获取主色调
    /// 
    /// # 返回值
    /// 
    /// 返回iced Color格式的主色调
    pub fn primary_color(&self) -> Color {
        let [r, g, b] = self.colors.primary;
        Color::from_rgb8(r, g, b)
    }

    /// 获取错误颜色
    /// 
    /// # 返回值
    /// 
    /// 返回iced Color格式的错误颜色
    pub fn error_color(&self) -> Color {
        let [r, g, b] = self.colors.error;
        Color::from_rgb8(r, g, b)
    }

    /// 获取成功颜色
    /// 
    /// # 返回值
    /// 
    /// 返回iced Color格式的成功颜色
    pub fn success_color(&self) -> Color {
        let [r, g, b] = self.colors.success;
        Color::from_rgb8(r, g, b)
    }

    /// 获取警告颜色
    /// 
    /// # 返回值
    /// 
    /// 返回iced Color格式的警告颜色
    pub fn warning_color(&self) -> Color {
        let [r, g, b] = self.colors.warning;
        Color::from_rgb8(r, g, b)
    }

    /// 获取边框颜色
    /// 
    /// # 返回值
    /// 
    /// 返回iced Color格式的边框颜色
    pub fn border_color(&self) -> Color {
        let [r, g, b] = self.colors.border;
        Color::from_rgb8(r, g, b)
    }

    /// 获取背景
    /// 
    /// # 返回值
    /// 
    /// 返回iced Background格式的背景
    pub fn background(&self) -> Background {
        Background::Color(self.background_color())
    }

    /// 检测系统主题
    /// 
    /// # 返回值
    /// 
    /// 返回检测到的主题类型
    pub fn detect_system_theme() -> ThemeType {
        // 在Windows上检测系统主题
        // 这里可以通过注册表或其他方式检测
        // 暂时返回浅色主题作为默认值
        ThemeType::Light
    }

    /// 应用主题到应用程序
    /// 
    /// # 参数
    /// 
    /// * `theme_type` - 要应用的主题类型
    /// 
    /// # 返回值
    /// 
    /// 返回对应的主题实例
    pub fn apply_theme(theme_type: ThemeType) -> Self {
        match theme_type {
            ThemeType::Light => Self::light_theme(),
            ThemeType::Dark => Self::dark_theme(),
            ThemeType::Auto => {
                let detected = Self::detect_system_theme();
                Self::apply_theme(detected)
            }
        }
    }

    /// 切换主题
    /// 
    /// # 参数
    /// 
    /// * `current_theme` - 当前主题类型
    /// 
    /// # 返回值
    /// 
    /// 返回切换后的主题
    pub fn toggle_theme(current_theme: ThemeType) -> Self {
        match current_theme {
            ThemeType::Light => Self::dark_theme(),
            ThemeType::Dark => Self::light_theme(),
            ThemeType::Auto => Self::light_theme(), // 自动主题切换为浅色
        }
    }

    /// 保存主题设置
    /// 
    /// # 参数
    /// 
    /// * `theme` - 要保存的主题
    /// 
    /// # 返回值
    /// 
    /// 保存操作的结果
    pub fn save_theme_settings(theme: &Theme) -> Result<(), Box<dyn std::error::Error>> {
        // 这里可以实现主题设置的持久化
        // 比如保存到配置文件或注册表
        let config_dir = dirs::config_dir()
            .ok_or("无法获取配置目录")?;
        
        let app_config_dir = config_dir.join("QtShut");
        std::fs::create_dir_all(&app_config_dir)?;
        
        let theme_file = app_config_dir.join("theme.json");
        let theme_json = serde_json::to_string_pretty(theme)?;
        std::fs::write(theme_file, theme_json)?;
        
        Ok(())
    }

    /// 加载主题设置
    /// 
    /// # 返回值
    /// 
    /// 返回加载的主题或默认主题
    pub fn load_theme_settings() -> Self {
        let config_dir = match dirs::config_dir() {
            Some(dir) => dir,
            None => return Self::default(),
        };
        
        let theme_file = config_dir.join("QtShut").join("theme.json");
        
        match std::fs::read_to_string(theme_file) {
            Ok(content) => {
                match serde_json::from_str::<Theme>(&content) {
                    Ok(theme) => theme,
                    Err(_) => Self::default(),
                }
            },
            Err(_) => Self::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_creation() {
        let light_theme = Theme::light_theme();
        assert_eq!(light_theme.theme_type, ThemeType::Light);
        assert_eq!(light_theme.name, "Light");

        let dark_theme = Theme::dark_theme();
        assert_eq!(dark_theme.theme_type, ThemeType::Dark);
        assert_eq!(dark_theme.name, "Dark");
    }

    #[test]
    fn test_color_conversion() {
        let theme = Theme::light_theme();
        let bg_color = theme.background_color();
        let text_color = theme.text_color();
        
        // 测试颜色转换是否正确
        assert_eq!(bg_color.r, 248.0 / 255.0);
        assert_eq!(text_color.r, 33.0 / 255.0);
    }

    #[test]
    fn test_theme_toggle() {
        let light_theme = Theme::toggle_theme(ThemeType::Dark);
        assert_eq!(light_theme.theme_type, ThemeType::Light);

        let dark_theme = Theme::toggle_theme(ThemeType::Light);
        assert_eq!(dark_theme.theme_type, ThemeType::Dark);
    }

    #[test]
    fn test_theme_serialization() {
        let theme = Theme::light_theme();
        let serialized = serde_json::to_string(&theme);
        assert!(serialized.is_ok());

        let deserialized: Result<Theme, _> = serde_json::from_str(&serialized.unwrap());
        assert!(deserialized.is_ok());
        assert_eq!(deserialized.unwrap().name, theme.name);
    }
}