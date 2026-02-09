# MacPurge — Техническое задание

## Обзор

Приложение для macOS для умной очистки системы, заточенное под разработчиков. Три компонента: лендинг (маркетинг), CLI-утилита (ядро), десктопное приложение (GUI). Не общий анализатор диска — только целевая проверка по конкретным категориям: dev-артефакты в проектах, пакетные менеджеры, dev-инструменты (Xcode, Android Studio, Docker, VM), установленные приложения, системный мусор. Ключевое отличие — **интеллектуальные рекомендации** с готовыми скриптами очистки и очистка по расписанию.

---

## Структура монорепо

```
/
├── landing/          # Маркетинговый сайт (Next.js)
├── cli/              # CLI-утилита (Rust)
├── app/              # Десктопное приложение (Tauri + Next.js)
│   ├── src-tauri/    # Rust-бэкенд Tauri
│   └── src/          # Next.js фронтенд
└── shared/           # Общие скрипты очистки, рецепты
```

---

## 1. CLI-утилита (`cli/`)

**Язык:** Rust

**Основные команды:**

### 1.1 Анализ dev-артефактов в проектах

- Пользователь указывает одну или несколько корневых папок с проектами (например `~/projects`, `~/work`) в конфиге `~/.macpurge/config.toml`
- `scan-dev` — сканирование всех указанных папок рекурсивно
- Находит и группирует по типу:
  - **Node.js:** `node_modules/`, `.next/`, `.nuxt/`, `dist/`, `.cache/`, `.parcel-cache/`
  - **Python:** `.venv/`, `venv/`, `__pycache__/`, `.mypy_cache/`, `.pytest_cache/`, `*.egg-info/`, `.tox/`
  - **Rust:** `target/`
  - **Go:** кеши в `~/go/pkg/mod/cache/`
  - **Java/Kotlin:** `.gradle/`, `build/`, `.m2/repository/` (глобально)
  - **C/C++:** `build/`, `cmake-build-*/`
  - **Swift/Xcode:** `DerivedData/`, `~/Library/Developer/Xcode/DerivedData/`
  - **Ruby:** `vendor/bundle/`
  - **PHP:** `vendor/` (с проверкой composer.lock)
  - **Dart/Flutter:** `.dart_tool/`, `build/`
- Для каждого найденного артефакта: путь, размер, дата последнего изменения, безопасно ли удалять
- Команда `clean-dev <path> --type node_modules,target --dry-run` для предпросмотра удаления
- Команда `clean-dev <path> --type all --older-than 30d` — удалить артефакты проектов, не менявшихся 30+ дней

### 1.2 Глобальные пакетные менеджеры

- `scan-packages` — анализ глобально установленных пакетов:
  - **npm:** `npm ls -g --depth=0`, размер `~/.npm/`, `~/node_modules/`
  - **yarn:** глобальный кеш (`yarn cache dir`)
  - **pnpm:** глобальный store (`pnpm store path`)
  - **pip:** `pip list`, `pip cache dir`
  - **cargo:** `~/.cargo/registry/`, `~/.cargo/git/`
  - **rustup:** установленные тулчейны (`~/.rustup/toolchains/`)
  - **go:** `~/go/pkg/mod/`
  - **gem:** `gem list`, `gem environment gemdir`
  - **brew:** `brew list`, `brew cleanup --dry-run`, `~/Library/Caches/Homebrew/`
  - **composer:** глобальный кеш
- Для каждого: имя, версия, размер, когда последний раз использовался (если можно определить)
- `clean-packages --type npm --unused` — удалить неиспользуемые
- `clean-packages --type cargo --cache-only` — очистить только кеши, не сами пакеты

### 1.3 Анализ установленных приложений

- `scan-apps` — полный аудит установленных программ:
  - Источники обнаружения: `/Applications/`, `~/Applications/`, `brew list --cask`, `mas list` (Mac App Store)
  - Для каждого приложения находит **все** связанные файлы:
    - `~/Library/Application Support/<app>/`
    - `~/Library/Caches/<app>/`
    - `~/Library/Preferences/<app>.plist`
    - `~/Library/Logs/<app>/`
    - `~/Library/Containers/<app>/`
    - `~/Library/Group Containers/<app>/`
    - `~/Library/Saved Application State/<app>.savedState/`
    - `~/Library/WebKit/<app>/`
    - `/Library/Application Support/<app>/`
    - `/Library/Caches/<app>/`
    - `/Library/LaunchAgents/`, `/Library/LaunchDaemons/` (связанные plist)
    - `~/Library/LaunchAgents/` (связанные plist)
    - Login Items
  - Суммарный размер приложения со всеми данными
  - Определение Bundle ID для точного сопоставления
