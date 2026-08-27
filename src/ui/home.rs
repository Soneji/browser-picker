//! The home / settings window shown when the app is opened directly (no URL).

use crate::{browsers, register};

pub fn show() {
    nwg::init().expect("Failed to init Native Windows GUI");

    let me = std::env::current_exe().ok();
    let list = browsers::detect(me.as_deref());

    let mut font = nwg::Font::default();
    let _ = nwg::Font::builder().size(16).family("Segoe UI").build(&mut font);

    let width = 440i32;
    let height = 420i32;
    let (x, y) = crate::ui::centered_position(width, height);

    let mut window = nwg::Window::default();
    nwg::Window::builder()
        .size((width, height))
        .position((x, y))
        .title(crate::PRODUCT_NAME)
        .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
        .build(&mut window)
        .expect("Failed to build window");

    let status = if register::is_registered() {
        format!(
            "{} is registered.\nSet it as default in Settings ▸ Apps ▸ Default apps.",
            crate::PRODUCT_NAME
        )
    } else {
        format!(
            "{} is not registered yet — click \"Set as default browser\".",
            crate::PRODUCT_NAME
        )
    };

    let mut status_label = nwg::Label::default();
    nwg::Label::builder()
        .text(&status)
        .parent(&window)
        .font(Some(&font))
        .position((14, 12))
        .size((width - 28, 44))
        .build(&mut status_label)
        .expect("Failed to build label");

    let items: Vec<String> = if list.is_empty() {
        vec!["No browsers detected".to_string()]
    } else {
        list.iter()
            .enumerate()
            .map(|(i, b)| format!("{}.   {}", i + 1, b.name))
            .collect()
    };
    let mut listbox: nwg::ListBox<String> = nwg::ListBox::default();
    nwg::ListBox::builder()
        .collection(items)
        .parent(&window)
        .font(Some(&font))
        .position((14, 64))
        .size((width - 28, 190))
        .build(&mut listbox)
        .expect("Failed to build list box");

    let mut set_btn = nwg::Button::default();
    nwg::Button::builder()
        .text("Set as default browser")
        .parent(&window)
        .font(Some(&font))
        .position((14, 268))
        .size((width - 28, 44))
        .build(&mut set_btn)
        .expect("Failed to build button");

    let half = (width - 28 - 8) / 2;
    let mut reg_btn = nwg::Button::default();
    nwg::Button::builder()
        .text("Register")
        .parent(&window)
        .font(Some(&font))
        .position((14, 320))
        .size((half, 36))
        .build(&mut reg_btn)
        .expect("Failed to build button");
    let mut unreg_btn = nwg::Button::default();
    nwg::Button::builder()
        .text("Unregister")
        .parent(&window)
        .font(Some(&font))
        .position((14 + half + 8, 320))
        .size((half, 36))
        .build(&mut unreg_btn)
        .expect("Failed to build button");

    let set_h = set_btn.handle;
    let reg_h = reg_btn.handle;
    let unreg_h = unreg_btn.handle;

    let handler =
        nwg::full_bind_event_handler(&window.handle, move |evt, _evt_data, handle| match evt {
            nwg::Event::OnButtonClick => {
                if handle == set_h {
                    match register::register() {
                        Ok(_) => {
                            open_default_apps();
                            nwg::simple_message(
                                crate::PRODUCT_NAME,
                                "Opened Windows Default Apps.\n\nFind \"Browser Picker\" in the \
                                 list and set it for HTTP and HTTPS.",
                            );
                        }
                        Err(e) => {
                            nwg::simple_message(
                                crate::PRODUCT_NAME,
                                &format!("Registration failed:\n{e}"),
                            );
                        }
                    }
                } else if handle == reg_h {
                    match register::register() {
                        Ok(_) => {
                            nwg::simple_message(crate::PRODUCT_NAME, "Registered.");
                        }
                        Err(e) => {
                            nwg::simple_message(crate::PRODUCT_NAME, &format!("Failed:\n{e}"));
                        }
                    }
                } else if handle == unreg_h {
                    let _ = register::unregister();
                    nwg::simple_message(crate::PRODUCT_NAME, "Unregistered.");
                }
            }
            nwg::Event::OnWindowClose => nwg::stop_thread_dispatch(),
            _ => {}
        });

    nwg::dispatch_thread_events();
    nwg::unbind_event_handler(&handler);
}

fn open_default_apps() {
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", "ms-settings:defaultapps"])
        .spawn();
}
