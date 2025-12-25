use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;

mod weibo_client;
use weibo_client::{Visibility, WeiboPrivacyClient};

#[derive(Parser, Debug)]
#[command(author, version, about = "微博批量隐私设置工具", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 批量设置微博隐私
    Hide {
        /// 微博用户ID
        #[arg(short, long)]
        user_id: String,

        /// Cookie字符串（从浏览器复制）
        #[arg(short, long)]
        cookie: Option<String>,

        /// Cookie文件路径
        #[arg(short = 'f', long)]
        cookie_file: Option<String>,

        /// 最大处理页数（默认处理所有）
        #[arg(short = 'p', long)]
        max_pages: Option<u32>,

        /// 隐私级别: public(公开), friends(仅好友), private(仅自己), fans(仅粉丝)
        #[arg(short = 'v', long, default_value = "friends")]
        visibility: String,

        /// 延迟时间（秒），每条微博设置后的等待时间
        #[arg(short = 'd', long, default_value = "1")]
        delay: u64,

        /// 跳过前N条微博
        #[arg(short = 's', long, default_value = "0")]
        skip: usize,

        /// 限制处理的微博数量
        #[arg(short = 'l', long)]
        limit: Option<usize>,

        /// 只显示将要处理的微博，不实际修改
        #[arg(long, default_value = "false")]
        dry_run: bool,
    },

    /// 获取微博列表（不修改）
    List {
        /// 微博用户ID
        #[arg(short, long)]
        user_id: String,

        /// Cookie字符串（从浏览器复制）
        #[arg(short, long)]
        cookie: Option<String>,

        /// Cookie文件路径
        #[arg(short = 'f', long)]
        cookie_file: Option<String>,

        /// 最大获取页数
        #[arg(short = 'p', long, default_value = "1")]
        max_pages: u32,

        /// 输出到文件
        #[arg(short, long)]
        output: Option<String>,
    },
}

/// 从命令行参数或文件读取 Cookie
fn load_cookie(cookie: &Option<String>, cookie_file: &Option<String>) -> Result<String> {
    if let Some(cookie_str) = cookie {
        Ok(cookie_str.clone())
    } else if let Some(cookie_path) = cookie_file {
        println!("从文件读取 Cookie: {}", cookie_path);
        let cookie_content = fs::read_to_string(cookie_path)
            .context(format!("无法读取 Cookie 文件: {}", cookie_path))?
            .trim()
            .to_string();
        Ok(cookie_content)
    } else {
        Err(anyhow::anyhow!("必须提供 Cookie，使用 --cookie 或 --cookie-file 参数"))
    }
}

