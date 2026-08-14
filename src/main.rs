use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem}; // Добавили CheckMenuItem
use serde::Deserialize;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use sysinfo::{ProcessesToUpdate, System};
use tray_icon::{Icon, TrayIconBuilder};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

const PROCESS_NAME: &str = "sing-box";
// const CLASH_API_URL: &str = "http://127.0.0.1:9090";
const CLASH_SOCKET_ADDR: &str = "127.0.0.1:9090";
const CLASH_SECRET: &str = "YOUR_SECRET_TOKEN";

const ICON_ACTIVE_BYTES: &[u8] = include_bytes!("../assets/icon_green.png");
const ICON_INACTIVE_BYTES: &[u8] = include_bytes!("../assets/icon_gray.png");

#[derive(Deserialize, Debug)]
struct ClashSelector {
    all: Vec<String>,
    now: String,
}

#[derive(serde::Serialize)]
struct ChangeProxyPayload {
    name: String,
}

struct ProxyMenuItem {
    id: muda::MenuId,
    name: String,
}

struct TrayApp {
    tray_icon: Option<tray_icon::TrayIcon>,
    tray_menu: Menu,
    start_item: MenuItem,
    stop_item: MenuItem,
    quit_item: MenuItem,
    icon_on: Icon,
    icon_off: Icon,
    sys: System,
    last_status: bool,
    proxy_items: Vec<ProxyMenuItem>,
}

