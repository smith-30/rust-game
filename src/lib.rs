use serde::Deserialize;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[macro_use]
mod browser;

// JSON のデシリアライズのターゲットとして Sheetを使えるようにする
#[derive(Deserialize)]
struct Sheet {
    frames: HashMap<String, Cell>,
}

#[derive(Deserialize)]
struct Rect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}
#[derive(Deserialize)]
struct Cell {
    frame: Rect,
}

// [重要]
// JsValue は JavaScript から直接渡される値すべてを表す型だ。
// Rust のコードでは一般に、この型のオブ ジェクトを特定の Rust 型に変換して使用する。

// unwrap(https://doc.rust-jp.rs/rust-by-example-ja/error/option_unwrap.html)
// `unwrap` returns a `panic` when it receives a `None`.
// `unwrap`を使用すると値が`None`だった際に`panic`を返します。

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let window = browser::window().expect("No Window Found");
    let document = browser::document().expect("No Document Found");
    let canvas: web_sys::HtmlCanvasElement = document
        .get_element_by_id("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>() // get_element_by_id　で取得する Element を cast しないといけない。返り値が、Option<Element> のため
        .unwrap();

    let context = browser::context().expect("Could not get browser context");

    // spawn_localを呼び出す際には、引数として asyncの付いたブロックを渡す必要がある
    // このブロックに move を付けているのは、ブロック 内部で参照している変数束縛のすべての所有権をこのブロックに与えるためだ。
    // future の考え方。https://blog.tiqwab.com/2022/03/26/rust-future.html
    // トレイトが用意されていて、ランタイムはライブラリとして提供されているものを使うっていうのが面白い
    browser::spawn_local(async move {
        let sheet: Sheet = browser::fetch_json("rhb.json")
            .await
            .expect("Could not fetch rhb.json")
            .into_serde()
            .expect("Could not convert rhb.json into a Sheet structure");

        // Rust では let 文を使うと、その変数の以前のバージョンを隠して新しく束縛を作り直すので、変数名を変更する必要はない。
        let (success_tx, success_rx) = futures::channel::oneshot::channel::<Result<(), JsValue>>();
        let success_tx = Rc::new(Mutex::new(Some(success_tx)));
        let error_tx = Rc::clone(&success_tx);

        let image = web_sys::HtmlImageElement::new().unwrap();
        // 画像のソースを指定した直後に画像を表示することはできない。
        // 画像がまだロードできていないから。
        // ロードを待つには、HtmlImageElementの onloadコールバックを使う必要がある。

        // Mutex の中身を外に移動することなく、中にある Sender にアクセスするために Option<T> 型を使う.
        // 同じ Mutexを別のスレッドがアクセスすると、Noneが返されるので適切に処 理することができる。
        let callback = Closure::once(move || {
            if let Some(success_tx) = success_tx.lock().ok().and_then(|mut opt| opt.take()) {
                success_tx.send(Ok(()));
            };
        });
        let error_callback = Closure::once(move |err| {
            if let Some(error_tx) = error_tx.lock().ok().and_then(|mut opt| opt.take()) {
                error_tx.send(Err(err));
            }
        });
        // callbackに対して as_refを呼び出している。
        // この関数は生の JsValueを返すので、これに対し て unchecked_refを呼び出して &Functionオブジェクトに変換する。
        // 引数は JavaScript では nullである可 能性があるので、このオブジェクトを Some でラップする。
        // Todo: as_ref() で返ってくるのは JsValue で、unchecked_ref で Function になるの謎。。
        //       呪文ぽい。https://rustwasm.github.io/wasm-bindgen/examples/closures.html
        image.set_onload(Some(callback.as_ref().unchecked_ref()));
        // 数行後に↑関数が終了して callback がスコープから外れたときにクロージャが破壊され、console err になる

        image.set_onerror(Some(error_callback.as_ref().unchecked_ref()));

        image.set_src("rhb.png");
        success_rx.await;

        let mut frame = -1;

        // 繰り返し処理用のClosure. once ではいので何度も呼び出せる
        let interval_callback = Closure::wrap(Box::new(move || {
            frame = (frame + 1) % 8;
            let frame_name = format!("Run ({}).png", frame + 1);

            context.clear_rect(0.0, 0.0, 600.0, 600.0);
            let sprite = sheet.frames.get(&frame_name).expect("Cell not found");
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &image,
                sprite.frame.x.into(),
                sprite.frame.y.into(),
                sprite.frame.w.into(),
                sprite.frame.h.into(),
                300.0,
                300.0,
                sprite.frame.w.into(),
                sprite.frame.h.into(),
            );
        }) as Box<dyn FnMut()>);

        // 50ms ごとに interval_callback を呼び出す
        browser::window()
            .unwrap()
            .set_interval_with_callback_and_timeout_and_arguments_0(
                interval_callback.as_ref().unchecked_ref(),
                50,
            );

        // このフューチャのスコープから 離れる際に Rust がクロージャを破棄しないようになる
        interval_callback.forget();
    });
    Ok(())
}
