use iced::{Element, Length, Task, Theme, Renderer};
use iced::widget::{button, column, container, progress_bar, row, scrollable, text, Column};
use std::path::PathBuf;
use std::sync::Arc;
use genpdf::fonts::FontData;
use crate::converter;
use log::{info, warn};

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub status: ConversionStatus,
}

#[derive(Debug, Clone)]
pub enum ConversionStatus {
    Pending,
    Converting,
    Success,
    Error(String),
}

pub struct App {
    files: Vec<FileEntry>,
    output_dir: Option<PathBuf>,
    is_converting: bool,
    font: Arc<FontData>,
    total_files: usize,
    completed_files: usize,
    show_about: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    AddFiles,
    FilesSelected(Vec<PathBuf>),
    RemoveFile(usize),
    SelectOutputDir,
    OutputDirSelected(PathBuf),
    ConvertAll,
    ConversionFinished(usize, Result<(), String>),
    ToggleAbout,
    OpenLink(String),
    None,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        // Only use known-good .ttf files or fallback. Avoid .ttc for now as they may cause hangs in genpdf.
        let system_fonts = [
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "C:\\Windows\\Fonts\\msyh.ttc", // Microsoft YaHei
            "C:\\Windows\\Fonts\\simhei.ttf", // SimHei
            "C:\\Windows\\Fonts\\arial.ttf",
        ];

        let mut selected_font = None;
        for path in system_fonts {
            if let Ok(bytes) = std::fs::read(path) {
                let bytes_arc = Arc::new(bytes);
                if let Ok(font) = converter::prepare_font(bytes_arc) {
                    info!("Successfully loaded system font: {}", path);
                    selected_font = Some(font);
                    break;
                }
            }
        }

        let font = selected_font.unwrap_or_else(|| {
             warn!("Loading embedded fallback font (Roboto).");
             let bytes = include_bytes!("../assets/Roboto-Regular.ttf").to_vec();
             converter::prepare_font(Arc::new(bytes)).expect("Failed to load embedded font")
        });

