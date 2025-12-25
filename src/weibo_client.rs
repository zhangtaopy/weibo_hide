use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Response};
use serde::Deserialize;
use std::time::Duration;

/// 微博可见性设置
#[derive(Debug, Clone, Copy)]
pub enum Visibility {
    /// 公开
    Public = 0,
    /// 仅自己可见
    Private = 1,
    /// 仅好友可见
    FriendsOnly = 2,
    /// 仅粉丝可见
    FansOnly = 10,
}

impl Visibility {
    pub fn as_str(&self) -> &str {
        match self {
            Visibility::Public => "公开",
            Visibility::FriendsOnly => "仅好友可见",
            Visibility::Private => "仅自己可见",
            Visibility::FansOnly => "仅粉丝可见",
        }
    }
}

/// 微博信息
#[derive(Debug, Deserialize, Clone)]
pub struct WeiboInfo {
    #[serde(deserialize_with = "deserialize_number_to_string")]
    pub id: String,
    pub text: Option<String>,
    pub created_at: Option<String>,
}

/// 微博列表响应
#[derive(Debug, Deserialize)]
struct WeiboListResponse {
    pub ok: i32,
    pub data: WeiboListData,
}

#[derive(Debug, Deserialize)]
struct WeiboListData {
    pub list: Vec<WeiboInfo>,
}

/// 设置隐私响应
#[derive(Debug, Deserialize)]
struct PrivacyResponse {
    pub ok: Option<i32>,
    pub msg: Option<String>,
}

// 自定义反序列化：将数字转换为字符串
fn deserialize_number_to_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct StringOrNumber;

    impl<'de> Visitor<'de> for StringOrNumber {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or number")
        }

        fn visit_i64<E>(self, value: i64) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_u64<E>(self, value: u64) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_str<E>(self, value: &str) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }
    }

    deserializer.deserialize_any(StringOrNumber)
}

pub struct WeiboPrivacyClient {
    client: Client,
    cookie: String,
    xsrf_token: String,
}

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const REQUEST_TIMEOUT_SECS: u64 = 30;
const MAX_RETRIES: u32 = 3;

