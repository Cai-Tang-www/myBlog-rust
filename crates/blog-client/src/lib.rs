use js_sys::{Date, Math};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure, prelude::*};
use web_sys::{
    CanvasRenderingContext2d, Document, Element, Event, HtmlCanvasElement, HtmlElement,
    HtmlInputElement, HtmlScriptElement, HtmlTextAreaElement, MouseEvent, SubmitEvent, Window,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    id: String,
    name: String,
    email: String,
    content: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

fn window() -> Option<Window> {
    web_sys::window()
}

fn document() -> Option<Document> {
    window()?.document()
}

fn base_path() -> String {
    document()
        .and_then(|doc| doc.body())
        .and_then(|body| body.get_attribute("data-base-path"))
        .unwrap_or_default()
}

fn page_kind() -> String {
    document()
        .and_then(|doc| doc.body())
        .and_then(|body| body.get_attribute("data-page-kind"))
        .unwrap_or_default()
}

fn set_hidden(element: &Element, hidden: bool) {
    if hidden {
        let _ = element.set_attribute("hidden", "");
    } else {
        let _ = element.remove_attribute("hidden");
    }
}

fn init_resume_link() {
    let Some(doc) = document() else { return };
    let Ok(Some(link)) = doc.query_selector("[data-resume-link]") else {
        return;
    };
    let callback = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        event.prevent_default();
        if let Some(win) = window() {
            let _ = win.alert_with_message("我还没写好...");
        }
    });
    let _ = link.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref());
    callback.forget();
}

fn init_back_to_top() {
    let Some(doc) = document() else { return };
    let Ok(Some(button)) = doc.query_selector(".back-to-top") else {
        return;
    };
    let button_for_scroll = button.clone();
    let callback = Closure::<dyn FnMut(Event)>::new(move |_| {
        let Some(win) = window() else { return };
        let y = win.scroll_y().unwrap_or_default();
        let total = document()
            .and_then(|doc| doc.document_element())
            .map(|root| {
                (root.scroll_height() as f64
                    - win
                        .inner_height()
                        .ok()
                        .and_then(|v| v.as_f64())
                        .unwrap_or_default())
                .max(0.0)
            })
            .unwrap_or_default();
        let percent = if total > 0.0 {
            ((y / total) * 100.0).round().clamp(0.0, 100.0) as i32
        } else {
            0
        };
        if let Ok(Some(label)) = button_for_scroll.query_selector("span") {
            label.set_text_content(Some(&format!("{percent}%")));
        }
        let _ = button_for_scroll
            .class_list()
            .toggle_with_force("visible", y > 300.0);
    });
    if let Some(win) = window() {
        let _ = win.add_event_listener_with_callback("scroll", callback.as_ref().unchecked_ref());
    }
    callback.forget();

    let click = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Some(win) = window() {
            let options = web_sys::ScrollToOptions::new();
            options.set_top(0.0);
            options.set_behavior(web_sys::ScrollBehavior::Smooth);
            win.scroll_to_with_scroll_to_options(&options);
        }
    });
    let _ = button.add_event_listener_with_callback("click", click.as_ref().unchecked_ref());
    click.forget();
}

fn init_search() {
    if page_kind() != "blog" {
        return;
    }
    let Some(doc) = document() else { return };
    let Some(input) = doc
        .get_element_by_id("search-input")
        .and_then(|node| node.dyn_into::<HtmlInputElement>().ok())
    else {
        return;
    };
    if let Some(win) = window()
        && let Ok(search) = win.location().search()
        && let Ok(params) = web_sys::UrlSearchParams::new_with_str(&search)
        && let Some(query) = params.get("q")
    {
        input.set_value(&query);
    }
    let input_for_event = input.clone();
    let handler = Closure::<dyn FnMut(Event)>::new(move |_| {
        let query = input_for_event.value().trim().to_lowercase();
        let Some(doc) = document() else { return };
        let Ok(cards) = doc.query_selector_all(".grid .post-card") else {
            return;
        };
        let mut visible = 0;
        for index in 0..cards.length() {
            let Some(card) = cards.item(index) else {
                continue;
            };
            let text = card.text_content().unwrap_or_default().to_lowercase();
            let show = query.is_empty() || text.contains(&query);
            if let Some(html) = card.dyn_ref::<HtmlElement>() {
                let _ = html
                    .style()
                    .set_property("display", if show { "" } else { "none" });
            }
            if show {
                visible += 1;
            }
        }
        if let Ok(Some(status)) = doc.query_selector(".search-status") {
            let message = if query.is_empty() {
                "可搜索标题、标签和摘要".to_owned()
            } else if visible == 0 {
                "没有匹配结果".to_owned()
            } else {
                format!("找到 {visible} 篇文章")
            };
            status.set_text_content(Some(&message));
        }
    });
    let _ = input.add_event_listener_with_callback("input", handler.as_ref().unchecked_ref());
    handler.forget();
    if !input.value().is_empty() {
        let _ = input.dispatch_event(
            &Event::new("input").unwrap_or_else(|_| Event::new("change").expect("event")),
        );
    }
    load_pagefind();
}