        (
            Self {
                files: Vec::new(),
                output_dir: None,
                is_converting: false,
                font,
                total_files: 0,
                completed_files: 0,
                show_about: false,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddFiles => {
                return Task::perform(async {
                    let files = rfd::AsyncFileDialog::new()
                        .add_filter("Documents", &["md", "json", "xml", "txt", "docx", "html", "htm", "csv", "png", "jpg", "jpeg", "bmp", "rs", "py", "js", "c", "cpp", "yaml", "yml", "toml", "xlsx", "xls"])
                        .pick_files()
                        .await;
                    
                    if let Some(files) = files {
                        files.into_iter().map(|f| f.path().to_path_buf()).collect()
                    } else {
                        Vec::new()
                    }
                }, Message::FilesSelected);
            }
            Message::FilesSelected(paths) => {
                info!("Selected {} files", paths.len());
                for path in paths {
                    if !self.files.iter().any(|f| f.path == path) {
                        info!("Adding file: {:?}", path);
                        self.files.push(FileEntry {
                            path,
                            status: ConversionStatus::Pending,
                        });
                    } else {
                        info!("Skipping duplicate file: {:?}", path);
                    }
                }
            }
            Message::RemoveFile(index) => {
                if index < self.files.len() {
                    if let Some(file) = self.files.get(index) {
                        info!("Removing file: {:?}", file.path);
                    }
                    self.files.remove(index);
                }
            }
            Message::SelectOutputDir => {
                return Task::perform(async {
                    let dir = rfd::AsyncFileDialog::new()
                        .pick_folder()
                        .await;
                    
                    dir.map(|d| d.path().to_path_buf())
                }, |d| if let Some(d) = d { Message::OutputDirSelected(d) } else { Message::None });
            }
            Message::OutputDirSelected(path) => {
                info!("Output directory set to: {:?}", path);
                self.output_dir = Some(path);
            }
            Message::ConvertAll => {
                if self.files.is_empty() || self.is_converting {
                    return Task::none();
                }

                info!("Starting batch conversion...");
                self.is_converting = true;
                self.completed_files = 0;
                self.total_files = 0;

                let mut tasks = Vec::new();
                
                let output_base = self.output_dir.clone();
                let font_arc = self.font.clone();

                // Count files to convert
                let files_to_convert: Vec<usize> = self.files.iter().enumerate()
                    .filter(|(_, f)| !matches!(f.status, ConversionStatus::Success))
                    .map(|(i, _)| i)
                    .collect();
                
                self.total_files = files_to_convert.len();
                info!("Files scheduled for conversion: {}", self.total_files);

                if self.total_files == 0 {
                    self.is_converting = false;
                    info!("No pending files to convert.");
                    return Task::none();
                }

                for i in files_to_convert {
                    if let Some(file) = self.files.get_mut(i) {
                         file.status = ConversionStatus::Converting;
                         
                         let input_path = file.path.clone();
                         let output_dir = output_base.clone().unwrap_or_else(|| input_path.parent().unwrap().to_path_buf());
                         let file_stem = input_path.file_stem().unwrap().to_string_lossy().to_string();
                         let output_path = output_dir.join(format!("{}.pdf", file_stem));
                         let font_for_task = font_arc.clone();

                         tasks.push(Task::perform(async move {
                            let (tx, rx) = futures::channel::oneshot::channel();
                            
                            std::thread::spawn(move || {
                                 let res = converter::convert(&input_path, &output_path, font_for_task);
                                 let _ = tx.send(res);
                            });
                            
                            match rx.await {
                                Ok(res) => match res {
                                    Ok(_) => Ok(()),
                                    Err(e) => Err(e.to_string()),
                                },
                                Err(_) => Err("Task cancelled or panicked".to_string()),
                            }
                        }, move |res| Message::ConversionFinished(i, res)));
                    }
                }
                
                return Task::batch(tasks);
            }
            Message::ConversionFinished(index, result) => {
                self.completed_files += 1;
                if let Some(file) = self.files.get_mut(index) {
                    match &result {
                        Ok(_) => {
                            info!("Conversion successful for: {:?}", file.path);
                            file.status = ConversionStatus::Success;
                        },
                        Err(e) => {
                            log::error!("Conversion failed for {:?}: {}", file.path, e);
                            file.status = ConversionStatus::Error(e.clone());
                        },
                    }
                }
                
                if self.completed_files >= self.total_files {
                    self.is_converting = false;
                    info!("Batch conversion completed.");
                }
            }
            Message::ToggleAbout => {
                self.show_about = !self.show_about;
            }
            Message::OpenLink(url) => {
                info!("Opening URL: {}", url);
                let _ = webbrowser::open(&url);
            }
            Message::None => {} // No-op
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        // Colors
        let primary_color = iced::Color::from_rgb(0.2, 0.6, 1.0); // Light blue
        let success_color = iced::Color::from_rgb(0.2, 0.8, 0.4); // Green
        let text_color = iced::Color::from_rgb(0.9, 0.9, 0.9);
        let muted_color = iced::Color::from_rgb(0.6, 0.6, 0.6);
        let panel_bg = iced::Color::from_rgb(0.12, 0.12, 0.12);
        let card_bg = iced::Color::from_rgb(0.18, 0.18, 0.18);
        
        if self.show_about {
            let about_content = container(
                column![
                    text("关于 Topdf").size(24).color(text_color),
                    text("一个高效、跨平台的文档转PDF工具").size(16).color(muted_color),
                    Column::new().spacing(10).push(
                        row![
                            text("作者: ").color(text_color),
                            button(text("StarsUnsurpass").color(primary_color))
                                .on_press(Message::OpenLink("https://github.com/StarsUnsurpass".to_string()))
                                .style(|_,_| button::Style { background: None, ..button::Style::default() })
                        ]
                    ).push(
                         row![
                            text("项目地址: ").color(text_color),
                            button(text("GitHub/Topdf").color(primary_color))
                                .on_press(Message::OpenLink("https://github.com/StarsUnsurpass/Topdf".to_string()))
                                .style(|_,_| button::Style { background: None, ..button::Style::default() })
                        ]
                    ),
                    button(text("返回").size(16))
                        .on_press(Message::ToggleAbout)
                        .padding(10)
                        .style(move |_theme, status| {
                             let mut base = button::Style::default();
                             base.background = Some(iced::Color::from_rgb(0.3, 0.3, 0.3).into());
                             base.text_color = text_color;
                             base.border = iced::Border { radius: 6.0.into(), ..iced::Border::default() };
                             match status {
                                 button::Status::Hovered => {
                                     base.background = Some(iced::Color::from_rgb(0.4, 0.4, 0.4).into());
                                     base
                                 },
                                 _ => base,
                             }
                        })
                ]
                .spacing(20)
                .align_x(iced::Alignment::Center)
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Color::from_rgb(0.08, 0.08, 0.08).into()),
                ..container::Style::default()
            });

            return about_content.into();
        }

