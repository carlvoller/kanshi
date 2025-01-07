use std::sync::{Arc, OnceLock};

use futures::StreamExt;
use kanshi::{
    FileSystemEventType, FileSystemTargetKind, Kanshi, KanshiEngines, KanshiImpl, KanshiOptions,
};
use neon::prelude::*;
use tokio::runtime::Runtime;

struct KanshiJS {
    engine: Kanshi,
}

impl Finalize for KanshiJS {
    fn finalize<'a, C: Context<'a>>(self, _: &mut C) {
        self.engine.close();
    }
}

fn runtime<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Runtime> {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();

    if let Some(rt) = RUNTIME.get() {
        Ok(rt)
    } else {
        let rt = Runtime::new();
        if let Ok(rt) = rt {
            let _ = RUNTIME.set(rt);
            Ok(RUNTIME.get().unwrap())
        } else {
            cx.throw_error(rt.err().unwrap().to_string())
        }
    }
}

impl KanshiJS {
    fn js_new(mut cx: FunctionContext) -> JsResult<JsBox<KanshiJS>> {
        let js_opts = cx.argument::<JsObject>(0)?;
        let mut kanshi_opts = KanshiOptions { force_engine: None };

        if let Ok(Some(force_engine)) = js_opts.get_opt::<JsString, _, _>(&mut cx, "forceEngine") {
            if let Ok(force_engine_str) = force_engine.to_string(&mut cx) {
                let utf16_str = String::from_utf16_lossy(&force_engine_str.to_utf16(&mut cx));
                let engine = KanshiEngines::from(&utf16_str);
                if let Ok(engine) = engine {
                    kanshi_opts.force_engine = Some(engine);
                } else {
                    return cx.throw_type_error(engine.err().unwrap().to_string());
                }
            } else {
                return cx.throw_type_error("'forceEngine' should be a String");
            }
        }

        let kanshi = Kanshi::new(kanshi_opts);
        if let Ok(kanshi) = kanshi {
            Ok(cx.boxed(KanshiJS { engine: kanshi }))
        } else {
            cx.throw_error(kanshi.err().unwrap().to_string())
        }
    }

    fn js_watch(mut cx: FunctionContext) -> JsResult<JsPromise> {
        let dir = cx.argument::<JsString>(0)?.value(&mut cx);
        let kanshi = (cx.this::<JsBox<KanshiJS>>()?).engine.clone();
        let (deferred, promise) = cx.promise();

        let rt = runtime(&mut cx)?;
        let channel = cx.channel();

        rt.spawn(async move {
            let watch_ret = kanshi.watch(&dir).await;

            deferred.settle_with(&channel, move |mut cx| {
                if let Err(e) = watch_ret {
                    cx.throw_error(e.to_string())
                } else {
                    Ok(cx.undefined())
                }
            });
        });

        Ok(promise)
    }

    fn js_start(mut cx: FunctionContext) -> JsResult<JsPromise> {
        let kanshi_js = cx.this::<JsBox<KanshiJS>>()?;
        let js_callback = Arc::new(cx.argument::<JsFunction>(0)?.root(&mut cx));

        let channel = cx.channel();
        let sub_thread_channel = cx.channel();
        let (deferred, promise) = cx.promise();
        let rt = runtime(&mut cx)?;

        let kanshi = kanshi_js.engine.clone();

        // Create a single stream to use for all callbacks.
        let mut stream = kanshi.get_events_stream();

        rt.spawn(async move {
            while let Some(event) = stream.next().await {
                let callback = js_callback.clone();
                let handle = sub_thread_channel
                    .send(move |mut cx| {
                        // let cbs = callbacks.as_ref();
                        let this = cx.undefined();

                        let js_event = JsObject::new(&mut cx);
                        let js_event_target = JsObject::new(&mut cx);

                        let event_type = &event.event_type;
                        let event_type_str = match event_type {
                            FileSystemEventType::MovedFrom(path) => {
                                let js_string = JsString::new(&mut cx, path.to_str().unwrap());
                                js_event_target.set(&mut cx, "previousPath", js_string)?;
                                event.event_type.to_string()
                            }
                            FileSystemEventType::MovedTo(path) => {
                                let js_string = JsString::new(&mut cx, path.to_str().unwrap());
                                js_event_target.set(&mut cx, "nextPath", js_string)?;
                                event.event_type.to_string()
                            }
                            x => x.to_string(),
                        };

                        let js_string = JsString::new(&mut cx, event_type_str);
                        js_event.set(&mut cx, "eventType", js_string)?;

                        if let Some(target) = event.target {
                            let js_string = JsString::new(&mut cx, target.path.to_str().unwrap());
                            js_event_target.set(&mut cx, "path", js_string)?;

                            let kind = match target.kind {
                                FileSystemTargetKind::Directory => {
                                    JsString::new(&mut cx, "directory")
                                }
                                FileSystemTargetKind::File => JsString::new(&mut cx, "file"),
                            };
                            js_event_target.set(&mut cx, "kind", kind)?;
                        }

                        js_event.set(&mut cx, "target", js_event_target)?;

                        let js_event_as_value = js_event.as_value(&mut cx);

                        callback
                            .to_inner(&mut cx)
                            .call(&mut cx, this, [js_event_as_value])?;

                        Ok(())
                    })
                    .join();

                if let Err(e) = handle {
                    println!("{:?}", e);
                }
            }
        });

        rt.spawn(async move {
            let ret = kanshi.start().await;
            deferred.settle_with(&channel, move |mut cx| {
                if let Err(e) = ret {
                    cx.throw_error(e.to_string())
                } else {
                    Ok(cx.undefined())
                }
            })
        });

        Ok(promise)
    }

    fn js_close(mut cx: FunctionContext) -> JsResult<JsBoolean> {
        let kanshi_js = cx.this::<JsBox<KanshiJS>>()?;
        let ret = kanshi_js.engine.close();

        Ok(cx.boolean(ret))
    }
}
#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("kanshiNew", KanshiJS::js_new)?;
    cx.export_function("kanshiWatch", KanshiJS::js_watch)?;
    cx.export_function("kanshiStart", KanshiJS::js_start)?;
    cx.export_function("kanshiClose", KanshiJS::js_close)?;
    Ok(())
}
