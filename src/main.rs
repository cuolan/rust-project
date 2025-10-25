use std::collections::HashMap;
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Word {
    english: String,
    chinese: String,
    group: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WordTable {
    name: String,
    words: Vec<Word>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FlashMemory {
    groups: HashMap<String, Vec<WordTable>>,
}

impl FlashMemory {
    fn new() -> Self {
        FlashMemory { groups: HashMap::new() }
    }

    fn add_word(&mut self, word: Word) {
        // 保持向后兼容，暂时不使用
    }

    fn create_group_if_absent(&mut self, group: &str) {
        self.groups.entry(group.to_string()).or_insert_with(Vec::new);
    }

    fn create_word_table(&mut self, group: &str, table_name: &str) {
        let tables = self.groups.entry(group.to_string()).or_insert_with(Vec::new);
        let mut name = table_name.to_string();
        let mut idx = 1;
        while tables.iter().any(|t| t.name == name) {
            idx += 1;
            name = format!("{}{}", table_name, idx);
        }
        tables.push(WordTable {
            name,
            words: Vec::new(),
        });
    }

    fn rename_group(&mut self, old: &str, new: &str) -> Result<(), &'static str> {
        if old == new { return Ok(()); }
        if self.groups.contains_key(new) { return Err("分组名已存在"); }
        if let Some(tables) = self.groups.remove(old) {
            self.groups.insert(new.to_string(), tables);
            Ok(())
        } else {
            Err("原分组不存在")
        }
    }

    fn rename_word_table(&mut self, group: &str, old_name: &str, new_name: &str) -> Result<(), &'static str> {
        if old_name == new_name { return Ok(()); }
        if let Some(tables) = self.groups.get_mut(group) {
            if tables.iter().any(|t| t.name == new_name) {
                return Err("单词表名已存在");
            }
            if let Some(table) = tables.iter_mut().find(|t| t.name == old_name) {
                table.name = new_name.to_string();
                Ok(())
            } else {
                Err("单词表不存在")
            }
        } else {
            Err("分组不存在")
        }
    }

    fn delete_group(&mut self, group: &str) -> Result<(), &'static str> {
        if self.groups.remove(group).is_some() {
            Ok(())
        } else {
            Err("分组不存在")
        }
    }

    fn delete_word_table(&mut self, group: &str, table_name: &str) -> Result<(), &'static str> {
        if let Some(tables) = self.groups.get_mut(group) {
            if let Some(pos) = tables.iter().position(|t| t.name == table_name) {
                tables.remove(pos);
                Ok(())
            } else {
                Err("单词表不存在")
            }
        } else {
            Err("分组不存在")
        }
    }

    fn get_groups(&self) -> Vec<&String> { self.groups.keys().collect() }

    fn get_word_tables_in_group(&self, group: &str) -> Option<&Vec<WordTable>> { 
        self.groups.get(group) 
    }

    fn get_words_in_group(&self, group: &str) -> Option<&Vec<Word>> { 
        // 保持向后兼容，暂时返回None
        None
    }
    
    fn add_words_to_table(&mut self, group: &str, table_name: &str, words: Vec<Word>) -> Result<(), &'static str> {
        if let Some(tables) = self.groups.get_mut(group) {
            if let Some(table) = tables.iter_mut().find(|t| t.name == table_name) {
                // 修改逻辑：覆盖原有内容，而非追加
                table.words = words;
                Ok(())
            } else {
                Err("单词表不存在")
            }
        } else {
            Err("分组不存在")
        }
    }

    fn save_to_file(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(filename, json)?;
        Ok(())
    }

    fn load_from_file(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(filename)?;
        let flash_memory: FlashMemory = serde_json::from_str(&content)?;
        Ok(flash_memory)
    }
}

struct FlashMemoryApp {
    flash_memory: FlashMemory,
    current_group: Option<String>,
    current_word_table: Option<String>,
    
    // 消息显示
    message: String,
    message_timer: f32,

    // 分组重命名
    renaming_group_active: bool,
    renaming_input: String,