        let nav_bar = row![
            text("Topdf").size(20).color(primary_color).font(iced::font::Font::DEFAULT),
            iced::widget::Space::new().width(Length::Fill),
            button(text("更多").size(14))
                .on_press(Message::ToggleAbout)
                .style(move |_theme, status| {
                    let mut base = button::Style::default();
                    base.background = None;
                    base.text_color = muted_color;
                    match status {
                        button::Status::Hovered => {
                            base.text_color = primary_color;
                            base
                        },
                         _ => base,
                    }
                })
        ]
        .padding(10)
        .align_y(iced::Alignment::Center);

        let title = text("Topdf 文档转换器").size(36).color(primary_color).font(iced::font::Font::DEFAULT);
        let subtitle = text("高效 · 极简 · 多格式支持").size(16).color(muted_color);
        
        let header = column![title, subtitle].spacing(5).align_x(iced::Alignment::Center);

        let add_btn = button(
            text("  + 添加文件  ").size(16)
        )
        .on_press(Message::AddFiles)
        .padding(12)
        .style(move |_theme, status| {
             let mut base = button::Style::default();
             base.background = Some(primary_color.into());
             base.text_color = iced::Color::WHITE;
             base.border = iced::Border {
                    color: iced::Color::TRANSPARENT,
                    width: 0.0,
                    radius: 8.0.into(),
                };
             match status {
                 button::Status::Hovered => {
                     base.background = Some(iced::Color::from_rgb(0.3, 0.7, 1.0).into());
                     base
                 },
                 button::Status::Pressed => {
                     base.background = Some(iced::Color::from_rgb(0.1, 0.5, 0.9).into());
                     base
                 },
                 _ => base,
             }
        });
            
