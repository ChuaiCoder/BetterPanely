use serde::Serialize;

/// Definition of a built-in tool
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub default_width: f64,
    pub default_height: f64,
    pub url: String,
}

/// Get all available built-in tools with localized names
pub fn get_builtin_tools(lang: &str) -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            id: "calculator".into(),
            name: crate::locales::t("tool.calculator", lang).into(),
            description: crate::locales::t("tool.calculator.desc", lang).into(),
            icon: "🔢".into(),
            default_width: 280.0,
            default_height: 420.0,
            url: "src/tools/calculator/index.html".into(),
        },
        ToolDefinition {
            id: "notes".into(),
            name: crate::locales::t("tool.notes", lang).into(),
            description: crate::locales::t("tool.notes.desc", lang).into(),
            icon: "📝".into(),
            default_width: 350.0,
            default_height: 400.0,
            url: "src/tools/notes/index.html".into(),
        },
        ToolDefinition {
            id: "timer".into(),
            name: crate::locales::t("tool.timer", lang).into(),
            description: crate::locales::t("tool.timer.desc", lang).into(),
            icon: "⏱️".into(),
            default_width: 300.0,
            default_height: 200.0,
            url: "src/tools/timer/index.html".into(),
        },
        ToolDefinition {
            id: "weather".into(),
            name: crate::locales::t("tool.weather", lang).into(),
            description: crate::locales::t("tool.weather.desc", lang).into(),
            icon: "🌤️".into(),
            default_width: 300.0,
            default_height: 350.0,
            url: "src/tools/weather/index.html".into(),
        },
    ]
}

