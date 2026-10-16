use iritsi_core::Alert;

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub struct Colors {
    pub accent: u32,
    pub accent_dark: u32,
}

#[cfg(target_os = "android")]
mod android {
    use iritsi_core::{Alert, Line, Phase};
    use jni::{
        objects::{JObject, JValue},
        JNIEnv, JavaVM,
    };
    use serde_json::{json, Value};

    use super::Colors;

    fn argb(rgb: u32) -> i32 {
        (0xff00_0000 | rgb) as i32
    }

    fn with_activity(f: impl FnOnce(&mut JNIEnv, &JObject) -> jni::errors::Result<()>) {
        let ctx = ndk_context::android_context();
        let Ok(vm) = (unsafe { JavaVM::from_raw(ctx.vm().cast()) }) else {
            return;
        };
        let Ok(mut env) = vm.attach_current_thread() else {
            return;
        };
        let activity = unsafe { JObject::from_raw(ctx.context().cast()) };
        if f(&mut env, &activity).is_err() {
            let _ = env.exception_describe();
            let _ = env.exception_clear();
        }
    }

    fn to_json(alert: &Alert, colors: &Colors) -> Value {
        let mut value = json!({
            "at": alert.at,
            "done": alert.phase == Phase::Done,
            "title": alert.title,
            "text": alert.text,
            "chip": alert.chip,
            "buzz": alert.buzz,
            "accent": argb(colors.accent),
        });
        if let Some(countdown_to) = alert.countdown_to {
            value["countdown_to"] = json!(countdown_to);
        }
        if let Some(progress) = &alert.progress {
            let segments: Vec<Value> = progress
                .segments
                .iter()
                .map(|segment| {
                    let color = match segment.line {
                        Some(Line::L1) => argb(colors.accent),
                        Some(Line::L2) => argb(colors.accent_dark),
                        None => argb(0xd0d0d0),
                    };
                    json!({ "length": segment.length.max(1), "color": color })
                })
                .collect();
            value["progress"] = json!({
                "value": progress.value,
                "segments": segments,
                "points": progress.points,
                "point_color": argb(0x000000),
            });
        }
        value
    }

    pub fn schedule(alerts: &[Alert], colors: &Colors) {
        let json =
            Value::Array(alerts.iter().map(|alert| to_json(alert, colors)).collect()).to_string();
        with_activity(|env, activity| {
            let json = env.new_string(json)?;
            env.call_method(
                activity,
                "scheduleAlerts",
                "(Ljava/lang/String;)V",
                &[JValue::Object(&json)],
            )?;
            Ok(())
        });
    }

    pub fn cancel() {
        with_activity(|env, activity| {
            env.call_method(activity, "cancelAlerts", "()V", &[])?;
            Ok(())
        });
    }
}

#[cfg(target_os = "android")]
pub use android::{cancel, schedule};

#[cfg(not(target_os = "android"))]
pub fn schedule(_alerts: &[Alert], _colors: &Colors) {}

#[cfg(not(target_os = "android"))]
pub fn cancel() {}