fn load_pagefind() {
    let Some(doc) = document() else { return };
    if let Ok(Some(status)) = doc.query_selector(".search-status") {
        status.set_text_content(Some("正在加载全文索引…"));
    }
    let Ok(script) = doc.create_element("script") else {
        return;
    };
    let Ok(script) = script.dyn_into::<HtmlScriptElement>() else {
        return;
    };
    script.set_src(&format!("{}/pagefind/pagefind-ui.js", base_path()));
    script.set_async(true);
    let onload = Closure::<dyn FnMut(Event)>::new(move |_| {
        let Some(doc) = document() else { return };
        if let Ok(link) = doc.create_element("link") {
            let _ = link.set_attribute("rel", "stylesheet");
            let _ =
                link.set_attribute("href", &format!("{}/pagefind/pagefind-ui.css", base_path()));
            if let Some(head) = doc.head() {
                let _ = head.append_child(&link);
            }
        }
        let js = r#"
if (window.PagefindUI) {
  const host = document.querySelector('#search');
  const grid = document.querySelector('.grid');
  host.innerHTML = '';
  const pagefind = new window.PagefindUI({
    element: '#search',
    showSubResults: true,
    highlightParam: 'highlight',
    translations: {
      placeholder: '搜索文章标题、标签或正文内容',
      clear_search: '清除',
      zero_results: '没有找到 [SEARCH_TERM] 的结果',
      many_results: '找到 [COUNT] 个结果',
      one_result: '找到 1 个结果'
    }
  });
  const syncGrid = () => {
    const input = host.querySelector('input[type="search"]');
    if (grid) grid.hidden = Boolean(input && input.value.trim());
  };
  host.addEventListener('input', syncGrid);
  const query = new URLSearchParams(window.location.search).get('q');
  if (query) {
    pagefind.triggerSearch(query);
    if (grid) grid.hidden = true;
  } else {
    queueMicrotask(syncGrid);
  }
}
"#;
        let _ = js_sys::eval(js);
    });
    script.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget();

    let onerror = Closure::<dyn FnMut(Event)>::new(move |_| {
        let Some(doc) = document() else { return };
        if let Ok(Some(status)) = doc.query_selector(".search-status") {
            status.set_text_content(Some("全文索引加载失败，已切换到标题、标签和摘要筛选"));
        }
    });
    script.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();

    if let Some(head) = doc.head() {
        let _ = head.append_child(&script);
    }
}

fn field_error(name: &str, message: Option<&str>) {
    let Some(doc) = document() else { return };
    if let Ok(Some(error)) = doc.query_selector(&format!("[data-error-for=\"{name}\"]")) {
        error.set_text_content(message);
        set_hidden(&error, message.is_none());
    }
    if let Some(field) = doc.get_element_by_id(&format!("contact-{name}")) {
        let _ = field
            .class_list()
            .toggle_with_force("inputError", message.is_some());
    }
}

fn render_messages(messages: &[Message]) {
    let Some(doc) = document() else { return };
    let Ok(Some(history)) = doc.query_selector(".history") else {
        return;
    };
    set_hidden(&history, messages.is_empty());
    if let Ok(Some(count)) = doc.query_selector("[data-message-count]") {
        count.set_text_content(Some(&messages.len().to_string()));
    }
    let Ok(Some(list)) = doc.query_selector(".messageList") else {
        return;
    };
    list.set_inner_html("");
    for message in messages {
        let Ok(card) = doc.create_element("article") else {
            continue;
        };
        card.set_class_name("messageCard");
        let Ok(head) = doc.create_element("div") else {
            continue;
        };
        head.set_class_name("messageHead");
        let Ok(name) = doc.create_element("strong") else {
            continue;
        };
        name.set_class_name("messageName");
        name.set_text_content(Some(&message.name));
        let Ok(time) = doc.create_element("time") else {
            continue;
        };
        time.set_class_name("messageTime");
        time.set_text_content(Some(&message.created_at));
        let Ok(body) = doc.create_element("p") else {
            continue;
        };
        body.set_class_name("messageBody");
        body.set_text_content(Some(&message.content));
        let _ = head.append_child(&name);
        let _ = head.append_child(&time);
        let _ = card.append_child(&head);
        let _ = card.append_child(&body);
        let _ = list.append_child(&card);
    }
}

