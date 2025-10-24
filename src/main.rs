use std::collections::HashMap;
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Word {
    english: String,
    chinese: String,
    group: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FlashMemory {
    groups: HashMap<String, Vec<Word>>,
}

impl FlashMemory {
    fn new() -> Self {
        FlashMemory {
            groups: HashMap::new(),
        }
    }

    fn add_word(&mut self, word: Word) {
        self.groups
            .entry(word.group.clone())
            .or_insert_with(Vec::new)
            .push(word);
    }

    fn get_groups(&self) -> Vec<&String> {
        self.groups.keys().collect()
    }

    fn get_words_in_group(&self, group: &str) -> Option<&Vec<Word>> {
        self.groups.get(group)
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
    
    // UI状态
    show_add_word_dialog: bool,
    new_english: String,
    new_chinese: String,
    new_group: String,
    
    // 闪记状态
    is_flashing: bool,
    current_word_index: usize,
    show_meaning: bool,
    
    // 鼠标悬停状态
    hovered_word: Option<usize>,
    
    // 消息显示
    message: String,
    message_timer: f32,
}

impl Default for FlashMemoryApp {
    fn default() -> Self {
        let flash_memory = FlashMemory::load_from_file("words.json")
            .unwrap_or_else(|_| FlashMemory::new());
        
        Self {
            flash_memory,
            current_group: None,
            show_add_word_dialog: false,
            new_english: String::new(),
            new_chinese: String::new(),
            new_group: String::new(),
            is_flashing: false,
            current_word_index: 0,
            show_meaning: false,
            hovered_word: None,
            message: String::new(),
            message_timer: 0.0,
        }
    }
}

impl eframe::App for FlashMemoryApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 更新消息计时器
        if self.message_timer > 0.0 {
            self.message_timer -= ctx.input(|i| i.unstable_dt);
            if self.message_timer <= 0.0 {
                self.message.clear();
            }
        }

        // 主面板
        egui::CentralPanel::default().show(ctx, |ui| {
            // 设置字体大小
            let mut style = (*ctx.style()).clone();
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::new(24.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(16.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::new(14.0, egui::FontFamily::Proportional),
            );
            ctx.set_style(style);

            // 标题
            ui.heading("单词闪记系统");
            ui.separator();

            // 显示消息
            if !self.message.is_empty() {
                ui.colored_label(egui::Color32::DARK_GREEN, &self.message);
                ui.separator();
            }

            // 分组选择区域
            let groups = self.flash_memory.get_groups();
            let groups_clone: Vec<String> = groups.iter().map(|s| s.to_string()).collect();
            ui.horizontal(|ui| {
                ui.label("分组:");
                
                let has_groups = !groups_clone.is_empty();
                if has_groups {
                    for group in &groups_clone {
                        let is_selected = self.current_group.as_ref() == Some(group);
                        if ui.selectable_label(is_selected, group.as_str()).clicked() {
                            self.current_group = Some(group.to_string());
                        }
                    }
                } else {
                    ui.label("暂无分组");
                }
            });

            ui.separator();

            // 操作按钮
            ui.horizontal(|ui| {
                if ui.button("添加单词").clicked() {
                    self.show_add_word_dialog = true;
                }
                
                if ui.button("开始闪记").clicked() {
                    if let Some(group) = &self.current_group {
                        if let Some(words) = self.flash_memory.get_words_in_group(group) {
                            if !words.is_empty() {
                                self.is_flashing = true;
                                self.current_word_index = 0;
                                self.show_meaning = false;
                            } else {
                                self.show_message("该分组没有单词");
                            }
                        }
                    } else {
                        self.show_message("请先选择分组");
                    }
                }
                
                if ui.button("保存数据").clicked() {
                    match self.flash_memory.save_to_file("words.json") {
                        Ok(_) => self.show_message("数据已保存到 words.json"),
                        Err(e) => self.show_message(&format!("保存失败: {}", e)),
                    }
                }
            });

            ui.separator();

            // 显示当前分组的单词
            if let Some(ref group) = self.current_group {
                if let Some(words) = self.flash_memory.get_words_in_group(group) {
                    ui.label(format!("分组 '{}' 中的单词:", group));
                    ui.separator();

                    // 添加表头
                    ui.horizontal(|ui| {
                        ui.label("英文单词");
                        ui.separator();
                        ui.label("中文意思");
                    });
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (i, word) in words.iter().enumerate() {
                            ui.horizontal(|ui| {
                                // 英文单词列
                                let english_response = ui.allocate_response(
                                    egui::Vec2::new(200.0, 25.0),
                                    egui::Sense::hover()
                                );
                                
                                // 中文意思列
                                let chinese_response = ui.allocate_response(
                                    egui::Vec2::new(200.0, 25.0),
                                    egui::Sense::hover()
                                );

                                // 检查鼠标悬停
                                let is_hovered = english_response.hovered() || chinese_response.hovered();
                                if is_hovered {
                                    self.hovered_word = Some(i);
                                } else if self.hovered_word == Some(i) {
                                    self.hovered_word = None;
                                }

                                // 绘制英文单词
                                let english_rect = english_response.rect;
                                let painter = ui.painter();
                                
                                if is_hovered {
                                    painter.rect_filled(english_rect, 3.0, egui::Color32::from_rgb(230, 250, 230));
                                }

                                painter.text(
                                    english_rect.left_center() + egui::Vec2::new(5.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    &word.english,
                                    egui::FontId::proportional(16.0),
                                    if is_hovered { egui::Color32::DARK_GREEN } else { egui::Color32::BLACK },
                                );

                                // 绘制中文意思
                                let chinese_rect = chinese_response.rect;
                                
                                if is_hovered {
                                    painter.rect_filled(chinese_rect, 3.0, egui::Color32::from_rgb(230, 250, 230));
                                }

                                painter.text(
                                    chinese_rect.left_center() + egui::Vec2::new(5.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    &word.chinese,
                                    egui::FontId::proportional(16.0),
                                    if is_hovered { egui::Color32::DARK_GREEN } else { egui::Color32::BLACK },
                                );
                            });
                            
                            ui.allocate_space(egui::Vec2::new(0.0, 2.0));
                        }
                    });
                }
            } else if groups_clone.is_empty() {
                ui.label("暂无分组，请先添加单词");
            } else {
                ui.label("请选择一个分组查看单词");
            }
        });

        // 添加单词对话框
        if self.show_add_word_dialog {
            egui::Window::new("添加新单词")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("英文单词:");
                    ui.text_edit_singleline(&mut self.new_english);
                    
                    ui.label("中文意思:");
                    ui.text_edit_singleline(&mut self.new_chinese);
                    
                    ui.label("分组名称:");
                    ui.text_edit_singleline(&mut self.new_group);
                    
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        if ui.button("添加").clicked() {
                            if !self.new_english.trim().is_empty() 
                                && !self.new_chinese.trim().is_empty() 
                                && !self.new_group.trim().is_empty() {
                                
                                let word = Word {
                                    english: self.new_english.trim().to_string(),
                                    chinese: self.new_chinese.trim().to_string(),
                                    group: self.new_group.trim().to_string(),
                                };
                                
                                self.flash_memory.add_word(word);
                                self.current_group = Some(self.new_group.trim().to_string());
                                
                                // 清空输入框
                                self.new_english.clear();
                                self.new_chinese.clear();
                                self.new_group.clear();
                                
                                self.show_add_word_dialog = false;
                                self.show_message("单词添加成功！");
                            } else {
                                self.show_message("请填写完整信息！");
                            }
                        }
                        
                        if ui.button("取消").clicked() {
                            self.show_add_word_dialog = false;
                            self.new_english.clear();
                            self.new_chinese.clear();
                            self.new_group.clear();
                        }
                    });
                });
        }
    }
}

impl FlashMemoryApp {
    fn show_message(&mut self, message: &str) {
        self.message = message.to_string();
        self.message_timer = 3.0; // 显示3秒
    }
}

// 在 Windows 上尝试加载系统中文字体（simhei/simkai），用于支持中文显示
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
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "cjk".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "cjk".to_owned());
            loaded = true;
            break;
        }
    }

    // 如果未加载到中文字体，仍沿用默认字体设置
    if loaded {
        ctx.set_fonts(fonts);
    } else {
        ctx.set_fonts(fonts);
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("单词闪记系统"),
        ..Default::default()
    };
    
    eframe::run_native(
        "单词闪记系统",
        options,
        Box::new(|cc| {
            // 应用自定义中文字体以避免中文显示为方块
            configure_chinese_fonts(&cc.egui_ctx);
            Ok(Box::new(FlashMemoryApp::default()))
        }),
    )
}