impl WeiboPrivacyClient {
    /// 创建新客户端
    pub fn new(cookie: String) -> Result<Self> {
        let xsrf_token = Self::extract_xsrf_token(&cookie)
            .ok_or_else(|| anyhow!("无法从 Cookie 中提取 XSRF-TOKEN，请确保 Cookie 完整"))?;

        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .cookie_store(true)
            .user_agent(USER_AGENT)
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            client,
            cookie,
            xsrf_token,
        })
    }

    /// 从 Cookie 中提取 XSRF-TOKEN
    fn extract_xsrf_token(cookie: &str) -> Option<String> {
        cookie
            .split(';')
            .find(|s| s.trim().starts_with("XSRF-TOKEN="))
            .and_then(|s| s.split('=').nth(1))
            .map(|s| s.trim().to_string())
    }

    /// 获取用户所有微博 ID 列表
    pub async fn get_all_weibo_ids(&self, user_id: &str, max_pages: Option<u32>) -> Result<Vec<WeiboInfo>> {
        let mut all_weibos = Vec::new();
        let mut page = 1;
        let max_pages = max_pages.unwrap_or(u32::MAX);

        loop {
            if page > max_pages {
                break;
            }

            let url = format!(
                "https://weibo.com/ajax/statuses/mymblog?uid={}&page={}&feature=0",
                user_id, page
            );

            let response = self.get_with_retry(&url, user_id).await?;
            let response_text = response.text().await?;

            let weibo_response: WeiboListResponse = serde_json::from_str(&response_text)
                .context(format!("Failed to parse JSON response at page {}", page))?;

            if weibo_response.ok != 1 {
                return Err(anyhow!("API 返回错误: ok={}", weibo_response.ok));
            }

            let weibos = weibo_response.data.list;

            if weibos.is_empty() {
                break;
            }

            println!("✓ 第 {} 页: 获取 {} 条微博", page, weibos.len());
            all_weibos.extend(weibos);

            page += 1;

            // 避免请求过快
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        Ok(all_weibos)
    }

    /// 设置微博隐私
    pub async fn set_weibo_privacy(&self, weibo_id: &str, visibility: Visibility) -> Result<()> {
        // 微博设置隐私的 API 端点（根据实际抓包结果）
        let url = "https://weibo.com/ajax/statuses/modifyVisible";

        let visible_value = match visibility {
            Visibility::Public => 0,
            Visibility::FriendsOnly => 2,
            Visibility::Private => 1,
            Visibility::FansOnly => 10,
        };

        // 使用 form 格式，参数名是 ids（复数）不是 id
        let visible_str = visible_value.to_string();
        let params = vec![("ids", weibo_id), ("visible", visible_str.as_str())];

        //println!("\n[DEBUG] 设置微博 {} 的隐私，参数: ids={}, visible={}", weibo_id, weibo_id, visible_str);

        for retry in 0..MAX_RETRIES {
            let request = self
                .client
                .post(url)
                .header("Accept", "application/json, text/plain, */*")
                .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("Cookie", &self.cookie)
                .header("X-Xsrf-Token", &self.xsrf_token)
                .header("X-Requested-With", "XMLHttpRequest")
                .header("Referer", "https://weibo.com")
                .header("Origin", "https://weibo.com")
                .header("Client-Version", "3.0.0")
                .header("Sec-Fetch-Dest", "empty")
                .header("Sec-Fetch-Mode", "cors")
                .header("Sec-Fetch-Site", "same-origin")
                .form(&params);

            match request.send().await {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        // 尝试解析响应
                        let text = response.text().await?;

                        // 打印响应内容用于调试
                        println!("\n[DEBUG] 微博 {} 响应: {}", weibo_id, &text[..std::cmp::min(200, text.len())]);

                        // 微博 API 可能返回不同格式，我们尝试解析
                        if let Ok(privacy_resp) = serde_json::from_str::<PrivacyResponse>(&text) {
                            if let Some(ok) = privacy_resp.ok {
                                if ok == 1 {
                                    return Ok(());
                                } else {
                                    return Err(anyhow!(
                                        "设置失败: {}",
                                        privacy_resp.msg.unwrap_or_else(|| "未知错误".to_string())
                                    ));
                                }
                            }
                        }

                        // 如果成功但无法解析，也视为成功
                        return Ok(());
                    }

                    if retry == MAX_RETRIES - 1 {
                        let error_body = response.text().await.unwrap_or_default();
                        println!("\n[DEBUG] HTTP 错误 {}: {}", status, &error_body[..std::cmp::min(500, error_body.len())]);
                        return Err(anyhow!("HTTP error {}: {}", status, error_body));
                    }
                }
                Err(e) => {
                    if retry == MAX_RETRIES - 1 {
                        return Err(anyhow!("请求失败: {}", e));
                    }
                }
            }

            // 指数退避
            let delay = Duration::from_secs(2u64.pow(retry));
            tokio::time::sleep(delay).await;
        }

        unreachable!()
    }

    /// 带重试的 GET 请求
    async fn get_with_retry(&self, url: &str, user_id: &str) -> Result<Response> {
        for retry in 0..MAX_RETRIES {
            let request = self
                .client
                .get(url)
                .header("Accept", "application/json, text/plain, */*")
                .header("Accept-Language", "zh-CN,zh;q=0.9")
                .header("Referer", format!("https://weibo.com/u/{}", user_id))
                .header("X-Requested-With", "XMLHttpRequest")
                .header("Cookie", &self.cookie)
                .header("X-Xsrf-Token", &self.xsrf_token)
                .header("Accept-Encoding", "gzip, deflate, br, zstd")
                .header("Client-Version", "v2.47.139")
                .header("Sec-Fetch-Dest", "empty")
                .header("Sec-Fetch-Mode", "cors")
                .header("Sec-Fetch-Site", "same-origin");

            match request.send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response);
                    }

                    if retry == MAX_RETRIES - 1 {
                        let status = response.status();
                        let error_body = response.text().await.unwrap_or_default();
                        return Err(anyhow!("HTTP error {}: {}", status, error_body));
                    }
                }
                Err(e) => {
                    if retry == MAX_RETRIES - 1 {
                        return Err(anyhow!("Failed to request: {}", e));
                    }
                }
            }

            let delay = Duration::from_secs(2u64.pow(retry));
            tokio::time::sleep(delay).await;
        }

        unreachable!()
    }
}
