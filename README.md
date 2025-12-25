# 微博批量隐私设置工具

这是一个用 Rust 编写的命令行工具，可以批量将你的新浪微博设置为仅好友可见（或其他隐私级别）。
本项目完全由claude生成，本人修改不足1%

## 功能特性

- 批量设置微博隐私级别（公开/仅好友/仅自己）
- 支持从 Cookie 文件读取认证信息
- 进度条显示处理进度
- 支持跳过前 N 条微博
- 预览模式（dry-run）查看将要处理的微博
- 可调节请求延迟，避免触发反爬虫机制

## 安装

确保你已经安装了 Rust 工具链（1.70+），然后：

```bash
cd weibo_hide
cargo build --release
```

编译完成后，可执行文件位于 `target/release/weibo_hide.exe`

## 使用方法

### 1. 获取 Cookie

首先需要从浏览器获取你的微博 Cookie：

1. 使用浏览器登录 weibo.com
2. 打开开发者工具（F12）
3. 切换到 Network 标签
4. 刷新页面，找到任意请求
5. 在请求头中找到 Cookie，复制完整的 Cookie 字符串
6. 将 Cookie 保存到文件（如 `cookie.txt`）

**重要**：Cookie 中必须包含 `XSRF-TOKEN` 字段！

### 2. 获取用户 ID

访问你的微博主页，URL 类似：`https://weibo.com/u/1234567890`

其中 `1234567890` 就是你的用户 ID。

### 3. 预览将要处理的微博

使用 `--dry-run` 参数先预览：

```bash
weibo_hide hide --user-id 1234567890 --cookie-file cookie.txt --dry-run
```

### 4. 批量设置隐私

确认无误后，去掉 `--dry-run` 参数执行：

```bash
# 设置为仅好友可见（默认）
weibo_hide hide --user-id 1234567890 --cookie-file cookie.txt

# 设置为仅自己可见
weibo_hide hide --user-id 1234567890 --cookie-file cookie.txt --visibility private

# 设置为公开
weibo_hide hide --user-id 1234567890 --cookie-file cookie.txt --visibility public
```

## 命令参数

### hide 命令（批量设置隐私）

```
weibo_hide hide [OPTIONS] --user-id <USER_ID>

选项：
  -u, --user-id <USER_ID>          微博用户ID（必需）
  -c, --cookie <COOKIE>            Cookie字符串
  -f, --cookie-file <COOKIE_FILE>  Cookie文件路径
  -p, --max-pages <MAX_PAGES>      最大处理页数（默认全部）
  -v, --visibility <VISIBILITY>    隐私级别 [默认: friends]
                                   可选值: public, friends, private
  -d, --delay <DELAY>              每条微博处理后的延迟（秒）[默认: 1]
  -s, --skip <SKIP>                跳过前N条微博 [默认: 0]
      --dry-run                    预览模式，不实际修改
  -h, --help                       显示帮助信息
```

### list 命令（查看微博列表）

```
weibo_hide list [OPTIONS] --user-id <USER_ID>

选项：
  -u, --user-id <USER_ID>          微博用户ID（必需）
  -c, --cookie <COOKIE>            Cookie字符串
  -f, --cookie-file <COOKIE_FILE>  Cookie文件路径
  -p, --max-pages <MAX_PAGES>      最大获取页数 [默认: 1]
  -o, --output <OUTPUT>            输出到文件
  -h, --help                       显示帮助信息
```

## 使用示例

```bash
# 1. 预览将要处理的微博
weibo_hide hide -u 1234567890 -f cookie.txt --dry-run

# 2. 批量设置为仅好友可见（处理前5页）
weibo_hide hide -u 1234567890 -f cookie.txt -p 5

# 3. 跳过前100条，设置剩余微博为仅好友可见
weibo_hide hide -u 1234567890 -f cookie.txt -s 100

# 4. 设置所有微博为仅自己可见，每条延迟2秒
weibo_hide hide -u 1234567890 -f cookie.txt -v private -d 2

# 5. 查看微博列表（前3页）
weibo_hide list -u 1234567890 -f cookie.txt -p 3

# 6. 导出微博列表到文件
weibo_hide list -u 1234567890 -f cookie.txt -p 10 -o weibos.txt
```

## 重要提示

⚠️ **使用前必读**：

1. **Cookie 安全**：Cookie 包含你的登录凭证，请妥善保管，不要泄露给他人
2. **频率限制**：建议设置适当的延迟（1-2秒），避免请求过快被封号
3. **API 变化**：微博的 API 可能会变化，如果工具失效，需要更新 API 端点
4. **备份数据**：建议先使用 `--dry-run` 预览，确认无误后再执行
5. **分批处理**：如果微博数量很多，建议分批处理（使用 `-p` 和 `-s` 参数）

## 故障排除

### 问题 1: "无法从 Cookie 中提取 XSRF-TOKEN"

**解决方法**：确保你的 Cookie 完整，包含 `XSRF-TOKEN=...` 字段。重新登录微博并复制完整的 Cookie。

### 问题 2: API 返回 403 或 401 错误

**解决方法**：
- Cookie 可能已过期，重新获取
- 检查用户 ID 是否正确
- 尝试增加延迟时间（`-d 2` 或更长）

### 问题 3: "设置失败"

**可能原因**：
1. 微博 API 端点已变化（需要更新代码）
2. 该微博不支持隐私设置（如转发的微博）
3. 网络问题或被限流

**解决方法**：
- 检查网络连接
- 增加延迟时间
- 如果持续失败，可能需要抓包分析新的 API

## 技术说明

### API 端点

当前使用的微博 API 端点：

- 获取微博列表：`https://weibo.com/ajax/statuses/mymblog?uid={user_id}&page={page}&feature=0`
- 设置隐私：`https://weibo.com/ajax/statuses/modifyVisible`（可能需要根据实际情况调整）

如果 API 失效，你可以：
1. 登录微博网页版
2. 手动修改一条微博的隐私设置
3. 在浏览器开发者工具中查看网络请求
4. 找到新的 API 端点和参数
5. 修改 `src/weibo_client.rs` 中的相关代码

### 项目结构

```
weibo_hide/
├── Cargo.toml           # 项目配置和依赖
├── src/
│   ├── main.rs          # 主程序入口
│   └── weibo_client.rs  # 微博 API 客户端
└── README.md            # 本文档
```

## 许可证

本项目仅供学习交流使用，请遵守微博的服务条款。

## 贡献

欢迎提交 Issue 和 Pull Request！
