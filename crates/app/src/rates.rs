use chrono::{Local, NaiveDate};
use wallet_ledger::Rate;

pub fn fetch() -> Result<Rate, String> {
    let response = gpui_mobile::android::jni::with_env(|env| {
        let activity = gpui_mobile::android::jni::activity(env)?;
        let result = env
            .call_method(
                &activity,
                jni::jni_str!("fetchExchangeRate"),
                jni::jni_sig!("()Ljava/lang/String;"),
                &[],
            )
            .and_then(|value| value.l())
            .map_err(|error| {
                env.exception_describe();
                env.exception_clear();
                log::warn!("Exchange rate request failed: {error}");
                "无法连接汇率服务，请检查网络".to_owned()
            })?;
        Ok(gpui_mobile::android::jni::get_string(env, &result))
    })?;
    let body: serde_json::Value =
        serde_json::from_str(&response).map_err(|_| "汇率服务返回了无效数据".to_owned())?;
    let date: NaiveDate = body["date"]
        .as_str()
        .ok_or("汇率日期缺失")?
        .parse()
        .map_err(|_| "汇率日期无效")?;
    let value = body["rates"]["CNY"].as_f64().ok_or("人民币汇率缺失")?;
    if body["base"] != "USD" || !value.is_finite() || !(0.1..=100.0).contains(&value) {
        return Err("汇率数据无效".into());
    }
    Rate::new(
        date,
        Local::now().date_naive(),
        (value * 1_000_000.0).round() as u64,
    )
    .map_err(|e| e.to_string())
}
