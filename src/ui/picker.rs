//! The picker window shown when a link is clicked.

use std::rc::Rc;

use crate::browsers::{self};

pub fn show(url: String) {
    nwg::init().expect("Failed to init Native Windows GUI");

    let me = std::env::current_exe().ok();
    let list = browsers::detect(me.as_deref());

    if list.is_empty() {
        nwg::simple_message(
            crate::PRODUCT_NAME,
            "No browsers were detected on this system.",
        );
        return;
    }

    let mut font = nwg::Font::default();
    let _ = nwg::Font::builder().size(16).family("Segoe UI").build(&mut font);

    // Layout (client-area coordinates).
    let pad = 12i32;
    let btn_h = 40i32;
    let gap = 6i32;
    let btn_w = 340i32;
    let footer_h = 22i32;
    let count = list.len() as i32;
    let width = btn_w + pad * 2;
    // Extra height leaves room for the title bar (outer vs client) so nothing clips.
    let height = pad + count * (btn_h + gap) + footer_h + pad + 56;

    let (x, y) = crate::ui::centered_position(width, height);

    let mut window = nwg::Window::default();
    nwg::Window::builder()
        .size((width, height))
        .position((x, y))
        .title("Open link in…")
        .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
        .build(&mut window)
        .expect("Failed to build window");

    let mut buttons: Vec<nwg::Button> = Vec::with_capacity(list.len());
    for (i, b) in list.iter().enumerate() {
        // "&1" makes Alt+1 an accelerator that clicks the button.
        let label = if i < 9 {
            format!("&{}   {}", i + 1, b.name)
        } else {
            b.name.clone()
        };
        let mut btn = nwg::Button::default();
        nwg::Button::builder()
            .text(&label)
            .parent(&window)
            .font(Some(&font))
            .position((pad, pad + i as i32 * (btn_h + gap)))
            .size((btn_w, btn_h))
            .build(&mut btn)
            .expect("Failed to build button");
        buttons.push(btn);
    }

    let mut footer = nwg::Label::default();
    let _ = nwg::Label::builder()
        .text(&format!(
            "{count} browser(s)  ·  Alt+number to pick  ·  Esc or ✕ to cancel"
        ))
        .parent(&window)
        .font(Some(&font))
        .position((pad, pad + count * (btn_h + gap)))
        .size((btn_w, footer_h))
        .build(&mut footer);

    let list_rc = Rc::new(list);
    let buttons_rc = Rc::new(buttons);
    let url_rc = Rc::new(url);

    let handler = nwg::full_bind_event_handler(&window.handle, move |evt, evt_data, handle| {
        match evt {
            nwg::Event::OnButtonClick => {
                for (i, btn) in buttons_rc.iter().enumerate() {
                    if handle == btn.handle {
                        if let Err(e) = browsers::launch(&list_rc[i], &url_rc) {
                            nwg::simple_message(
                                crate::PRODUCT_NAME,
                                &format!("Couldn't launch {}:\n{e}", list_rc[i].name),
                            );
                        }
                        nwg::stop_thread_dispatch();
                        break;
                    }
                }
            }
            nwg::Event::OnKeyPress => {
                if let nwg::EventData::OnKey(key) = evt_data {
                    if key == 0x1B {
                        // VK_ESCAPE
                        nwg::stop_thread_dispatch();
                    }
                }
            }
            nwg::Event::OnWindowClose => nwg::stop_thread_dispatch(),
            _ => {}
        }
    });

    nwg::dispatch_thread_events();
    nwg::unbind_event_handler(&handler);
}