fn read_messages() -> Vec<Message> {
    window()
        .and_then(|win| win.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item("contact_messages").ok().flatten())
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or_default()
}

fn init_contact() {
    if page_kind() != "contact" {
        return;
    }
    let messages = Rc::new(RefCell::new(read_messages()));
    render_messages(&messages.borrow());
    let Some(doc) = document() else { return };
    let Some(content) = doc
        .get_element_by_id("contact-content")
        .and_then(|e| e.dyn_into::<HtmlTextAreaElement>().ok())
    else {
        return;
    };
    let content_for_count = content.clone();
    let count = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Some(doc) = document()
            && let Ok(Some(node)) = doc.query_selector("[data-char-count]")
        {
            node.set_text_content(Some(&content_for_count.value().chars().count().to_string()));
        }
    });
    let _ = content.add_event_listener_with_callback("input", count.as_ref().unchecked_ref());
    count.forget();
    let Some(form) = doc.get_element_by_id("contact-form") else {
        return;
    };
    let messages_for_submit = messages.clone();
    let submit = Closure::<dyn FnMut(SubmitEvent)>::new(move |event: SubmitEvent| {
        event.prevent_default();
        let Some(doc) = document() else { return };
        let name = doc
            .get_element_by_id("contact-name")
            .and_then(|e| e.dyn_into::<HtmlInputElement>().ok());
        let email = doc
            .get_element_by_id("contact-email")
            .and_then(|e| e.dyn_into::<HtmlInputElement>().ok());
        let content = doc
            .get_element_by_id("contact-content")
            .and_then(|e| e.dyn_into::<HtmlTextAreaElement>().ok());
        let (Some(name), Some(email), Some(content)) = (name, email, content) else {
            return;
        };
        let n = name.value().trim().to_owned();
        let e = email.value().trim().to_owned();
        let c = content.value().trim().to_owned();
        let name_error = if n.is_empty() {
            Some("请输入昵称")
        } else {
            None
        };
        let email_error = if e.is_empty() {
            Some("请输入邮箱")
        } else if !e.contains('@') || !e.rsplit('.').next().is_some_and(|tail| tail.len() >= 2) {
            Some("邮箱格式不正确")
        } else {
            None
        };
        let content_error = if c.is_empty() {
            Some("请输入留言内容")
        } else if c.chars().count() < 2 {
            Some("留言内容至少 2 个字符")
        } else {
            None
        };
        field_error("name", name_error);
        field_error("email", email_error);
        field_error("content", content_error);
        if name_error.is_some() || email_error.is_some() || content_error.is_some() {
            return;
        }
        let created = Date::new_0()
            .to_locale_string("zh-CN", &JsValue::UNDEFINED)
            .as_string()
            .unwrap_or_default();
        let message = Message {
            id: Date::now().round().to_string(),
            name: n,
            email: e,
            content: c,
            created_at: created,
        };
        let mut items = messages_for_submit.borrow_mut();
        items.insert(0, message);
        if let Some(storage) = window().and_then(|w| w.local_storage().ok().flatten())
            && let Ok(json) = serde_json::to_string(&*items)
        {
            let _ = storage.set_item("contact_messages", &json);
        }
        render_messages(&items);
        name.set_value("");
        email.set_value("");
        content.set_value("");
        if let Ok(Some(count)) = doc.query_selector("[data-char-count]") {
            count.set_text_content(Some("0"));
        }
        if let Ok(Some(toast)) = doc.query_selector(".successToast") {
            set_hidden(&toast, false);
            let toast_clone = toast.clone();
            let hide = Closure::<dyn FnMut()>::new(move || set_hidden(&toast_clone, true));
            if let Some(win) = window() {
                let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                    hide.as_ref().unchecked_ref(),
                    3500,
                );
            }
            hide.forget();
        }
    });
    let _ = form.add_event_listener_with_callback("submit", submit.as_ref().unchecked_ref());
    submit.forget();
}