    // 单词表重命名
    renaming_word_table_active: bool,
    renaming_word_table_input: String,

    // 右键菜单状态
    context_menu_group: Option<String>,
    show_context_menu: bool,
    context_menu_pos: egui::Pos2,
    
    // 单词表右键菜单状态
    context_menu_word_table: Option<(String, String)>, // (group, word_table)
    show_word_table_context_menu: bool,
    word_table_context_menu_pos: egui::Pos2,
    
    // 可拖动分界线的宽度
    sidebar_width: f32,
    
    // 单词表编辑相关
    editing_word_table: Option<(String, String)>, // (group, word_table)
    word_table_content: String,
    
    // 新建单词表相关
    creating_new_word_table: Option<String>, // 正在为哪个分组创建新单词表
    new_word_table_name: String, // 新单词表名称输入框
    
    // 闪记系统相关
    flash_mode: FlashMode, // 闪记模式状态
    current_page: usize, // 当前页码
    words_per_page: usize, // 每页显示的单词数量

    // 闪记计时与索引
    flash_index: usize,      // 当前显示的单词索引
    flash_timer: f32,        // 用于2秒切换的计时器
    countdown_remaining: i32, // 321倒计时剩余秒数（0表示结束）
    countdown_timer: f32,     // 倒计时计时器

    // 真实时间计时
    last_tick: std::time::Instant,
}

#[derive(Debug, Clone, PartialEq)]
enum FlashMode {
    Preview,  // 预览模式
    Started,  // 开始闪记
    Paused,   // 暂停
}

impl Default for FlashMemoryApp {
    fn default() -> Self {
        let flash_memory = FlashMemory::load_from_file("words.json").unwrap_or_else(|_| FlashMemory::new());
        Self {
            flash_memory,
            current_group: None,
            current_word_table: None,
            message: String::new(),
            message_timer: 0.0,
            renaming_group_active: false,
            renaming_input: String::new(),
            renaming_word_table_active: false,
            renaming_word_table_input: String::new(),
            context_menu_group: None,
            show_context_menu: false,
            context_menu_pos: egui::Pos2::ZERO,
            context_menu_word_table: None,
            show_word_table_context_menu: false,
            word_table_context_menu_pos: egui::Pos2::ZERO,
            
            sidebar_width: 150.0,
            
            editing_word_table: None,
            word_table_content: String::new(),
            
            creating_new_word_table: None,
            new_word_table_name: String::new(),
            
            flash_mode: FlashMode::Preview,
            current_page: 0,
            words_per_page: 20, // 两列每列10个单词

            flash_index: 0,
            flash_timer: 0.0,
            countdown_remaining: 0,
            countdown_timer: 0.0,
            last_tick: std::time::Instant::now(),
        }
    }
}