/// 解析隐私级别
fn parse_visibility(visibility_str: &str) -> Result<Visibility> {
    match visibility_str.to_lowercase().as_str() {
        "public" | "公开" => Ok(Visibility::Public),
        "friends" | "好友" | "仅好友" => Ok(Visibility::FriendsOnly),
        "private" | "私密" | "仅自己" => Ok(Visibility::Private),
        "fans" | "粉丝" | "仅粉丝" => Ok(Visibility::FansOnly),
        _ => Err(anyhow::anyhow!(
            "无效的隐私级别: {}，可选值: public, friends, private, fans",
            visibility_str
        )),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Hide {
            user_id,
            cookie,
            cookie_file,
            max_pages,
            visibility,
            delay,
            skip,
            limit,
            dry_run,
        } => {
            println!("=== 微博批量隐私设置工具 ===\n");

            // 读取 Cookie
            let cookie_data = load_cookie(&cookie, &cookie_file)?;

            // 解析隐私级别
            let visibility_level = parse_visibility(&visibility)?;

            println!("目标用户 ID: {}", user_id);
            println!("隐私级别: {}", visibility_level.as_str());
            if let Some(pages) = max_pages {
                println!("最大处理页数: {}", pages);
            }
            println!("跳过前 {} 条", skip);
            if let Some(n) = limit {
                println!("限制处理 {} 条", n);
            }
            if dry_run {
                println!("⚠️  预览模式：只显示将要处理的微博，不实际修改");
            }
            println!();

            // 创建客户端
            println!("正在初始化客户端...");
            let client = WeiboPrivacyClient::new(cookie_data)?;
            println!("✓ 客户端初始化成功\n");

            // 获取所有微博
            println!("正在获取微博列表...");
            let weibos = client.get_all_weibo_ids(&user_id, max_pages).await?;
            println!("✓ 共获取 {} 条微博\n", weibos.len());

            if weibos.is_empty() {
                println!("没有找到微博");
                return Ok(());
            }

            // 跳过指定数量
            let mut weibos_to_process: Vec<_> = weibos.into_iter().skip(skip).collect();

            // 限制处理数量
            if let Some(n) = limit {
                weibos_to_process.truncate(n);
            }

            if weibos_to_process.is_empty() {
                println!("跳过后没有需要处理的微博");
                return Ok(());
            }

            println!("将要处理 {} 条微博\n", weibos_to_process.len());

            if dry_run {
                println!("预览前10条:");
                for (idx, weibo) in weibos_to_process.iter().take(10).enumerate() {
                    let text = weibo
                        .text
                        .as_ref()
                        .map(|s| {
                            let preview: String = s.chars().take(30).collect();
                            preview
                        })
                        .unwrap_or_else(|| "无内容".to_string());
                    println!(
                        "  {}. ID: {} - {}...",
                        idx + 1 + skip,
                        weibo.id,
                        text
                    );
                }
                if weibos_to_process.len() > 10 {
                    println!("  ... 还有 {} 条", weibos_to_process.len() - 10);
                }
                println!("\n使用相同命令但不加 --dry-run 参数即可开始修改");
                return Ok(());
            }

            // 确认
            println!("准备将这些微博设置为: {}", visibility_level.as_str());
            println!("按 Ctrl+C 取消，或按回车继续...");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            // 创建进度条
            let pb = ProgressBar::new(weibos_to_process.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                    .unwrap()
                    .progress_chars("#>-"),
            );

            let mut success_count = 0;
            let mut failed_count = 0;
            let mut failed_ids = Vec::new();

            for weibo in weibos_to_process {
                let result = client.set_weibo_privacy(&weibo.id, visibility_level).await;

                match result {
                    Ok(_) => {
                        success_count += 1;
                        pb.set_message(format!("✓ {} 成功", weibo.id));
                    }
                    Err(e) => {
                        failed_count += 1;
                        failed_ids.push((weibo.id.clone(), e.to_string()));
                        pb.set_message(format!("✗ {} 失败: {}", weibo.id, e));
                    }
                }

                pb.inc(1);

                // 延迟
                if delay > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                }
            }

            pb.finish_with_message("完成");

            println!("\n=== 处理完成 ===");
            println!("✓ 成功: {} 条", success_count);
            if failed_count > 0 {
                println!("✗ 失败: {} 条", failed_count);
                println!("\n失败详情:");
                for (id, err) in failed_ids.iter().take(10) {
                    println!("  - ID {}: {}", id, err);
                }
            }
        }

        Commands::List {
            user_id,
            cookie,
            cookie_file,
            max_pages,
            output,
        } => {
            println!("=== 获取微博列表 ===\n");

            // 读取 Cookie
            let cookie_data = load_cookie(&cookie, &cookie_file)?;

            println!("目标用户 ID: {}", user_id);
            println!("最大获取页数: {}\n", max_pages);

            // 创建客户端
            let client = WeiboPrivacyClient::new(cookie_data)?;

            // 获取微博
            let weibos = client.get_all_weibo_ids(&user_id, Some(max_pages)).await?;

            println!("\n共获取 {} 条微博\n", weibos.len());

            // 显示或保存
            if let Some(output_path) = output {
                let mut content = String::new();
                for (idx, weibo) in weibos.iter().enumerate() {
                    content.push_str(&format!("{}. ID: {}\n", idx + 1, weibo.id));
                    if let Some(ref text) = weibo.text {
                        content.push_str(&format!("   内容: {}\n", text));
                    }
                    if let Some(ref created_at) = weibo.created_at {
                        content.push_str(&format!("   时间: {}\n", created_at));
                    }
                    content.push_str("\n");
                }

                fs::write(&output_path, content)?;
                println!("✓ 已保存到: {}", output_path);
            } else {
                for (idx, weibo) in weibos.iter().take(20).enumerate() {
                    let text = weibo
                        .text
                        .as_ref()
                        .map(|s| {
                            let preview: String = s.chars().take(50).collect();
                            preview
                        })
                        .unwrap_or_else(|| "无内容".to_string());
                    println!("{}. ID: {} - {}...", idx + 1, weibo.id, text);
                }
                if weibos.len() > 20 {
                    println!("... 还有 {} 条（使用 --output 参数保存完整列表）", weibos.len() - 20);
                }
            }
        }
    }

    Ok(())
}
