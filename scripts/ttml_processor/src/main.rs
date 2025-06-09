mod metadata_processor;
mod ttml_generator;
mod ttml_parser;
mod types;
mod validator;

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

use clap::Parser;
use env_logger::Env;

use metadata_processor::MetadataStore;
use types::{DefaultLanguageOptions, TtmlGenerationOptions, TtmlTimingMode};

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 输入的 TTML 文件路径
    #[arg(short, long)]
    input: PathBuf,

    /// 输出的文件路径。如果未提供，结果将打印到标准输出。
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// 输出一个包含元数据的 JSON 文件路径
    #[arg(long)]
    json_output: Option<PathBuf>,

    // 设置TTML的计时模式 ('word' 或 'line')
    #[arg(long, value_enum, default_value_t = TtmlTimingMode::Word)]
    timing_mode: TtmlTimingMode,
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // 解析命令行参数
    let args = Args::parse();

    // --- 1. 读取输入文件 ---
    let ttml_content = match fs::read_to_string(&args.input) {
        Ok(content) => content,
        Err(e) => {
            log::error!("无法读取输入文件 {:?}: {}", args.input, e);
            process::exit(1);
        }
    };

    // --- 2. 解析 TTML 内容 ---
    log::info!("开始解析 TTML 文件...");
    let parsed_data =
        match ttml_parser::parse_ttml_content(&ttml_content, &DefaultLanguageOptions::default()) {
            Ok(data) => {
                if !data.warnings.is_empty() {
                    for warning in &data.warnings {
                        log::warn!("解析警告: {}", warning);
                    }
                }
                log::info!("文件解析成功。");
                data
            }
            Err(e) => {
                log::error!("解析 TTML 文件失败: {}", e);
                process::exit(1);
            }
        };

    // --- 3. 处理元数据 ---
    let mut metadata_store = MetadataStore::new();
    metadata_store.load_from_raw(&parsed_data.raw_metadata);
    metadata_store.deduplicate_values();
    log::info!("元数据处理完毕。");

    log::info!("准备验证的元数据内容: {:?}", metadata_store);

    // --- 4. 验证数据 ---
    log::info!("正在验证歌词数据和元数据...");
    if let Err(errors) =
        validator::validate_lyrics_and_metadata(&parsed_data.lines, &metadata_store)
    {
        log::error!("文件验证失败，发现以下问题:");
        for error in errors {
            eprintln!("- {}", error);
        }
        process::exit(1);
    }
    log::info!("文件验证通过。");

    if let Some(json_output_path) = &args.json_output {
        let serializable_metadata = metadata_store.to_serializable_map();

        let json_string = match serde_json::to_string_pretty(&serializable_metadata) {
            Ok(s) => s,
            Err(e) => {
                log::error!("序列化元数据到 JSON 失败: {}", e);
                process::exit(1);
            }
        };

        if let Err(e) = fs::write(json_output_path, json_string) {
            log::error!("写入元数据 JSON 文件 {:?} 失败: {}", json_output_path, e);
            process::exit(1);
        }
    }

    // --- 5. 生成新的 TTML ---
    log::info!("正在生成 TTML 文件...");
    let generation_options = TtmlGenerationOptions {
        timing_mode: args.timing_mode,
        ..Default::default()
    };

    let final_ttml = match ttml_generator::generate_ttml(
        &parsed_data.lines,
        &metadata_store,
        &generation_options,
    ) {
        Ok(content) => content,
        Err(e) => {
            log::error!("生成 TTML 文件失败: {}", e);
            process::exit(1);
        }
    };

    // --- 6. 输出结果 ---
    match args.output {
        Some(output_path) => {
            log::info!("正在将结果写入文件: {:?}", output_path);
            if let Err(e) = fs::write(&output_path, final_ttml) {
                log::error!("写入输出文件 {:?} 失败: {}", output_path, e);
                process::exit(1);
            }
            log::info!("处理成功！输出文件已保存。");
        }
        None => {
            log::info!("正在将结果打印到标准输出...");
            if let Err(e) = io::stdout().write_all(final_ttml.as_bytes()) {
                log::error!("写入标准输出失败: {}", e);
                process::exit(1);
            }
        }
    }
}