- `uninstall <app-name> --full` — полное удаление приложения со всеми следами
- `uninstall <app-name> --dry-run` — показать, что будет удалено

### 1.4 Dev-окружения и инструменты

- `scan-devtools` — анализ тяжёлых dev-инструментов:

**Xcode:**
  - `~/Library/Developer/Xcode/DerivedData/` — кеш сборок
  - `~/Library/Developer/Xcode/Archives/` — старые архивы
  - `~/Library/Developer/Xcode/iOS DeviceSupport/` — символы для устройств
  - `~/Library/Developer/Xcode/watchOS DeviceSupport/`
  - `~/Library/Developer/CoreSimulator/Devices/` — iOS/watchOS/tvOS симуляторы
  - `~/Library/Developer/CoreSimulator/Caches/`
  - `~/Library/Caches/com.apple.dt.Xcode/`
  - Список симуляторов: `xcrun simctl list` — удаление неиспользуемых рантаймов и устройств
  - Старые Xcode Command Line Tools

**Android Studio / SDK:**
  - `~/Library/Android/sdk/` — SDK платформы, build-tools, system-images
  - `~/.android/avd/` — эмуляторы Android (часто 5-15 ГБ каждый)
  - `~/.gradle/caches/` — Gradle кеш
  - `~/.gradle/wrapper/dists/` — скачанные дистрибутивы Gradle
  - `~/Library/Application Support/Google/AndroidStudio*/`
  - Список эмуляторов: `emulator -list-avds` — удаление неиспользуемых
  - Старые SDK платформы и build-tools (оставлять только используемые в проектах)

**Docker:**
  - `docker system df` — общий анализ
  - Остановленные контейнеры (`docker ps -a --filter status=exited`)
  - Dangling images (`docker images -f dangling=true`)
  - Неиспользуемые volumes (`docker volume ls -f dangling=true`)
  - Build cache (`docker builder prune`)
  - `~/Library/Containers/com.docker.docker/Data/vms/` — Docker Desktop VM disk
  - Команды очистки: `docker system prune`, `docker builder prune`, точечное удаление

**Виртуальные машины:**
  - Parallels: `~/Parallels/` — образы VM
  - UTM: `~/Library/Containers/com.utmapp.UTM/Data/Documents/`
  - VMware Fusion: `~/Virtual Machines.localized/`
  - VirtualBox: `~/VirtualBox VMs/`
  - Для каждой VM: имя, размер диска, дата последнего запуска, состояние (running/stopped)
  - Снапшоты VM (часто забытые и огромные)

### 1.5 Системная очистка

- `scan-system` — анализ системного мусора:
  - Кеши браузеров (Chrome, Safari, Firefox, Arc, Brave)
  - Системные логи (`/var/log/`, `~/Library/Logs/`)
  - Старые iOS бэкапы (`~/Library/Application Support/MobileSync/Backup/`)
  - Time Machine локальные снапшоты
  - Корзина (`~/.Trash/`)
  - Downloads (файлы старше N дней)
  - Mail attachments и кеши

### 1.6 Очистка по расписанию

- `schedule` — настройка автоматической очистки:
  - Конфиг в `~/.macpurge/config.toml`:
    ```toml
    [schedule]
    enabled = true
    cron = "0 3 * * 0"  # каждое воскресенье в 3:00

    [schedule.tasks]
    clean_dev = { types = ["node_modules", "target", "__pycache__"], older_than = "30d" }
    clean_packages = { types = ["npm_cache", "cargo_cache", "pip_cache"] }
    clean_docker = { dangling_images = true, stopped_containers = true, build_cache = true }
    clean_system = { browser_caches = true, logs_older_than = "14d", trash = true }
    ```
  - Реализация через macOS LaunchAgent (plist)
  - `schedule enable` / `schedule disable` — вкл/выкл
  - `schedule run-now` — запустить немедленно
  - `schedule status` — показать когда последний/следующий запуск
  - Лог выполнения в `~/.macpurge/schedule.log`
  - Уведомления через macOS Notification Center: «Очищено X ГБ»
  - В GUI: визуальный конфигуратор расписания с чекбоксами по категориям

