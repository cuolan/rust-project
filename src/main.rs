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
                               self.editing_word_table = Some((group.clone(), table_name.clone()));
                               // 清空输入框内容
                               self.word_table_content = String::new();
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
                // 单词表编辑界面
                ui.vertical(|ui| {
                    ui.add_space(10.0);
                    
                    // 按钮区域移到上方
                    ui.horizontal(|ui| {
                        if ui.button("返回").clicked() {
                            self.editing_word_table = None;
                            self.word_table_content.clear();
                        }
                        
                        ui.add_space(20.0);
                        
                        if ui.button("保存").clicked() {
                            // TODO: 实现保存功能
                            self.show_message("保存功能待实现");
                            self.editing_word_table = None;
                            self.word_table_content.clear();
                        }
                    });
                    
                    ui.add_space(10.0);
                    
                    // 调小字体的说明文字
                    ui.small("单词和释义之间用空格分隔，换行添加新单词:");
                    ui.add_space(5.0);
                    
                    let available_height = ui.available_height() - 20.0; // 调整高度计算
                    ui.add_sized(
                        [ui.available_width(), available_height],
                        egui::TextEdit::multiline(&mut self.word_table_content)
                            .font(egui::TextStyle::Monospace)
                    );
                });
            } else {
                // 默认内容区
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.heading("内容区域");
                    ui.add_space(20.0);
                    
                    if !self.message.is_empty() {
                        ui.colored_label(egui::Color32::DARK_GREEN, &self.message);
                    }
                    
                    ui.add_space(20.0);
                    if let Some(ref group) = self.current_group {
                        ui.label(format!("当前选中分组: {}", group));
                    } else {
                        ui.label("请选择左侧分组");
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