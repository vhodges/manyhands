use gpui_kit::component::button::*;
use gpui_kit::component::*;
use gpui_kit::*;

struct HelloWorld;

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("hello")
                    .primary()
                    .label("Say hello")
                    .on_click(|_, _, _| println!("Hello from Manyhands!")),
            )
    }
}

fn main() {
    // SAFETY: this is the first action, before GPUI creates background threads.
    if unsafe { manyhands::runtime::initialize_git_transport_before_threads() }.is_err() {
        eprintln!("Git transport initialization failed");
        std::process::exit(1);
    }
    let app = gpui_kit::application().with_assets(gpui_kit::assets::Assets);

    app.run(move |cx| {
        gpui_kit::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| HelloWorld);
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("failed to open Manyhands window");
        })
        .detach();
    });
}