### 1.7 Интеграция с Claude Code

- `analyze --ai` — отправить результаты сканирования в Claude для получения персонализированных рекомендаций
- Claude анализирует: какие проекты давно не трогались, какие кеши безопасно удалить, какие приложения дублируют функционал
- Вывод: приоритизированный список рекомендаций с обоснованием и командами для выполнения

---

## 2. Десктопное приложение (`app/`)

**Стек:** Tauri 2 (Rust-бэкенд) + Next.js (React-фронтенд)

### 2.0 Архитектура Tauri + Next.js

**Почему Tauri:**
- Общая кодовая база бэкенда с CLI (Rust) — переиспользование логики сканирования и очистки
- Минимальный размер бандла (~10-15 МБ vs ~150+ МБ у Electron)
- Нативный WebView (WKWebView на macOS) — низкое потребление памяти
- Прямой доступ к системным API macOS через Rust (без FFI-прослоек)
- Встроенная поддержка auto-update, system tray, notifications

**Структура проекта:**
```
app/
├── src-tauri/
│   ├── Cargo.toml          # Зависимости: tauri, serde, shared-crate
│   ├── tauri.conf.json      # Конфигурация Tauri (permissions, window, bundling)
│   ├── capabilities/        # Tauri v2 capability-based permissions
│   │   └── main.json
│   ├── src/
│   │   ├── main.rs          # Точка входа Tauri
│   │   ├── commands/        # Tauri commands (IPC-хэндлеры)
│   │   │   ├── scan.rs      # scan_dev, scan_packages, scan_apps, scan_devtools, scan_system
│   │   │   ├── clean.rs     # clean_dev, clean_packages, uninstall_app
│   │   │   ├── schedule.rs  # schedule_enable, schedule_disable, schedule_status
│   │   │   └── ai.rs        # analyze_with_ai
│   │   └── lib.rs
│   └── icons/               # Иконки приложения
├── src/                      # Next.js фронтенд
│   ├── app/                  # App Router
│   │   ├── layout.tsx
│   │   ├── page.tsx          # Dashboard
│   │   ├── dev/page.tsx      # Dev-панель
│   │   ├── apps/page.tsx     # Панель приложений
│   │   ├── devtools/page.tsx # Панель dev-инструментов
│   │   ├── ai/page.tsx       # AI-рекомендации
│   │   ├── recipes/page.tsx  # Готовые рецепты
│   │   ├── schedule/page.tsx # Расписание
│   │   └── settings/page.tsx # Настройки
│   ├── components/           # React-компоненты
│   │   ├── ui/               # Базовые UI-компоненты (shadcn/ui)
│   │   ├── treemap/          # Treemap-визуализация (D3.js)
│   │   ├── charts/           # Графики (Recharts)
│   │   └── layout/           # Sidebar, Header, Navigation
│   ├── lib/
│   │   ├── tauri.ts          # Обёртки над invoke() для Tauri commands
│   │   ├── types.ts          # TypeScript типы (зеркало Rust-структур)
│   │   └── utils.ts
│   └── hooks/                # React-хуки для работы с данными
├── next.config.js            # output: 'export' для статической генерации
├── package.json
└── tailwind.config.ts
```

**Взаимодействие фронтенд ↔ бэкенд:**
- Все операции через Tauri IPC (`invoke()`) — фронтенд вызывает Rust-команды
- Rust-бэкенд переиспользует core-логику из CLI (общий crate `shared/`)
- Длительные операции (сканирование) используют Tauri Events для стриминга прогресса
- Типы данных синхронизированы: Rust `serde::Serialize` → TypeScript interfaces

```typescript
// src/lib/tauri.ts — пример обёртки
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export async function scanDev(paths: string[]) {
  return invoke<ScanResult>('scan_dev', { paths });
}

export function onScanProgress(callback: (progress: ScanProgress) => void) {
  return listen<ScanProgress>('scan-progress', (event) => callback(event.payload));
}
```