impl ApplicationHandler for TrayApp {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: WindowEvent,
    ) {
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let current_status = is_process_running(&mut self.sys);

        if current_status != self.last_status {
            self.last_status = current_status;
            if let Some(ref mut tray) = self.tray_icon {
                if current_status {
                    tray.set_icon(Some(self.icon_on.clone())).unwrap();
                    std::thread::sleep(Duration::from_millis(200));
                    self.rebuild_proxy_menu();
                } else {
                    tray.set_icon(Some(self.icon_off.clone())).unwrap();
                    self.clear_proxy_menu();
                }
            }
        }

        // Исправлено: Смена порядка сравнения ID и извлечение CheckMenuItem
        if current_status && !self.proxy_items.is_empty() {
            if let Ok(selector) = fetch_clash_proxies() {
                for item in &self.proxy_items {
                    if let Some(menu_item) = self
                        .tray_menu
                        .items()
                        .into_iter()
                        .find(|i| item.id == i.id())
                    {
                        if let muda::MenuItemKind::Check(m_item) = menu_item {
                            let _ = m_item.set_checked(item.name == selector.now);
                        }
                    }
                }
            }
        }

        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.start_item.id() {
                println!("Нажата кнопка Start. Запуск sing-box...");
                let home_dir =
                    std::env::var("HOME").unwrap_or_else(|_| "/Users/shared".to_string());
                let config_path = format!("{}/.config/sing-box/config.json", home_dir);

                let run_result = std::process::Command::new("sudo")
                    .arg("/usr/local/bin/sing-box")
                    .arg("-c")
                    .arg(&config_path)
                    .arg("run")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();

                match run_result {
                    Ok(_) => {
                        println!("sing-box запущен в фоне! Ожидаем старта API...");

                        // ПРИНУДИТЕЛЬНОЕ ОБНОВЛЕНИЕ ПРИ СТАРТЕ:
                        // Даем ОС 600мс, чтобы sing-box точно запустил свой Clash HTTP-сервер
                        std::thread::sleep(Duration::from_millis(1000));

                        // Синхронизируем статус, чтобы фоновый поток не запутался
                        self.last_status = true;
                        if let Some(ref mut tray) = self.tray_icon {
                            tray.set_icon(Some(self.icon_on.clone())).unwrap();
                        }

                        // Сразу строим меню селекторов!
                        self.rebuild_proxy_menu();
                    }
                    Err(e) => {
                        eprintln!("Ошибка запуска: {}", e);
                    }
                }
            } else if event.id == self.stop_item.id() {
                println!("Нажата кнопка Stop...");
                let _ = std::process::Command::new("sudo")
                    .arg("killall")
                    .arg(PROCESS_NAME)
                    .output();
            } else if event.id == self.quit_item.id() {
                println!("Выход...");
                let _ = std::process::Command::new("sudo")
                    .arg("killall")
                    .arg(PROCESS_NAME)
                    .output();
                self.tray_icon.take();
                event_loop.exit();
            } else {
                if let Some(clicked_proxy) =
                    self.proxy_items.iter().find(|item| item.id == event.id)
                {
                    println!("Переключаем на: {}", clicked_proxy.name);
                    if let Err(e) = switch_clash_proxy(&clicked_proxy.name) {
                        eprintln!("Ошибка Clash API: {}", e);
                    } else {
                        // Исправлено: Смена порядка сравнения ID для ручного обновления галочек
                        if let Ok(selector) = fetch_clash_proxies() {
                            for item in &self.proxy_items {
                                if let Some(menu_item) = self
                                    .tray_menu
                                    .items()
                                    .into_iter()
                                    .find(|i| item.id == i.id())
                                {
                                    if let muda::MenuItemKind::Check(m_item) = menu_item {
                                        let _ = m_item.set_checked(item.name == selector.now);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        event_loop.set_control_flow(ControlFlow::WaitUntil(
            std::time::Instant::now() + Duration::from_secs(2),
        ));
    }
}

impl TrayApp {
    // Исправлено: Удаление элементов по ссылке на сам элемент внутри MenuItemKind
    fn clear_proxy_menu(&mut self) {
        for item in &self.proxy_items {
            if let Some(menu_item) = self
                .tray_menu
                .items()
                .into_iter()
                .find(|i| item.id == i.id())
            {
                // Извлекаем конкретный тип из MenuItemKind перед удалением
                match menu_item {
                    muda::MenuItemKind::MenuItem(i) => {
                        let _ = self.tray_menu.remove(&i);
                    }
                    muda::MenuItemKind::Check(i) => {
                        let _ = self.tray_menu.remove(&i);
                    }
                    muda::MenuItemKind::Icon(i) => {
                        let _ = self.tray_menu.remove(&i);
                    }
                    muda::MenuItemKind::Predefined(i) => {
                        let _ = self.tray_menu.remove(&i);
                    }
                    muda::MenuItemKind::Submenu(i) => {
                        let _ = self.tray_menu.remove(&i);
                    }
                }
            }
        }
        self.proxy_items.clear();
    }

    // Исправлено: Использование CheckMenuItem вместо обычного MenuItem
    fn rebuild_proxy_menu(&mut self) {
        self.clear_proxy_menu();

        if let Ok(selector) = fetch_clash_proxies() {
            let mut new_items = Vec::new();

            let separator = PredefinedMenuItem::separator();
            let _ = self.tray_menu.insert(&separator, 2);

            for (index, proxy_name) in selector.all.iter().enumerate() {
                let is_current = proxy_name == &selector.now;

                // Создаем чекбокс-пункт
                let item = CheckMenuItem::with_id(
                    muda::MenuId::new(format!("proxy_{}", proxy_name)),
                    proxy_name,
                    true,
                    is_current,
                    None,
                );

                let _ = self.tray_menu.insert(&item, 3 + index);

                new_items.push(ProxyMenuItem {
                    id: item.id().clone(),
                    name: proxy_name.clone(),
                });
            }
            self.proxy_items = new_items;
        }
    }
}

/// Делает ручной GET запрос через TCP-сокет и извлекает JSON
fn fetch_clash_proxies() -> Result<ClashSelector, Box<dyn std::error::Error>> {
    // 1. Открываем TCP соединение с таймаутом
    let mut stream = TcpStream::connect_timeout(
        &CLASH_SOCKET_ADDR.parse()?, 
        Duration::from_millis(500)
    )?;
    stream.set_read_timeout(Some(Duration::from_millis(500)))?;

    // 2. Формируем сырой HTTP/1.1 GET запрос вручную. Важно соблюдать перенос строк \r\n!
    let request = format!(
        "GET /proxies/select-out HTTP/1.1\r\n\
         Host: {}\r\n\
         Authorization: Bearer {}\r\n\
         Connection: close\r\n\r\n",
        CLASH_SOCKET_ADDR, CLASH_SECRET
    );

    // 3. Отправляем байты запроса в сокет
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    // 4. Читаем весь ответ из сокета в буфер
    let mut response_bytes = Vec::new();
    stream.read_to_end(&mut response_bytes)?;
    
    // Преобразуем байты в строку для обработки заголовков
    let response_str = String::from_utf8_lossy(&response_bytes);

    // 5. Парсим HTTP-ответ. Нам нужно отделить заголовки от тела JSON.
    // По спецификации HTTP, тело ответа всегда отделяется от заголовков двойным переносом строки \r\n\r\n
    if let Some(body_index) = response_str.find("\r\n\r\n") {
        let body = &response_str[body_index + 4..]; // +4 символа, чтобы пропустить \r\n\r\n
        
        // Парсим чистый JSON строку в нашу структуру
        let selector: ClashSelector = serde_json::from_str(body)?;
        Ok(selector)
    } else {
        Err("Неверный формат HTTP-ответа от sing-box".into())
    }
}

/// Делает ручной PUT запрос с JSON-нагрузкой через TCP-сокет
fn switch_clash_proxy(target_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect_timeout(
        &CLASH_SOCKET_ADDR.parse()?, 
        Duration::from_millis(500)
    )?;
    stream.set_read_timeout(Some(Duration::from_millis(500)))?;

    // Сериализуем структуру payload в JSON строку
    let payload = ChangeProxyPayload { name: target_name.to_string() };
    let json_body = serde_json::to_string(&payload)?;

    // Формируем сырой HTTP/1.1 PUT запрос. 
    // Для POST/PUT запросов обязательно передавать заголовки Content-Type и Content-Length!
    let request = format!(
        "PUT /proxies/select-out HTTP/1.1\r\n\
         Host: {}\r\n\
         Authorization: Bearer {}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n\
         {}",
        CLASH_SOCKET_ADDR, CLASH_SECRET, json_body.len(), json_body
    );

    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    // Для PUT-запроса sing-box возвращает статус 204 No Content и закрывает сокет.
    // Читаем ответ, чтобы гарантировать завершение операции на стороне ОС
    let mut buffer = [0; 128];
    let _ = stream.read(&mut buffer)?;

    Ok(())
}

fn load_icon_from_memory(bytes: &[u8]) -> Icon {
    let image = image::load_from_memory(bytes)
        .expect("Не удалось декодировать иконку")
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).unwrap()
}

fn is_process_running(sys: &mut System) -> bool {
    sys.refresh_processes(ProcessesToUpdate::All, true);
    sys.processes().values().any(|val| {
        let name_matches = val.name().to_string_lossy().contains(PROCESS_NAME);
        #[cfg(not(target_os = "windows"))]
        {
            use sysinfo::ProcessStatus;
            name_matches
                && val.status() != ProcessStatus::Dead
                && val.status() != ProcessStatus::Zombie
        }
        #[cfg(target_os = "windows")]
        name_matches
    })
}

fn main() {
    #[cfg(target_os = "macos")]
    {
        // Импортируем MainThreadMarker напрямую из корня objc2
        use objc2::MainThreadMarker;
        use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

        // 1. Получаем маркер главного потока
        if let Some(mtm) = MainThreadMarker::new() {
            // 2. Запрашиваем экземпляр приложения, передавая маркер
            let app = NSApplication::sharedApplication(mtm);

            // 3. Устанавливаем политику Accessory
            let _ = app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        }
    }

    let event_loop = EventLoop::new().unwrap();

    // Создаем элементы меню
    let tray_menu = Menu::new();
    let start_item = MenuItem::new("Start", true, None);
    let stop_item = MenuItem::new("Stop", true, None);
    let quit_item = MenuItem::new("Quit", true, None);

    tray_menu
        .append_items(&[
            &start_item,
            &stop_item,
            &PredefinedMenuItem::separator(),
            &quit_item,
        ])
        .unwrap();

    // Загружаем иконки из памяти
    let icon_on = load_icon_from_memory(ICON_ACTIVE_BYTES);
    let icon_off = load_icon_from_memory(ICON_INACTIVE_BYTES);

    // Создаем иконку в системном менюбаре
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu.clone()))
        .with_tooltip("Sing-box Monitor")
        .with_icon(icon_off.clone())
        .build()
        .unwrap();

    let mut app = TrayApp {
        tray_icon: Some(tray_icon),
        tray_menu, // Теперь владение передается успешно!
        start_item,
        stop_item,
        quit_item,
        icon_on,
        icon_off,
        sys: System::new(),
        last_status: false,
        proxy_items: Vec::new(),
    };

    // Настраиваем начальный таймер и запускаем приложение через run_app
    event_loop.set_control_flow(ControlFlow::WaitUntil(
        std::time::Instant::now() + Duration::from_secs(2),
    ));

    event_loop.run_app(&mut app).unwrap();
}
