use rand::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::console;

// unwrap(https://doc.rust-jp.rs/rust-by-example-ja/error/option_unwrap.html)
// `unwrap` returns a `panic` when it receives a `None`.
// `unwrap`を使用すると値が`None`だった際に`panic`を返します。

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    // js の console 名前空間を模したもの
    console::log_1(&JsValue::from_str("Hello world!"));

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas: web_sys::HtmlCanvasElement = document
        .get_element_by_id("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>() // get_element_by_id　で取得する Element を cast しないといけない。返り値が、Option<Element> のため
        .unwrap();

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    // spawn_localを呼び出す際には、引数として asyncの付いたブロックを渡す必要がある
    // このブロックに move を付けているのは、ブロック 内部で参照している変数束縛のすべての所有権をこのブロックに与えるためだ。
    // future の考え方。https://blog.tiqwab.com/2022/03/26/rust-future.html
    // トレイトが用意されていて、ランタイムはライブラリとして提供されているものを使うっていうのが面白い
    wasm_bindgen_futures::spawn_local(async move {
        // oneshot チャネルは、レシーバが Futureトレイト を実装するチャネルで、メッセージを受け取るのを awaitで待つことができる。
        // onloadコールバックがそ のチャネルにメッセージを送るように設定すれば、レシーバで await することで、画像がロードされるまで実行を停止することができる。
        let (success_tx, success_rx) = futures::channel::oneshot::channel::<()>();
        let image = web_sys::HtmlImageElement::new().unwrap();
        // 画像のソースを指定した直後に画像を表示することはできないのだ。
        // 画像がまだロードできていないからだ。
        // ロードを待つには、HtmlImageElementの onloadコールバックを使う必要がある。

        let callback = Closure::once(move || {
            success_tx.send(());
        });
        // callbackに対して as_refを呼び出している。
        // この関数は生の JsValueを返すので、これに対し て unchecked_refを呼び出して &Functionオブジェクトに変換する。
        // 引数は JavaScript では nullである可 能性があるので、このオブジェクトを Some でラップする。
        // Todo: as_ref() で返ってくるのは JsValue で、unchecked_ref で Function になるの謎。。
        //       呪文ぽい。https://rustwasm.github.io/wasm-bindgen/examples/closures.html
        image.set_onload(Some(callback.as_ref().unchecked_ref()));
        // 数行後に↑関数が終了して callback がスコープから外れたときにクロージャが破壊され、console err になる

        image.set_src("Idle (1).png");
        success_rx.await;
        context.draw_image_with_html_image_element(&image, 0.0, 0.0);

        sierpinski(
            &context,
            [(300.0, 0.0), (0.0, 600.0), (600.0, 600.0)],
            (0, 255, 0),
            5,
        );
    });
    Ok(())
}

fn draw_triangle(
    context: &web_sys::CanvasRenderingContext2d,
    points: [(f64, f64); 3],
    color: (u8, u8, u8),
) {
    let color_str = format!("rgb({}, {}, {})", color.0, color.1, color.2);
    context.set_fill_style(&wasm_bindgen::JsValue::from_str(&color_str));

    let [top, left, right] = points;
    context.move_to(top.0, top.1);
    context.begin_path();
    context.line_to(left.0, left.1);
    context.line_to(right.0, right.1);
    context.line_to(top.0, top.1);
    context.close_path();
    context.stroke();
    context.fill()
}

fn sierpinski(
    context: &web_sys::CanvasRenderingContext2d,
    points: [(f64, f64); 3],
    color: (u8, u8, u8),
    depth: u8,
) {
    let mut rng = thread_rng();

    draw_triangle(&context, points, color);
    let depth = depth - 1;
    let [top, left, right] = points;
    if depth > 0 {
        let next_color = (
            rng.gen_range(0..255),
            rng.gen_range(0..255),
            rng.gen_range(0..255),
        );
        let left_middle = midpoint(top, left);
        let right_middle = midpoint(top, right);
        let bottom_middle = midpoint(left, right);
        sierpinski(
            &context,
            [top, left_middle, right_middle],
            next_color,
            depth,
        );
        sierpinski(
            &context,
            [left_middle, left, bottom_middle],
            next_color,
            depth,
        );
        sierpinski(
            &context,
            [right_middle, bottom_middle, right],
            next_color,
            depth,
        );
    }
}

fn midpoint(point_1: (f64, f64), point_2: (f64, f64)) -> (f64, f64) {
    ((point_1.0 + point_2.0) / 2.0, (point_1.1 + point_2.1) / 2.0)
}