```rust
// src-tauri/src/commands/scan.rs — пример Tauri command
#[tauri::command]
async fn scan_dev(paths: Vec<String>, app: tauri::AppHandle) -> Result<ScanResult, String> {
    let scanner = shared::DevScanner::new(paths);
    scanner.scan_with_progress(|progress| {
        app.emit("scan-progress", &progress).ok();
    }).map_err(|e| e.to_string())
}
```

**Next.js конфигурация для Tauri:**
- `output: 'export'` в `next.config.js` — статическая генерация (Tauri загружает HTML/CSS/JS локально)
- Никаких серверных API-роутов — вся логика в Rust через IPC
- `next dev` для разработки с hot reload, Tauri dev-сервер проксирует на localhost

**Tauri Permissions (v2):**
- `fs:read` / `fs:write` — доступ к файловой системе
- `shell:execute` — запуск CLI-команд (docker, xcrun, brew и т.д.)
- `notification:default` — macOS уведомления
- `os:default` — информация о системе

### 2.1 Визуализация

- Treemap по категориям: dev-артефакты, приложения, dev-инструменты (Xcode/Android/Docker/VM), системный мусор, пакетные менеджеры
- Реализация: D3.js treemap layout + React-компоненты для рендеринга
- Drill-down внутри каждой категории
- Цветовая кодировка по типу контента
- Анимированные переходы между уровнями (Framer Motion)
- **Не** полный анализ диска (не -клон) — только то, что умеем чистить

### 2.2 Dashboard

- Общая сводка: занято / свободно, потенциально можно освободить
- Категории: Dev-артефакты, Приложения, Dev-инструменты (Xcode/Android/Docker/VM), Пакетные менеджеры, Системное
- Для каждой категории: размер, количество элементов, кнопка «Детали»

### 2.3 Dev-панель

- Список проектов с артефактами, сгруппированный по языку/фреймворку
- Для каждого проекта: путь, размер артефактов, последний коммит (если git), safe to clean
- Массовое выделение и очистка
- Фильтры: по языку, по размеру, по дате последнего изменения

### 2.4 Панель приложений

- Список всех установленных приложений с полным размером (app + данные + кеши)
- Для каждого: иконка, имя, версия, размер .app, размер данных, размер кешей
- Кнопка «Полное удаление» — удаляет из всех мест
- Поиск по имени

### 2.5 Панель Dev-инструментов

- **Xcode:** список симуляторов с размерами, DerivedData, Archives, DeviceSupport — кнопки очистки для каждого
- **Android:** список эмуляторов с размерами, SDK компоненты, Gradle кеш
- **Docker:** визуализация `docker system df` — images, containers, volumes, build cache с кнопками prune
- **VM:** список виртуальных машин (Parallels/UTM/VMware/VirtualBox) с размерами и датой последнего запуска

### 2.6 Панель рекомендаций (AI)

- Рекомендации от Claude: что можно безопасно удалить, с обоснованием
- Группировка по приоритету (высокий / средний / низкий выигрыш)
- Для каждой рекомендации: описание, размер, команда, кнопка «Выполнить»
- Пользователь сам решает, что удалять — галочками выбирает из списка

### 2.7 Готовые рецепты очистки

- Предустановленные скрипты:
  - «Очистить dev-кеши во всех проектах»
  - «Удалить node_modules старше 30 дней»
  - «Очистить Docker»
  - «Очистить Xcode DerivedData + симуляторы»
  - «Очистить Android эмуляторы + Gradle кеш»
  - «Очистить глобальные npm/cargo/pip кеши»
  - «Очистить браузерные кеши»
- Каждый рецепт: описание, dry-run предпросмотр, кнопка выполнения
- Возможность создавать свои рецепты

### 2.8 Расписание очистки

- Визуальный конфигуратор расписания: выбор дня/времени
- Чекбоксы по категориям: что включить в автоочистку
- Настройка порогов (например, удалять node_modules только если проект не менялся 30+ дней)
- Лог предыдущих запусков с результатами
- macOS уведомления после выполнения

### 2.9 Настройки