impl eframe::App for FlashMemoryApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 更新消息计时器
        if self.message_timer > 0.0 {
            self.message_timer -= ctx.input(|i| i.unstable_dt);
            if self.message_timer <= 0.0 { self.message.clear(); }
        }

        // 全局文字样式
        let mut style = (*ctx.style()).clone();
        style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::new(20.0, egui::FontFamily::Proportional));
        style.text_styles.insert(egui::TextStyle::Body, egui::FontId::new(16.0, egui::FontFamily::Proportional));
        style.text_styles.insert(egui::TextStyle::Button, egui::FontId::new(14.0, egui::FontFamily::Proportional));
        ctx.set_style(style);

        // 左侧目录侧栏 - 使用可拖动的宽度
        egui::SidePanel::left("sidebar")
            .min_width(150.0)
            .max_width(300.0)
            .default_width(self.sidebar_width)
            .resizable(true)
            .show(ctx, |ui| {
            // 顶部"目录"标题 - 悬停时直接显示为加号
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let resp = ui.allocate_response(egui::Vec2::new(60.0, 20.0), egui::Sense::click_and_drag());
                
                if resp.hovered() {
                    // 悬停时显示加号
                    ui.painter().text(
                        resp.rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "+",
                        egui::FontId::proportional(18.0),
                        egui::Color32::DARK_BLUE,
                    );
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                    
                    // 点击创建新分组
                    if resp.clicked() {
                        let base = "新分组";
                        let mut name = base.to_string();
                        let mut idx = 1;
                        while self.flash_memory.groups.contains_key(&name) {
                            idx += 1;
                            name = format!("{}{}", base, idx);
                        }
                        self.flash_memory.create_group_if_absent(&name);
                        self.current_group = Some(name.clone());
                        self.renaming_group_active = true;
                        self.renaming_input = name;
                        self.show_message("已创建新分组，可直接重命名");
                        self.auto_save(); // 自动保存
                    }
                } else {
                    // 正常状态显示"目录"
                    ui.painter().text(
                        resp.rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "目录",
                        egui::FontId::proportional(16.0),
                        egui::Color32::BLACK,
                    );
                }
            });
            ui.separator();

            // 分组列表（垂直）
            let mut groups: Vec<String> = self.flash_memory.get_groups().into_iter().map(|s| s.to_string()).collect();
            groups.sort();
            for g in groups {
                let selected = self.current_group.as_ref() == Some(&g);
                // 当前选中且处于重命名状态：显示输入框
                if selected && self.renaming_group_active {
                    let resp = ui.text_edit_singleline(&mut self.renaming_input);
                    // 按 Enter 确认重命名
                    let confirm = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    // 按 Escape 取消重命名
                    let cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
                    // 失去焦点时自动确认重命名（相当于按Enter）
                    let lost_focus = resp.lost_focus();
                    
                    if confirm || lost_focus {
                        let new_name = self.renaming_input.trim();
                        if new_name.is_empty() {
                            self.show_message("分组名不能为空");
                            self.renaming_group_active = false;
                        } else {
                            let old = g.clone();
                            match self.flash_memory.rename_group(&old, new_name) {
                                Ok(_) => {
                                    self.current_group = Some(new_name.to_string());
                                    self.renaming_group_active = false;
                                    self.show_message("分组已重命名");
                                    self.auto_save(); // 自动保存
                                }
                                Err(e) => {
                                    self.show_message(e);
                                    self.renaming_group_active = false;
                                }
                            }
                        }
                    } else if cancel {
                        self.renaming_group_active = false;
                    }
                    
                    // 请求焦点以确保输入框保持活跃状态
                    resp.request_focus();
                } else {
                    let resp = ui.selectable_label(selected, g.clone());
                    // 双击重命名分组（优先处理双击）
                    if resp.double_clicked() {
                        self.current_group = Some(g.clone());
                        self.renaming_group_active = true;
                        self.renaming_input = g.clone();
                    } else if resp.clicked() {
                        // 点击当前分组时收起，点击其他分组时展开
                        if self.current_group.as_ref() == Some(&g) {
                            // 如果点击的是当前分组，则收起
                            self.current_group = None;
                            self.current_word_table = None;
                        } else {
                            // 如果点击的是其他分组，则展开
                            self.current_group = Some(g.clone());
                            self.current_word_table = None; // 清除单词表选择
                        }
                    }
                    // 右键菜单
                    if resp.secondary_clicked() {
                        self.context_menu_group = Some(g.clone());
                        self.show_context_menu = true;
                        self.context_menu_pos = resp.interact_pointer_pos().unwrap_or_default();
                    }
                }
                
                // 显示该分组下的单词表
                 if selected {
                     let tables: Vec<WordTable> = self.flash_memory.get_word_tables_in_group(&g)
                         .map(|t| t.clone())
                         .unwrap_or_default();
                     for table in tables {
                         ui.add_space(2.0);
                         ui.horizontal(|ui| {
                             ui.add_space(20.0); // 缩进
                             let table_selected = self.current_word_table.as_ref() == Some(&table.name);
                             
                             // 单词表重命名状态
                             if table_selected && self.renaming_word_table_active {
                                 let resp = ui.text_edit_singleline(&mut self.renaming_word_table_input);
                                 
                                 // 按 Enter 确认重命名
                                 let confirm = ui.input(|i| i.key_pressed(egui::Key::Enter));
                                 // 按 Escape 取消重命名
                                 let cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
                                 // 失去焦点时自动确认重命名（相当于按Enter）
                                 let lost_focus = resp.lost_focus();
                                 
                                 if confirm || lost_focus {
                                     let new_name = self.renaming_word_table_input.trim();
                                     if new_name.is_empty() {
                                         self.show_message("单词表名不能为空");
                                         self.renaming_word_table_active = false;
                                     } else {
                                         match self.flash_memory.rename_word_table(&g, &table.name, new_name) {
                                             Ok(_) => {
                                                 self.current_word_table = Some(new_name.to_string());
                                                 self.renaming_word_table_active = false;
                                                 self.show_message("单词表已重命名");
                                                 self.auto_save(); // 自动保存
                                             }
                                             Err(e) => {
                                                 self.show_message(e);
                                                 self.renaming_word_table_active = false;
                                             }
                                         }
                                     }
                                 } else if cancel {
                                     self.renaming_word_table_active = false;
                                 }
                                 
                                 // 请求焦点以确保输入框保持活跃状态
                                 resp.request_focus();
                             } else {
                                 let resp = ui.selectable_label(table_selected, &table.name);
                                 // 双击重命名（优先处理双击）
                                 if resp.double_clicked() {
                                     self.current_word_table = Some(table.name.clone());
                                     self.renaming_word_table_active = true;
                                     self.renaming_word_table_input = table.name.clone();
                                     // 请求焦点到输入框
                                     ui.memory_mut(|mem| mem.request_focus(egui::Id::new("word_table_rename")));
                                 } else if resp.clicked() {
                                     // 只有在非双击时才处理单击
                                     self.current_word_table = Some(table.name.clone());
                                 }
                                 // 右键菜单
                                 if resp.secondary_clicked() {
                                     self.context_menu_word_table = Some((g.clone(), table.name.clone()));
                                     self.show_word_table_context_menu = true;
                                     self.word_table_context_menu_pos = resp.interact_pointer_pos().unwrap_or_default();
                                 }
                             }
                         });
                     }
                 }
                ui.add_space(4.0);
            }
        });

        // 右键上下文菜单
         if self.show_context_menu {
             egui::Area::new("context_menu".into())
                 .fixed_pos(self.context_menu_pos)
                 .order(egui::Order::Foreground)
                 .show(ctx, |ui| {
                    egui::Frame::popup(&ctx.style()).show(ui, |ui| {
                        ui.set_min_width(120.0);
                        if ui.button("新建单词表").clicked() {
                            if let Some(ref group) = self.context_menu_group {
                                self.flash_memory.create_word_table(group, "新单词表");
                                // 直接进入重命名状态
                                self.current_group = Some(group.clone());
                                self.current_word_table = Some("新单词表".to_string());
                                self.renaming_word_table_active = true;
                                self.renaming_word_table_input = "新单词表".to_string();
                                self.show_message("已创建新单词表，可直接重命名");
                                self.auto_save(); // 自动保存
                            }
                            self.show_context_menu = false;
                        }
                        if ui.button("删除分组").clicked() {
                            if let Some(ref group) = self.context_menu_group {
                                match self.flash_memory.delete_group(group) {
                                    Ok(_) => {
                                        if self.current_group.as_ref() == Some(group) {
                                            self.current_group = None;
                                            self.current_word_table = None;
                                        }
                                        self.show_message("分组已删除");
                                        self.auto_save(); // 自动保存
                                    }
                                    Err(e) => {
                                        self.show_message(e);
                                    }
                                }
                            }
                            self.show_context_menu = false;
                        }
                    });
                });
        }

        // 单词表右键菜单
        if self.show_word_table_context_menu {
            egui::Area::new("word_table_context_menu".into())
                .fixed_pos(self.word_table_context_menu_pos)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                   egui::Frame::popup(&ctx.style()).show(ui, |ui| {
                       ui.set_min_width(120.0);
                       if ui.button("修改").clicked() {
                           if let Some((group, table_name)) = &self.context_menu_word_table {
                               // 同步当前选择到该表，确保保存返回后预览显示
                               self.current_group = Some(group.clone());
                               self.current_word_table = Some(table_name.clone());
                               self.current_page = 0; // 进入修改时预览回到第一页
                               self.editing_word_table = Some((group.clone(), table_name.clone()));
                               // 预填原有单词到编辑框（格式：英文 空格 中文）
                               if let Some(tables) = self.flash_memory.get_word_tables_in_group(group) {
                                   if let Some(table) = tables.iter().find(|t| t.name == *table_name) {
                                       self.word_table_content = table.words
                                           .iter()
                                           .take(100) // 预填最多100行
                                           .map(|w| format!("{} {}", w.english, w.chinese))
                                           .collect::<Vec<_>>()
                                           .join("\n");
                                   } else {
                                       self.word_table_content.clear();
                                   }
                               } else {
                                   self.word_table_content.clear();
                               }
                           }
                           self.show_word_table_context_menu = false;
                       }
                       if ui.button("删除").clicked() {
                           if let Some((group, table_name)) = &self.context_menu_word_table {
                               match self.flash_memory.delete_word_table(group, table_name) {
                                   Ok(_) => {
                                       // 如果删除的是当前选中的单词表，清除选择
                                       if self.current_word_table.as_ref() == Some(table_name) {
                                           self.current_word_table = None;
                                       }
                                       self.show_message("单词表已删除");
                                       self.auto_save(); // 自动保存
                                   }
                                   Err(e) => {
                                       self.show_message(e);
                                   }
                               }
                           }
                           self.show_word_table_context_menu = false;
                       }
                   });
               });
        }

        // 点击其他地方关闭单词表右键菜单
        if self.show_word_table_context_menu {
            if ctx.input(|i| i.pointer.primary_clicked()) {
                // 检查是否点击在菜单区域外
                let menu_rect = egui::Rect::from_min_size(
                    self.word_table_context_menu_pos,
                    egui::Vec2::new(120.0, 60.0)
                );
                if let Some(pointer_pos) = ctx.input(|i| i.pointer.interact_pos()) {
                    if !menu_rect.contains(pointer_pos) {
                        self.show_word_table_context_menu = false;
                    }
                }
            }
        }

        // 点击其他地方关闭右键菜单
        if self.show_context_menu {
            if ctx.input(|i| i.pointer.primary_clicked()) {
                // 检查是否点击在菜单区域外
                let menu_rect = egui::Rect::from_min_size(
                    self.context_menu_pos,
                    egui::Vec2::new(120.0, 60.0)
                );
                if let Some(pointer_pos) = ctx.input(|i| i.pointer.interact_pos()) {
                    if !menu_rect.contains(pointer_pos) {
                        self.show_context_menu = false;
                    }
                }
            }
        }
        // 右侧内容区 - 移除所有旧功能，只保留基本布局
        egui::CentralPanel::default().show(ctx, |ui| {
            // 背景色轻微绿色
            let rect = ui.max_rect();
            ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(223, 238, 223));

            // 检查是否在编辑单词表
            if let Some((group, table_name)) = &self.editing_word_table.clone() {
                // 单词表编辑界面（整体可滚动，输入框保持充满内容区域）
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.add_space(10.0);
                    
                    // 按钮区域移到上方
                    ui.horizontal(|ui| {
                        if ui.button("返回").clicked() {
                            self.editing_word_table = None;
                            self.word_table_content.clear();
                        }
                        
                        ui.add_space(20.0);
                        
                        if ui.button("保存").clicked() {
                            if let Some((group, table_name)) = &self.editing_word_table.clone() {
                                // 最多处理前100行
                                let total_lines = self.word_table_content.lines().count();
                                let limited_text = self.word_table_content
                                    .lines()
                                    .take(100)
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                let words = self.parse_words_from_text(&limited_text, group);
                                
                                if total_lines > 100 {
                                    self.show_message("超过100行，仅保存前100行");
                                }
                                
                                if words.is_empty() {
                                    self.show_message("没有找到有效的单词格式");
                                } else {
                                    match self.flash_memory.add_words_to_table(group, table_name, words.clone()) {
                                        Ok(_) => {
                                            self.auto_save();
                                            self.show_message(&format!("成功保存 {} 个单词", words.len()));
                                            // 保持当前选择为刚编辑的分组和表，回到预览直接显示
                                            self.current_group = Some(group.clone());
                                            self.current_word_table = Some(table_name.clone());
                                            self.current_page = 0; // 保存后预览回到第一页，保证可见
                                            self.editing_word_table = None;
                                            self.word_table_content.clear();
                                        }
                                        Err(e) => {
                                            self.show_message(&format!("保存失败: {}", e));
                                        }
                                    }
                                }
                            }
                        }
                    });
                    
                    ui.add_space(10.0);
                    
                    // 说明文字
                    ui.small("单词和释义之间用空格分隔，换行添加新单词（最多保存前100行）:");
                    ui.add_space(5.0);
                    
                    // 编辑框高度：固定为剩余可用空间，TextEdit 自带滚动
                    let edit_height = ui.available_height().max(200.0);
                    ui.add_sized(
                        [ui.available_width(), edit_height],
                        egui::TextEdit::multiline(&mut self.word_table_content)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                    );
                });
            } else {
                // 闪记系统内容区
                ui.vertical(|ui| {
                    // 控制按钮区域
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        
                        // 开始按钮：仅预览状态可点击
                        let start_button = ui.add_enabled(
                            self.flash_mode == FlashMode::Preview,
                            egui::Button::new("开始")
                        );
                        if start_button.clicked() {
                            let total = self.get_current_words().len();
                            if total == 0 {
                                self.show_message("当前单词表为空，无法开始");
                            } else {
                                self.flash_mode = FlashMode::Started;
                                self.flash_index = 0;
                                self.flash_timer = 0.0;
                                self.countdown_remaining = 3;
                                self.countdown_timer = 0.0;
                                self.last_tick = std::time::Instant::now();
                                ctx.request_repaint();
                            }
                        }
                        
                        ui.add_space(10.0);
                        
                        // 暂停/继续按钮：在开始与暂停状态均可点击
                        let pause_label = if self.flash_mode == FlashMode::Paused { "继续" } else { "暂停" };
                        let pause_enabled = self.flash_mode == FlashMode::Started || self.flash_mode == FlashMode::Paused;
                        let pause_button = ui.add_enabled(pause_enabled, egui::Button::new(pause_label));
                        if pause_button.clicked() {
                            match self.flash_mode {
                                FlashMode::Started => {
                                    self.flash_mode = FlashMode::Paused;
                                    // 防止恢复时 dt 累计过大
                                    self.last_tick = std::time::Instant::now();
                                }
                                FlashMode::Paused => {
                                    self.flash_mode = FlashMode::Started;
                                    self.last_tick = std::time::Instant::now();
                                    // 恢复动画需要持续重绘
                                    ctx.request_repaint();
                                }
                                _ => {}
                            }
                        }
                        
                        ui.add_space(10.0);
                        
                        // 结束按钮：非预览状态可点击
                        let end_button = ui.add_enabled(
                            self.flash_mode != FlashMode::Preview,
                            egui::Button::new("结束")
                        );
                        if end_button.clicked() {
                            self.flash_mode = FlashMode::Preview;
                            self.current_page = 0;
                            self.flash_index = 0;
                            self.flash_timer = 0.0;
                            self.countdown_remaining = 0;
                            self.countdown_timer = 0.0;
                            self.last_tick = std::time::Instant::now();
                        }
                        

                    });
                    
                    ui.separator();
                    ui.add_space(10.0);
                    
                    // 单词显示区域
                    if let Some(ref table_name) = self.current_word_table {
                        let all_words = self.get_current_words();
                        if all_words.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(50.0);
                                ui.label("该单词表为空");
                                ui.add_space(20.0);
                                ui.label("请添加单词到此表中");
                            });
                        } else {
                            match self.flash_mode {
                                FlashMode::Preview => {
                                    // 预览模式：展示全部单词表格
                                    egui::ScrollArea::vertical().show(ui, |ui| {
                                        egui_extras::TableBuilder::new(ui)
                                            .striped(true)
                                            .resizable(false)
                                            .column(egui_extras::Column::remainder())
                                            .column(egui_extras::Column::remainder())
                                            .body(|mut body| {
                                                let font_size: f32 = 18.0;
                                                let row_height: f32 = (font_size + 14.0_f32).max(32.0_f32);
                                                let row_count = all_words.len();
                                                body.rows(row_height, row_count, |mut row| {
                                                    let idx = row.index();
                                                    if let Some(word) = all_words.get(idx) {
                                                        row.col(|ui| {
                                                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                                                ui.label(egui::RichText::new(&word.english).size(font_size).strong());
                                                            });
                                                        });
                                                        row.col(|ui| {
                                                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                                                ui.label(egui::RichText::new(&word.chinese).size(font_size));
                                                            });
                                                        });
                                                    }
                                                });
                                            });
                                    });
                                }
                                _ => {
                                    // 闪记模式：321倒计时 -> 每1.5秒切换一个单词（前1秒只显示英文，后0.5秒显示中文；到末尾自动结束，不循环）
                                    if self.flash_mode == FlashMode::Started {
                                        // 使用系统时间推进动画，并请求持续重绘
                                        let now = std::time::Instant::now();
                                        let dt = (now - self.last_tick).as_secs_f32();
                                        self.last_tick = now;
                                        ctx.request_repaint();
                                    
                                        if self.countdown_remaining > 0 {
                                            self.countdown_timer += dt;
                                            if self.countdown_timer >= 1.0 {
                                                self.countdown_remaining -= 1;
                                                self.countdown_timer = 0.0;
                                                // 倒计时刚结束时，确保词计时器从0开始，避免释义提前出现
                                                if self.countdown_remaining == 0 {
                                                    self.flash_timer = 0.0;
                                                }
                                            }
                                        } else {
                                            self.flash_timer += dt;
                                            if self.flash_timer >= 2.0 {
                                                self.flash_timer = 0.0;
                                                if self.flash_index + 1 < all_words.len() {
                                                    self.flash_index += 1;
                                                } else {
                                                    // 播放到最后一个词后自动结束（返回预览，不循环）
                                                    self.flash_mode = FlashMode::Preview;
                                                    self.current_page = 0;
                                                    self.show_message("本轮学习结束");
                                                }
                                            }
                                        }
                                    }
                                    
                                    let english_size: f32 = 64.0;
                                    let chinese_size: f32 = 32.0;
                                    let card_height = ui.available_height() - 20.0;
                                    let show_meaning = self.countdown_remaining == 0 && self.flash_timer >= 1.0; // 前1秒只英文，后1秒显示中文
                                    // 始终按“英文+预留释义”计算内容高度，避免释义出现导致整体向上/向下位移
                                    let content_height = if self.countdown_remaining > 0 {
                                        english_size + 20.0
                                    } else {
                                        english_size + chinese_size + 24.0
                                    };
                                    let top_space = (card_height - content_height).max(0.0) / 2.0;
                                    
                                    egui::Frame::none()
                                        .stroke(egui::Stroke::new(2.0, egui::Color32::BLACK))
                                        .fill(egui::Color32::WHITE)
                                        .show(ui, |ui| {
                                            ui.set_min_size(egui::Vec2::new(ui.available_width(), card_height));
                                            ui.add_space(top_space);
                                            ui.vertical_centered(|ui| {
                                                if self.countdown_remaining > 0 {
                                                    ui.label(egui::RichText::new(format!("{}", self.countdown_remaining)).size(english_size).strong());
                                                } else {
                                                    let word = &all_words[self.flash_index];
                                                    ui.label(egui::RichText::new(&word.english).size(english_size).strong());
                                                    // 始终预留英文到释义的间距与释义高度，避免出现瞬间位移
                                                    ui.add_space(12.0);
                                                    if show_meaning {
                                                        ui.label(egui::RichText::new(&word.chinese).size(chinese_size));
                                                    } else {
                                                        // 不显示释义时，预留同等高度空间以稳定布局
                                                        ui.add_space(chinese_size);
                                                    }
                                                }
                                            });
                                        });
                                }
                            }
                        }
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.add_space(50.0);
                            ui.heading("闪记系统");
                            ui.add_space(20.0);
                            
                            if !self.message.is_empty() {
                                ui.colored_label(egui::Color32::DARK_GREEN, &self.message);
                                ui.add_space(20.0);
                            }
                            
                            if let Some(ref group) = self.current_group {
                                ui.label(format!("当前分组: {}", group));
                                ui.add_space(10.0);
                                ui.label("请选择左侧单词表开始学习");
                            } else {
                                ui.label("请选择左侧分组和单词表");
                            }
                        });
                    }
                });
            }
        });
    }
}