fn init_toc() {
    if page_kind() != "post" {
        return;
    }
    let Some(doc) = document() else { return };
    let Ok(items) = doc.query_selector_all(".article-toc .toc-item") else {
        return;
    };
    if items.length() == 0 {
        return;
    }
    for i in 0..items.length() {
        let Some(item) = items
            .item(i)
            .and_then(|node| node.dyn_into::<Element>().ok())
        else {
            continue;
        };
        let Ok(Some(button)) = item.query_selector("button[data-section-id]") else {
            continue;
        };
        let Some(id) = button.get_attribute("data-section-id") else {
            continue;
        };
        let click = Closure::<dyn FnMut(Event)>::new(move |_| {
            let Some(doc) = document() else { return };
            if let Some(target) = doc.get_element_by_id(&id) {
                target.scroll_into_view_with_bool(true);
                if let Some(win) = window() {
                    let _ = win.history().and_then(|h| {
                        h.replace_state_with_url(&JsValue::NULL, "", Some(&format!("#{id}")))
                    });
                }
            }
        });
        let _ = button.add_event_listener_with_callback("click", click.as_ref().unchecked_ref());
        click.forget();
    }
    let update = Closure::<dyn FnMut(Event)>::new(move |_| {
        let Some(doc) = document() else { return };
        let Ok(items) = doc.query_selector_all(".article-toc .toc-item") else {
            return;
        };
        let mut active = 0;
        for i in 0..items.length() {
            let Some(item) = items
                .item(i)
                .and_then(|node| node.dyn_into::<Element>().ok())
            else {
                continue;
            };
            let Ok(Some(button)) = item.query_selector("button[data-section-id]") else {
                continue;
            };
            let Some(id) = button.get_attribute("data-section-id") else {
                continue;
            };
            if let Some(section) = doc.get_element_by_id(&id)
                && section.get_bounding_client_rect().top() <= 140.0
            {
                active = i;
            }
        }
        for i in 0..items.length() {
            if let Some(item) = items
                .item(i)
                .and_then(|node| node.dyn_into::<Element>().ok())
            {
                let _ = item
                    .class_list()
                    .toggle_with_force("toc-active", i == active);
            }
        }
        if let Ok(Some(progress)) = doc.query_selector(".article-toc .toc-progress") {
            let pct = if items.length() > 1 {
                active as f64 / (items.length() - 1) as f64 * 100.0
            } else {
                0.0
            };
            if let Some(html) = progress.dyn_ref::<HtmlElement>() {
                let _ = html.style().set_property("height", &format!("{pct}%"));
            }
        }
    });
    if let Some(win) = window() {
        let _ = win.add_event_listener_with_callback("scroll", update.as_ref().unchecked_ref());
        let _ = win.add_event_listener_with_callback("resize", update.as_ref().unchecked_ref());
        if let Ok(event) = Event::new("scroll") {
            let _ = win.dispatch_event(&event);
        }
    }
    update.forget();
}

fn load_script(
    src: &str,
    onload: Closure<dyn FnMut(Event)>,
    onerror: Option<Closure<dyn FnMut(Event)>>,
) {
    let Some(doc) = document() else { return };
    let Ok(script) = doc.create_element("script") else {
        return;
    };
    let Ok(script) = script.dyn_into::<HtmlScriptElement>() else {
        return;
    };
    script.set_src(src);
    script.set_async(true);
    script.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget();
    if let Some(error) = onerror {
        script.set_onerror(Some(error.as_ref().unchecked_ref()));
        error.forget();
    }
    if let Some(head) = doc.head() {
        let _ = head.append_child(&script);
    }
}