- Указание корневых папок с проектами (обязательная настройка при первом запуске)
- Конфиг Full Disk Access
- Настройки расписания
- Настройки AI-интеграции (API key)
- Тема оформления (light/dark, следовать системной)

### 2.10 Техническое

- GUI вызывает Rust-бэкенд напрямую через Tauri IPC (не через CLI-бинарник)
- Общий Rust crate (`shared/`) переиспользуется между CLI и Tauri-бэкендом
- Permissions: Full Disk Access (объяснение пользователю зачем)
- Progress bar для длительных операций — через Tauri Events (стриминг прогресса)
- Undo: перемещение в специальную корзину перед удалением (с таймером автоочистки)
- Auto-update: встроенный Tauri updater с GitHub Releases
- System tray: иконка в менюбаре с отображением свободного места в ГБ. Текст подсвечивается красным, если свободно менее 10 ГБ. Быстрый доступ к статусу и quick actions.
- Размер бандла: ~15-20 МБ (WKWebView + Rust)

---

## 3. Лендинг (`landing/`)

**Стек:** Next.js (деплой на Vercel)

### Страницы

- Hero: « для разработчиков» — визуализация + умная очистка
- Фичи: dev-артефакты, полное удаление приложений, AI-рекомендации, готовые рецепты
- Сравнение с конкурентами (MacPurge, OmniDiskSweeper)
- Скриншоты / видео
- Pricing (free CLI + paid GUI)
- Download

### Ключевые сообщения

- «Знает, что такое node_modules и target/»
- «Удаляет приложения полностью, а не только .app»
- «Готовые рецепты для разработчиков»
- «AI подскажет, что безопасно удалить»

---

## Общая Rust-архитектура

```
shared/                      # Общий crate
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── scanner/             # Логика сканирования
    │   ├── dev.rs           # Dev-артефакты
    │   ├── packages.rs      # Пакетные менеджеры
    │   ├── apps.rs          # Установленные приложения
    │   ├── devtools.rs      # Dev-инструменты
    │   └── system.rs        # Системный мусор
    ├── cleaner/             # Логика очистки
    │   ├── dev.rs
    │   ├── packages.rs
    │   ├── apps.rs
    │   └── devtools.rs
    ├── scheduler/           # Расписание (LaunchAgent)
    ├── config.rs            # Работа с config.toml
    └── types.rs             # Общие типы данных

cli/                         # CLI-обёртка
├── Cargo.toml               # depends on shared
└── src/main.rs              # clap commands → shared functions

app/src-tauri/               # Tauri-обёртка
├── Cargo.toml               # depends on shared
└── src/
    ├── main.rs
    └── commands/            # Tauri IPC commands → shared functions
```

Оба бинарника (CLI и Tauri) используют одну и ту же логику из `shared/`, различаясь только интерфейсом: CLI выводит в терминал, Tauri передаёт данные через IPC в Next.js фронтенд.

---

## Приоритеты реализации

1. **Shared crate: core-логика сканирования и очистки**
2. **CLI: scan-dev + clean-dev** — ядро продукта
3. **CLI: scan-devtools (Xcode, Android, Docker, VM)**
4. **CLI: scan-packages + clean-packages**
5. **CLI: scan-apps + uninstall**
6. **CLI: scan-system**
7. **CLI: schedule (LaunchAgent)**
8. **Tauri app: scaffolding + Next.js + базовый Dashboard**
9. **Tauri app: все панели + treemap визуализация (D3.js)**
10. **Tauri app: расписание + настройки + system tray**
11. **CLI: AI-интеграция**
12. **Tauri app: AI-панель + рецепты**
13. **Landing (Next.js на Vercel)**

---

## Нефункциональные требования

- Сканирование `~/projects` с 50 проектами — менее 10 секунд
- Никогда не удалять без явного подтверждения пользователя (кроме scheduled tasks с pre-approved конфигом)
- Dry-run по умолчанию для всех деструктивных операций
- Логирование всех удалений в `~/.macpurge/history.log`
- Размер бандла приложения: не более 25 МБ
- Поддержка macOS 13+ (Ventura и выше)
- Apple Silicon (arm64) и Intel (x86_64) — universal binary через Tauri bundler
