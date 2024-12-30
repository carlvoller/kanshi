use neon::prelude::*;
use kanshi::{Kanshi, KanshiImpl, KanshiOptions};

fn hello(mut cx: FunctionContext) -> JsResult<JsString> {
    let kanshi = Kanshi::new(KanshiOptions { force_engine: None });
    if let Ok(_) = kanshi {
        Ok(cx.string("kanshi worked!"))
    } else {
        let e = kanshi.err().unwrap();
        Ok(cx.string(format!("kanshi not working... {e}")))
    }
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("hello", hello)?;
    Ok(())
}