impl FlashMemoryApp {
    fn show_message(&mut self, message: &str) {
        self.message = message.to_string();
        self.message_timer = 3.0; // 显示3秒
    }
    
    fn auto_save(&mut self) {
        match self.flash_memory.save_to_file("words.json") {
            Ok(_) => {
                // 保存成功，不显示消息以避免干扰用户
            }
            Err(e) => {
                self.show_message(&format!("保存失败: {}", e));
            }
        }
    }
    
    fn get_current_words(&self) -> Vec<Word> {
        if let (Some(group), Some(table_name)) = (&self.current_group, &self.current_word_table) {
            if let Some(tables) = self.flash_memory.get_word_tables_in_group(group) {
                if let Some(table) = tables.iter().find(|t| t.name == *table_name) {
                    return table.words.clone();
                }
            }
        }
        Vec::new()
    }
    
    fn get_total_pages(&self) -> usize {
        let words = self.get_current_words();
        if words.is_empty() {
            0
        } else {
            (words.len() + self.words_per_page - 1) / self.words_per_page
        }
    }
    
    fn get_current_page_words(&self) -> Vec<Word> {
        let words = self.get_current_words();
        let start = self.current_page * self.words_per_page;
        let end = std::cmp::min(start + self.words_per_page, words.len());
        if start < words.len() {
            words[start..end].to_vec()
        } else {
            Vec::new()
        }
    }
    