        let file_list_content: Element<Message> = if self.files.is_empty() {
            container(
                column![
                    text("暂无文件").size(20).color(muted_color),
                    text("拖拽文件到此处 或 点击上方“添加文件”按钮").size(14).color(muted_color)
                ].spacing(10).align_x(iced::Alignment::Center)
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else {
            let list = Column::with_children(
                self.files.iter().enumerate().map(|(i, file)| {
                    let name = file.path.file_name().unwrap_or_default().to_string_lossy();
                    let (status_txt, status_color) = match &file.status {
                        ConversionStatus::Pending => ("等待中", muted_color),
                        ConversionStatus::Converting => ("转换中...", primary_color),
                        ConversionStatus::Success => ("转换成功", success_color),
                        ConversionStatus::Error(_e) => ("转换失败", iced::Color::from_rgb(0.9, 0.3, 0.3)),
                    };
                    
                    let status_element = if let ConversionStatus::Error(e) = &file.status {
                         column![
                             text(status_txt).size(12).color(status_color),
                             text(e).size(10).color(status_color)
                         ]
                    } else {
                         column![text(status_txt).size(12).color(status_color)]
                    };

                    let remove_btn = if !self.is_converting {
                        button(text(" × ").size(14))
                            .on_press(Message::RemoveFile(i))
                            .padding(5)
                            .style(move |_theme, status| {
                                let mut base = button::Style::default();
                                base.text_color = muted_color;
                                base.background = Some(iced::Color::TRANSPARENT.into());
                                match status {
                                    button::Status::Hovered => {
                                        base.text_color = iced::Color::from_rgb(0.9, 0.3, 0.3);
                                        base
                                    },
                                    _ => base,
                                }
                            })
                    } else {
                        button(text(" ").size(14)).style(|_,_| {
                            let mut s = button::Style::default();
                            s.background = Some(iced::Color::TRANSPARENT.into());
                            s
                        }) 
                    };

                    container(row![
                        column![
                            text(name).size(14).color(text_color),
                            status_element
                        ].width(Length::Fill).spacing(4),
                        remove_btn
                    ]
                    .align_y(iced::Alignment::Center)
                    .spacing(10))
                    .padding(12)
                    .style(move |_theme| container::Style {
                        background: Some(card_bg.into()),
                        border: iced::Border {
                            color: iced::Color::from_rgb(0.25, 0.25, 0.25),
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        ..container::Style::default()
                    })
                    .into()
                })
            ).spacing(8);
            
            scrollable(list).into()
        };

        let output_text = if let Some(p) = &self.output_dir {
            format!("输出路径: {}", p.display())
        } else {
            "输出路径: 默认 (源文件所在目录)".to_string()
        };
        
        let progress_section: Element<Message> = if self.is_converting || (self.completed_files > 0 && self.completed_files < self.total_files) {
             let progress = if self.total_files > 0 {
                 self.completed_files as f32 / self.total_files as f32 * 100.0
             } else {
                 0.0
             };
             
             column![
                 row![
                     text::<Theme, Renderer>("总体进度:").size(12).color(muted_color),
                     text::<Theme, Renderer>(format!("{} / {}", self.completed_files, self.total_files)).size(12).color(primary_color)
                 ].spacing(5),
                 progress_bar::<Theme>(0.0..=100.0, progress).style(move |_theme| progress_bar::Style {
                     background: iced::Color::from_rgb(0.2, 0.2, 0.2).into(),
                     bar: primary_color.into(),
                     border: iced::Border {
                         radius: 3.0.into(),
                         ..iced::Border::default()
                     },
                 })
             ].spacing(8).into()
        } else {
             Column::new().into()
        };

        let left_panel = container(column![
            row![add_btn, text("待转换列表").size(18).color(text_color)].spacing(20).align_y(iced::Alignment::Center),
            container(file_list_content)
                .height(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(iced::Color::from_rgb(0.1, 0.1, 0.1).into()),
                    border: iced::Border {
                        color: iced::Color::from_rgb(0.2, 0.2, 0.2),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..container::Style::default()
                })
                .padding(15),
            progress_section,
            row![
                button(text("选择输出文件夹").size(14))
                    .on_press(Message::SelectOutputDir)
                    .padding(10)
                    .style(move |_theme, status| {
                        let mut base = button::Style::default();
                        base.background = Some(iced::Color::from_rgb(0.25, 0.25, 0.25).into());
                        base.text_color = text_color;
                        base.border = iced::Border { radius: 6.0.into(), ..iced::Border::default() };
                        match status {
                            button::Status::Hovered => {
                                base.background = Some(iced::Color::from_rgb(0.35, 0.35, 0.35).into());
                                base
                            },
                            _ => base,
                        }
                    }),
                container(text(output_text).size(12).color(muted_color)).width(Length::Fill).align_y(iced::Alignment::Center),
                button(text(" 开始转换 ").size(16).font(iced::font::Font::DEFAULT)) // bold if possible
                    .on_press(Message::ConvertAll)
                    .padding(12)
                    .style(move |_theme, status| {
                         let mut base = button::Style::default();
                         base.background = Some(success_color.into());
                         base.text_color = iced::Color::WHITE;
                         base.border = iced::Border {
                                color: iced::Color::TRANSPARENT,
                                width: 0.0,
                                radius: 8.0.into(),
                            };
                         match status {
                             button::Status::Hovered => {
                                 base.background = Some(iced::Color::from_rgb(0.3, 0.9, 0.5).into());
                                 base
                             },
                             button::Status::Pressed => {
                                 base.background = Some(iced::Color::from_rgb(0.1, 0.7, 0.3).into());
                                 base
                             },
                             _ => base,
                         }
                    })
            ].spacing(15).align_y(iced::Alignment::Center)
        ]
        .spacing(20))
        .width(Length::FillPortion(2));

        // Right Panel: Info and Help
        let right_panel = container(column![
            text("支持的文件格式").size(18).color(success_color),
            column![
                text("• 文档: DOCX, TXT").size(14).color(text_color),
                text("• 数据: JSON, XML, CSV, YAML, TOML, Excel").size(14).color(text_color),
                text("• 网页: HTML, Markdown (MD)").size(14).color(text_color),
                text("• 图片: PNG, JPG, BMP").size(14).color(text_color),
                text("• 代码: RS, PY, JS, C, CPP").size(14).color(text_color),
            ].spacing(8),
            
            text("操作指南").size(18).color(iced::Color::from_rgb(1.0, 0.8, 0.4)), // Gold
            column![
                text("1. 点击“添加文件”或直接将文件拖入窗口。").size(14).color(text_color),
                text("2. (可选) 点击“选择输出文件夹”修改保存位置。").size(14).color(text_color),
                text("3. 点击“开始转换”按钮。").size(14).color(text_color),
            ].spacing(8),
            
            container(
                text("提示: 软件内置了中文字体支持，若仍出现乱码，请确保系统安装了微软雅黑或 SimHei 字体。"
                ).size(12).color(muted_color)
            ).padding(10).style(|_theme| container::Style {
                background: Some(iced::Color::from_rgb(0.15, 0.15, 0.18).into()),
                border: iced::Border { radius: 6.0.into(), ..iced::Border::default() },
                ..container::Style::default()
            })
        ]
        .spacing(20))
        .padding(25)
        .width(Length::FillPortion(1))
        .style(move |_theme| container::Style {
            background: Some(panel_bg.into()),
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: 12.0.into(),
            },
            ..container::Style::default()
        });

        let main_content = row![left_panel, right_panel].spacing(30).height(Length::Fill);

        container(column![nav_bar, header, main_content].spacing(20))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .style(|_theme| container::Style {
                background: Some(iced::Color::from_rgb(0.08, 0.08, 0.08).into()), // Very dark bg
                ..container::Style::default()
            })
            .into()
    }
}
