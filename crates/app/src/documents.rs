//! Single in-flight Android document request. JNI callbacks never touch GPUI.
use futures_channel::oneshot;
use jni::objects::{JObject, JValue};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

type Reply = Result<Option<String>, String>;
type Pending = Option<(u64, oneshot::Sender<Reply>)>;
static REQUEST: Mutex<Pending> = Mutex::new(None);
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub async fn choose(export: Option<String>) -> Reply {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let (sender, receiver) = oneshot::channel();
    {
        let mut pending = REQUEST.lock().unwrap();
        if pending.is_some() {
            return Err("另一个文件操作尚未结束".into());
        }
        *pending = Some((id, sender));
    }
    let started = gpui_mobile::android::jni::with_env(|env| {
        let activity = gpui_mobile::android::jni::activity(env)?;
        let json = match export {
            Some(text) => JObject::from(env.new_string(text).map_err(|e| e.to_string())?),
            None => JObject::null(),
        };
        let filename = env
            .new_string(format!(
                "小钱包-{}.json",
                chrono::Local::now().format("%Y%m%d-%H%M%S")
            ))
            .map_err(|e| e.to_string())?;
        env.call_method(
            &activity,
            jni::jni_str!("chooseWalletDocument"),
            jni::jni_sig!("(JLjava/lang/String;Ljava/lang/String;)V"),
            &[
                JValue::Long(id as i64),
                JValue::Object(&json),
                JValue::Object(&filename),
            ],
        )
        .map_err(|_| {
            env.exception_clear();
            "无法打开文件选择器".to_owned()
        })?;
        Ok(())
    });
    if let Err(error) = started {
        REQUEST.lock().unwrap().take();
        return Err(error);
    }
    receiver
        .await
        .unwrap_or_else(|_| Err("文件操作已取消".into()))
}

/// # Safety
/// Called by Android with a valid string and JNI call frame.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_app_jizhang_wallet_WalletDocuments_nativeResult(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    request: i64,
    kind: i32,
    content: *mut std::ffi::c_void,
) {
    let _ = gpui_mobile::android::jni::with_env(|env| {
        let text = unsafe { JObject::from_raw(env, content as jni::sys::jobject) };
        let content = gpui_mobile::android::jni::get_string(env, &text);
        let mut pending = REQUEST.lock().unwrap();
        if pending
            .as_ref()
            .is_some_and(|(id, _)| *id == request as u64)
        {
            let (_, sender) = pending.take().unwrap();
            let result = match kind {
                0 => Ok(None),
                1 | 2 => Ok(Some(content)),
                _ => Err(content),
            };
            let _ = sender.send(result);
        }
        Ok(())
    });
}

pub fn set_input_type(decimal: bool) {
    let _ = gpui_mobile::android::jni::with_env(|env| {
        let activity = gpui_mobile::android::jni::activity(env)?;
        env.call_method(
            &activity,
            jni::jni_str!("setWalletInputType"),
            jni::jni_sig!("(I)V"),
            &[JValue::Int(if decimal { 5 } else { 0 })],
        )
        .map_err(|e| {
            env.exception_clear();
            e.to_string()
        })?;
        Ok(())
    });
}

/// Showing an existing field must not reset its composition/session.
pub fn show_keyboard() {
    let _ = gpui_mobile::android::jni::with_env(|env| {
        let activity = gpui_mobile::android::jni::activity(env)?;
        env.call_method(
            &activity,
            jni::jni_str!("showWalletKeyboard"),
            jni::jni_sig!("()V"),
            &[],
        )
        .map_err(|e| {
            env.exception_clear();
            e.to_string()
        })?;
        Ok(())
    });
}