fn init_mermaid() {
    if page_kind() != "post" {
        return;
    }
    let Some(doc) = document() else { return };
    let Ok(nodes) = doc.query_selector_all("pre > code.language-mermaid") else {
        return;
    };
    if nodes.length() == 0 {
        return;
    }
    for i in 0..nodes.length() {
        let Some(code) = nodes.item(i) else { continue };
        let Some(pre) = code.parent_element() else {
            continue;
        };
        let Ok(container) = doc.create_element("div") else {
            continue;
        };
        container.set_class_name("mermaid");
        container.set_text_content(code.text_content().as_deref());
        let _ = pre.replace_with_with_node_1(&container);
    }
    let onload = Closure::<dyn FnMut(Event)>::new(move |_| {
        let _ = js_sys::eval(
            "if(window.mermaid){window.mermaid.initialize({startOnLoad:false,securityLevel:'loose',theme:'default'});window.mermaid.run({nodes:[...document.querySelectorAll('.mermaid')] }).catch(()=>document.querySelectorAll('.mermaid').forEach(n=>n.classList.add('mermaid-error')));}",
        );
    });
    let onerror = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Some(doc) = document()
            && let Ok(nodes) = doc.query_selector_all(".mermaid")
        {
            for i in 0..nodes.length() {
                if let Some(node) = nodes
                    .item(i)
                    .and_then(|node| node.dyn_into::<Element>().ok())
                {
                    let _ = node.class_list().add_1("mermaid-error");
                }
            }
        }
    });
    load_script(
        "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js",
        onload,
        Some(onerror),
    );
}

fn init_giscus() {
    if page_kind() != "post" {
        return;
    }
    let Some(doc) = document() else { return };
    let Some(body) = doc.body() else { return };
    if body.get_attribute("data-giscus-enabled").as_deref() != Some("true") {
        return;
    }
    let Ok(Some(host)) = doc.query_selector(".giscus-host") else {
        return;
    };
    let Ok(script) = doc.create_element("script") else {
        return;
    };
    let Ok(script) = script.dyn_into::<HtmlScriptElement>() else {
        return;
    };
    script.set_src("https://giscus.app/client.js");
    script.set_async(true);
    script.set_cross_origin(Some("anonymous"));
    for (name, attr, default) in [
        ("repo", "data-giscus-repo", ""),
        ("repo-id", "data-giscus-repo-id", ""),
        ("category", "data-giscus-category", ""),
        ("category-id", "data-giscus-category-id", ""),
        ("mapping", "data-giscus-mapping", "pathname"),
        ("theme", "data-giscus-theme", "light"),
    ] {
        let value = body.get_attribute(attr).unwrap_or_else(|| default.into());
        let _ = script.set_attribute(&format!("data-{name}"), &value);
    }
    for (name, value) in [
        ("data-strict", "0"),
        ("data-reactions-enabled", "1"),
        ("data-input-position", "top"),
        ("data-lang", "zh-CN"),
        ("data-loading", "lazy"),
    ] {
        let _ = script.set_attribute(name, value);
    }
    let error = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Some(doc) = document()
            && let Ok(Some(status)) = doc.query_selector(".comments-status")
        {
            status.set_text_content(Some("评论未加载成功。请检查网络或隐私拦截设置。"));
        }
    });
    script.set_onerror(Some(error.as_ref().unchecked_ref()));
    error.forget();
    let _ = host.append_child(&script);
}

