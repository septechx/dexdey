use std::collections::HashMap;

use jni::Env;
use jni::objects::{Global, JClass, JObject, JObjectArray, JString, JValue};
use jni::signature::RuntimeMethodSignature;
use jni::strings::JNIString;
use jni::vm::JavaVM;
use jni::{jni_sig, jni_str};

#[derive(Debug)]
pub(crate) struct RegisteredHandler {
    pub plugin_id: String,
    pub instance: Global<JObject<'static>>,
    pub method_name: String,
    pub method_sig: String,
}

#[derive(Debug, Default)]
pub(crate) struct EventBus {
    handlers: HashMap<String, Vec<RegisteredHandler>>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum EventBusError {
    #[error("JNI error: {0}")]
    Jni(#[from] jni::errors::Error),
    #[error("Handler error for plugin {plugin_id}: {message}")]
    HandlerError { plugin_id: String, message: String },
}

type Result<T> = std::result::Result<T, EventBusError>;

impl EventBus {
    pub fn new() -> Self {
        EventBus {
            handlers: HashMap::new(),
        }
    }

    pub fn register_plugin(
        &mut self,
        env: &mut Env,
        id: &str,
        instance: &Global<JObject<'static>>,
    ) -> Result<()> {
        let class = env
            .call_method(
                instance,
                jni_str!("getClass"),
                jni_sig!("()Ljava/lang/Class;"),
                &[],
            )?
            .l()?;

        let methods_arr = env
            .call_method(
                &class,
                jni_str!("getDeclaredMethods"),
                jni_sig!("()[Ljava/lang/reflect/Method;"),
                &[],
            )?
            .l()?;

        let methods = unsafe {
            JObjectArray::<JObject>::from_raw(env, methods_arr.into_raw() as jni::sys::jobjectArray)
        };
        let len = methods.len(env)?;

        if len == 0 {
            return Ok(());
        }

        let subscribe_class =
            env.find_class(jni_str!("com/velocitypowered/api/event/Subscribe"))?;

        for i in 0..len {
            let method = methods.get_element(env, i)?;

            let has_annotation = env
                .call_method(
                    &method,
                    jni_str!("isAnnotationPresent"),
                    jni_sig!("(Ljava/lang/Class;)Z"),
                    &[JValue::Object(&subscribe_class)],
                )?
                .z()?;

            if !has_annotation {
                continue;
            }

            let name_obj = env
                .call_method(
                    &method,
                    jni_str!("getName"),
                    jni_sig!("()Ljava/lang/String;"),
                    &[],
                )?
                .l()?;

            let method_name = env.as_cast::<JString>(&name_obj)?.try_to_string(env)?;

            let params_arr = env
                .call_method(
                    &method,
                    jni_str!("getParameterTypes"),
                    jni_sig!("()[Ljava/lang/Class;"),
                    &[],
                )?
                .l()?;

            let params = unsafe {
                JObjectArray::<JObject>::from_raw(
                    env,
                    params_arr.into_raw() as jni::sys::jobjectArray,
                )
            };
            let param_count = params.len(env)?;

            if param_count != 1 {
                continue;
            }

            let param = params.get_element(env, 0)?;
            let param_name_obj = env
                .call_method(
                    &param,
                    jni_str!("getName"),
                    jni_sig!("()Ljava/lang/String;"),
                    &[],
                )?
                .l()?;

            let event_class = env
                .as_cast::<JString>(&param_name_obj)?
                .try_to_string(env)?;

            let jni_sig_str = format!("(L{};)V", event_class.replace('.', "/"));

            let new_ref = env.new_global_ref(instance)?;

            self.handlers
                .entry(event_class)
                .or_default()
                .push(RegisteredHandler {
                    plugin_id: id.to_string(),
                    instance: new_ref,
                    method_name,
                    method_sig: jni_sig_str,
                });
        }

        Ok(())
    }

    pub fn fire(&self, env: &mut Env, event: &JObject, event_class: &str) -> Vec<EventBusError> {
        let Some(handlers) = self.handlers.get(event_class) else {
            return vec![];
        };

        let mut errors = vec![];

        for handler in handlers {
            let sig = match RuntimeMethodSignature::from_str(&handler.method_sig) {
                Ok(s) => s,
                Err(e) => {
                    errors.push(EventBusError::HandlerError {
                        plugin_id: handler.plugin_id.clone(),
                        message: e.to_string(),
                    });
                    continue;
                }
            };

            if let Err(e) = env.call_method(
                &handler.instance,
                JNIString::from(&handler.method_name),
                sig.method_signature(),
                &[JValue::Object(event)],
            ) {
                errors.push(EventBusError::HandlerError {
                    plugin_id: handler.plugin_id.clone(),
                    message: e.to_string(),
                });
            }
        }

        errors
    }

    pub fn fire_event(&self, env: &mut Env, event: Event) -> Vec<EventBusError> {
        self.fire(env, &event.event, &event.event_class)
    }
}

pub(crate) struct Event {
    pub event_class: String,
    pub event: Global<JObject<'static>>,
}

impl Event {
    pub fn proxy_initialize() -> core::result::Result<Event, jni::errors::Error> {
        Event::new(
            "com.velocitypowered.api.event.proxy.ProxyInitializeEvent",
            |env, class| {
                let obj = env.new_object(class, jni_sig!("()V"), &[])?;
                env.new_global_ref(&obj)
            },
        )
    }

    fn new<T: From<jni::errors::Error>>(
        class_name: &str,
        event: impl FnOnce(&mut Env, JClass) -> core::result::Result<Global<JObject<'static>>, T>,
    ) -> core::result::Result<Event, T> {
        let jvm = JavaVM::singleton()?;
        jvm.attach_current_thread(|env| -> core::result::Result<Event, T> {
            let class = env.find_class(JNIString::from(class_name.replace('.', "/")))?;
            event(env, class).map(|global| Event {
                event_class: class_name.to_string(),
                event: global,
            })
        })
    }
}

pub(crate) fn fire_event<T: From<jni::errors::Error>>(
    f: impl FnOnce(&mut Env) -> core::result::Result<Vec<EventBusError>, T>,
) -> core::result::Result<(), T> {
    let jvm = JavaVM::singleton()?;
    jvm.attach_current_thread(|env| -> core::result::Result<(), T> {
        let errors = f(env)?;

        for err in &errors {
            eprintln!("{err}");
        }

        Ok(())
    })?;

    Ok(())
}
