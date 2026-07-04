use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use std::time::Duration;
use sysinfo::{ProcessesToUpdate, System};
use tray_icon::{Icon, TrayIconBuilder};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

const PROCESS_NAME: &str = "sing-box";

// Встраиваем иконки прямо в бинарник во время компиляции.
// Пути ведут в папку assets, которая лежит рядом с Cargo.toml
const ICON_ACTIVE_BYTES: &[u8] = include_bytes!("../assets/icon_green.png");
const ICON_INACTIVE_BYTES: &[u8] = include_bytes!("../assets/icon_gray.png");

fn load_icon_from_memory(bytes: &[u8]) -> Icon {
    let image = image::load_from_memory(bytes)
        .expect("Не удалось декодировать иконку из памяти")
        .into_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    Icon::from_rgba(rgba, width, height).unwrap()
}

fn is_process_running(sys: &mut System) -> bool {
    // Передаем `true` вторым аргументом (clean_zombies)
    // Это заставит sysinfo удалить завершенные процессы из своего кэша
    sys.refresh_processes(ProcessesToUpdate::All, true);
    
    sys.processes()
        .values()
        .any(|val| {
            let name_matches = val.name().to_string_lossy().contains(PROCESS_NAME);
            // Дополнительно проверяем, что процесс не находится в состоянии Zombie/Dead,
            // если библиотека все еще удерживает его в списке
            #[cfg(not(target_os = "windows"))]
            {
                use sysinfo::ProcessStatus;
                name_matches && val.status() != ProcessStatus::Dead && val.status() != ProcessStatus::Zombie
            }
            #[cfg(target_os = "windows")]
            name_matches
        })
}

// Структура для управления состоянием приложения в новом цикле winit (run_app)
struct TrayApp {
    tray_icon: Option<tray_icon::TrayIcon>,
    start_item: MenuItem,
    stop_item: MenuItem,
    quit_item: MenuItem,
    icon_on: Icon,
    icon_off: Icon,
    sys: System,
    last_status: bool,
}

impl ApplicationHandler for TrayApp {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Инициализация графики при старте. Для трей-приложения здесь пусто,
        // так как трей мы создали заранее в main.
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _window_id: WindowId, _event: WindowEvent) {
        // Окон у нас нет, игнорируем
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Этот метод вызывается на каждой итерации цикла. Проверяем процесс:
        let current_status = is_process_running(&mut self.sys);

        // Меняем иконку только при смене статуса
        if current_status != self.last_status {
            self.last_status = current_status;
            if let Some(ref mut tray) = self.tray_icon {
                if current_status {
                    tray.set_icon(Some(self.icon_on.clone())).unwrap();
                } else {
                    tray.set_icon(Some(self.icon_off.clone())).unwrap();
                }
            }
        }

        // Обрабатываем нажатия на пункты меню
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.start_item.id() {
                println!("Нажата кнопка Start. Запуск sing-box через беспарольный sudo...");

                let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/Users/shared".to_string());
                let config_path = format!("{}/.config/sing-box/config.json", home_dir);

                // Запускаем напрямую через sudo. Благодаря правилу в visudo, 
                // система выполнит это мгновенно и без всплывающих окон!
                let run_result = std::process::Command::new("sudo")
                    .arg("/usr/local/bin/sing-box")
                    .arg("-c")
                    .arg(&config_path)
                    .arg("run")
                    .stdout(std::process::Stdio::null()) // Прячем логи бинарника
                    .stderr(std::process::Stdio::null())
                    .spawn();

                match run_result {
                    Ok(_) => println!("sing-box запущен в фоне!"),
                    Err(e) => {
                        eprintln!("Ошибка запуска. Проверьте путь или правила sudoers: {}", e);
                    }
                }

            } else if event.id == self.stop_item.id() {
                println!("Нажата кнопка Stop. Остановка процесса...");
                
                // Вызываем killall через sudo, чтобы завершить root-процесс
                let _ = std::process::Command::new("sudo")
                    .arg("killall")
                    .arg(PROCESS_NAME)
                    .output();
                
                println!("Команда остановки выполнена.");
                
            } else if event.id == self.quit_item.id() {
                println!("Выход из приложения. Гасим sing-box...");
                
                let _ = std::process::Command::new("sudo")
                    .arg("killall")
                    .arg(PROCESS_NAME)
                    .output();
                
                self.tray_icon.take();
                event_loop.exit();
            }
        }

        // Говорим winit подождать 2 секунды перед следующей проверкой
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            std::time::Instant::now() + Duration::from_secs(2),
        ));
    }
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
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Sing-box Monitor")
        .with_icon(icon_off.clone())
        .build()
        .unwrap();

    let mut app = TrayApp {
        tray_icon: Some(tray_icon),
        start_item,
        stop_item,
        quit_item,
        icon_on,
        icon_off,
        sys: System::new(),
        last_status: false,
    };

    // Настраиваем начальный таймер и запускаем приложение через run_app
    event_loop.set_control_flow(ControlFlow::WaitUntil(
        std::time::Instant::now() + Duration::from_secs(2),
    ));
    
    event_loop.run_app(&mut app).unwrap();
}