#[derive(Clone)]
struct Dot {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}
fn init_canvas() {
    let Some(win) = window() else { return };
    if win
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .is_some_and(|m| m.matches())
    {
        return;
    }
    let Some(doc) = document() else { return };
    let Some(canvas) = doc
        .get_element_by_id("canvas-nest")
        .and_then(|e| e.dyn_into::<HtmlCanvasElement>().ok())
    else {
        return;
    };
    let Some(context) = canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|v| v.dyn_into::<CanvasRenderingContext2d>().ok())
    else {
        return;
    };
    let width = win
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1280.0);
    let height = win
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(720.0);
    canvas.set_width(width as u32);
    canvas.set_height(height as u32);
    let count = if width < 720.0 { 55 } else { 115 };
    let dots = Rc::new(RefCell::new(
        (0..count)
            .map(|_| Dot {
                x: Math::random() * width,
                y: Math::random() * height,
                // Match the original canvas-nest motion: each particle starts
                // with a full-range velocity in [-1, 1].
                vx: 2.0 * Math::random() - 1.0,
                vy: 2.0 * Math::random() - 1.0,
            })
            .collect::<Vec<_>>(),
    ));
    let mouse = Rc::new(RefCell::new(None::<(f64, f64)>));
    let frame = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
    let frame_clone = frame.clone();
    let canvas_clone = canvas.clone();
    let dots_clone = dots.clone();
    let mouse_for_frame = mouse.clone();
    *frame_clone.borrow_mut() = Some(Closure::<dyn FnMut()>::new(move || {
        let w = canvas_clone.width() as f64;
        let h = canvas_clone.height() as f64;
        context.clear_rect(0.0, 0.0, w, h);
        let mut dots = dots_clone.borrow_mut();
        let cursor = *mouse_for_frame.borrow();
        for dot in &mut *dots {
            dot.x += dot.vx;
            dot.y += dot.vy;
            if dot.x < 0.0 || dot.x > w {
                dot.vx = -dot.vx;
            }
            if dot.y < 0.0 || dot.y > h {
                dot.vy = -dot.vy;
            }

            // Deeper blue dots keep the network legible even when isolated.
            context.set_fill_style_str("#123f9f");
            context.fill_rect(dot.x - 0.6, dot.y - 0.6, 1.6, 1.6);

            if let Some((mouse_x, mouse_y)) = cursor {
                // The legacy implementation attracts particles toward the
                // pointer in its connection ring. The previous Rust port used
                // the opposite sign, which made the field repel instead.
                let dx = dot.x - mouse_x;
                let dy = dot.y - mouse_y;
                let distance_squared = dx * dx + dy * dy;
                let mouse_max = 20_000.0;
                if distance_squared >= mouse_max / 2.0 && distance_squared < mouse_max {
                    dot.x -= 0.03 * dx;
                    dot.y -= 0.03 * dy;
                    let ratio = (mouse_max - distance_squared) / mouse_max;
                    context.begin_path();
                    context.set_line_width((ratio / 2.0 + 0.1).min(0.8));
                    context.set_stroke_style_str(&format!(
                        "rgba(18,63,159,{:0.3})",
                        (ratio * 0.68 + 0.38).min(0.98)
                    ));
                    context.move_to(dot.x, dot.y);
                    context.line_to(mouse_x, mouse_y);
                    context.stroke();
                }
            }
        }

        // Draw each particle connection once, matching the original 6,000px
        // max-distance network while using a darker, more legible blue.
        for i in 0..dots.len() {
            for j in i + 1..dots.len() {
                let dx = dots[i].x - dots[j].x;
                let dy = dots[i].y - dots[j].y;
                let distance_squared = dx * dx + dy * dy;
                if distance_squared < 6_000.0 {
                    let ratio = (6_000.0 - distance_squared) / 6_000.0;
                    context.begin_path();
                    context.set_line_width((ratio / 2.0 + 0.1).min(0.8));
                    context.set_stroke_style_str(&format!(
                        "rgba(18,63,159,{:0.3})",
                        (ratio * 0.68 + 0.38).min(0.98)
                    ));
                    context.move_to(dots[i].x, dots[i].y);
                    context.line_to(dots[j].x, dots[j].y);
                    context.stroke();
                }
            }
        }
        if let Some(win) = window()
            && let Some(callback) = frame.borrow().as_ref()
        {
            let _ = win.request_animation_frame(callback.as_ref().unchecked_ref());
        }
    }));
    if let Some(callback) = frame_clone.borrow().as_ref() {
        let _ = win.request_animation_frame(callback.as_ref().unchecked_ref());
    }
    let mouse_for_move = mouse.clone();
    let mousemove = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
        *mouse_for_move.borrow_mut() = Some((event.client_x() as f64, event.client_y() as f64));
    });
    let _ = win.add_event_listener_with_callback("mousemove", mousemove.as_ref().unchecked_ref());
    mousemove.forget();

    let mouse_for_leave = mouse.clone();
    let mouseleave = Closure::<dyn FnMut(Event)>::new(move |_| {
        *mouse_for_leave.borrow_mut() = None;
    });
    let _ = win.add_event_listener_with_callback("mouseout", mouseleave.as_ref().unchecked_ref());
    mouseleave.forget();

    let resize_canvas = canvas.clone();
    let resize = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Some(win) = window() {
            resize_canvas.set_width(
                win.inner_width()
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1280.0) as u32,
            );
            resize_canvas.set_height(
                win.inner_height()
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(720.0) as u32,
            );
        }
    });
    let _ = win.add_event_listener_with_callback("resize", resize.as_ref().unchecked_ref());
    resize.forget();
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    init_resume_link();
    init_back_to_top();
    init_search();
    init_contact();
    init_toc();
    init_mermaid();
    init_giscus();
    init_canvas();
}
