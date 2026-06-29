/// Get a localized string for the given key and language.
/// Falls back to English for unknown keys.
pub fn t<'a>(key: &'a str, lang: &str) -> &'a str {
    match lang {
        "zh" => zh(key),
        _ => en(key),
    }
}

fn en(key: &str) -> &str {
    match key {
        "menu.new_panel" => "New Panel",
        "menu.calculator" => "Calculator",
        "menu.notes" => "Notes",
        "menu.timer" => "Timer",
        "menu.weather" => "Weather",
        "menu.show_main" => "Show Main Window",
        "menu.settings" => "Settings",
        "menu.language" => "Language",
        "menu.lang_en" => "English",
        "menu.lang_zh" => "中文",
        "menu.quit" => "Quit",
        "menu.separator" => "─────────",
        "tray.tooltip" => "BetterPanely",

        "tool.calculator" => "Calculator",
        "tool.calculator.desc" => "Standard and scientific calculator",
        "tool.notes" => "Notes",
        "tool.notes.desc" => "Rich text notes with auto-save",
        "tool.timer" => "Timer",
        "tool.timer.desc" => "Countdown timer and stopwatch",
        "tool.weather" => "Weather",
        "tool.weather.desc" => "Current weather information",

        "incompatibility.uwp" => "UWP apps cannot be captured",
        "incompatibility.shell" => "System shell window",
        "incompatibility.self" => "BetterPanely window",
        "incompatibility.child" => "Already a child window",
        "incompatibility.elevated" => "Administrator process (UIPI blocked)",
        "incompatibility.fullscreen" => "Fullscreen / DirectX application",

        _ => key,
    }
}

fn zh(key: &str) -> &str {
    match key {
        "menu.new_panel" => "新建面板",
        "menu.calculator" => "计算器",
        "menu.notes" => "便签",
        "menu.timer" => "计时器",
        "menu.weather" => "天气",
        "menu.show_main" => "显示主窗口",
        "menu.settings" => "设置",
        "menu.language" => "语言",
        "menu.lang_en" => "English",
        "menu.lang_zh" => "中文",
        "menu.quit" => "退出",
        "menu.separator" => "─────────",
        "tray.tooltip" => "BetterPanely",

        "tool.calculator" => "计算器",
        "tool.calculator.desc" => "标准和科学计算器",
        "tool.notes" => "便签",
        "tool.notes.desc" => "富文本便签，自动保存",
        "tool.timer" => "计时器",
        "tool.timer.desc" => "倒计时和秒表",
        "tool.weather" => "天气",
        "tool.weather.desc" => "当前天气信息",

        "incompatibility.uwp" => "UWP 应用无法捕获",
        "incompatibility.shell" => "系统外壳窗口",
        "incompatibility.self" => "BetterPanely 自身窗口",
        "incompatibility.child" => "已是子窗口",
        "incompatibility.elevated" => "管理员进程 (UIPI 阻止)",
        "incompatibility.fullscreen" => "全屏/DirectX 应用",

        _ => key,
    }
}