    fn parse_words_from_text(&self, text: &str, group: &str) -> Vec<Word> {
        let mut words = Vec::new();
        
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            // 查找第一个空格的位置
            if let Some(space_pos) = line.find(' ') {
                let english = line[..space_pos].trim().to_string();
                let chinese = line[space_pos + 1..].trim().to_string();
                
                if !english.is_empty() && !chinese.is_empty() {
                    words.push(Word {
                        english,
                        chinese,
                        group: group.to_string(),
                    });
                }
            }
        }
        
        words
    }
}

// 尝试加载系统中文字体（simhei/simkai），用于支持中文显示
fn configure_chinese_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let mut loaded = false;
    for path in [
        "C\\Windows\\Fonts\\simhei.ttf",
        "C\\Windows\\Fonts\\simkai.ttf",
        "C:/Windows/Fonts/simhei.ttf",
        "C:/Windows/Fonts/simkai.ttf",
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert("cjk".to_owned(), egui::FontData::from_owned(bytes));
            fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "cjk".to_owned());
            fonts.families.entry(egui::FontFamily::Monospace).or_default().insert(0, "cjk".to_owned());
            loaded = true;
            break;
        }
    }
    if loaded { ctx.set_fonts(fonts); } else { ctx.set_fonts(fonts); }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 620.0]).with_title("单词闪记系统"),
        ..Default::default()
    };
    eframe::run_native(
        "单词闪记系统",
        options,
        Box::new(|cc| { configure_chinese_fonts(&cc.egui_ctx); Ok(Box::new(FlashMemoryApp::default())) }),
    )
}