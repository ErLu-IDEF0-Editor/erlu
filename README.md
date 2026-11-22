# erlu — IDEF0 на Rust/egui

Один редактор для Linux, Windows, macOS и браузера. Модель, геометрия, проверки,
история и экспорт общие; desktop предоставляет MCP, web — коннектор внешнего агента.
Приложение работает с новым версионированным форматом `.idef0` v1.0. Старый Qt JSON
и бинарные Qt-плагины не поддерживаются.

## Быстрый запуск

Нужны Rust 1.99, Node.js 22+, Python 3, `wasm-tools` и Trunk 0.21.14.
Для web и компонента цветов нужен target `wasm32-unknown-unknown`.

```sh
# NixOS: все инструменты и runtime-библиотеки
nix-shell
# Trunk и wasm-tools уже предоставлены окружением; cargo install не нужен.
```

На обычной Rust-установке вне Nix:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-tools --version 1.263.0 --locked
cargo install trunk --version 0.21.14 --locked
```

Далее в выбранном окружении:

```sh
# Общий WASM-плагин
npm --prefix web ci
npm --prefix web run plugins

# Desktop
cargo run -p erlu
cargo run -p erlu -- model.idef0

# Web: http://127.0.0.1:8080
trunk serve
```

Если Trunk отклоняет значение `NO_COLOR=1`, задайте `NO_COLOR=true`.
В Nix-среде это уже настроено. Nix-shell использует отдельный каталог
`target/nix-<компилятор>`, чтобы не смешивать артефакты с rustup и другими
версиями Rust. Плагин использует тот же `CARGO_TARGET_DIR`.
Если после переключения окружения возникла ошибка E0514, выйдите из старого
shell и снова выполните `nix-shell`; удалять весь `target` не требуется.

На Linux вне Nix нужны библиотеки X11/Wayland,
libxkbcommon и OpenGL. Зафиксированные версии Rust и crates находятся в
`rust-toolchain.toml` и `Cargo.lock`; npm — в `web/package-lock.json`.

## Работа с редактором

Правый клик добавляет блок; перетаскивание от выходной стороны блока или границы
создаёт стрелку. Ctrl+drag на стрелке создаёт ветвление. Двойной клик редактирует
блок, подпись или штамп. Средняя кнопка на блоке открывает декомпозицию, на фоне
поднимает на уровень выше. Клавиатура: Ctrl+Down/Up, Ctrl+Z/Shift+Z, Delete, F.
На macOS стандартные команды используют Cmd. Pinch служит навигации по декомпозиции.

В браузере файлы открываются через picker и сохраняются скачиванием. IndexedDB
хранит восстановительную копию; это не заменяет сохранение файла. После первого
запуска web-дистрибутив доступен офлайн. Новая версия service worker ожидает закрытия
старых вкладок, не перезагружая открытый документ.

## AI и плагины

```sh
# Основной вариант подключения локального MCP-клиента
cargo run -p erlu -- mcp --headless

# MCP управляет документом открытого окна
cargo run -p erlu -- --mcp
```

В режиме GUI endpoint и токен доступны в меню AI. В headless stdout содержит только
MCP; логирование идёт в stderr. Изменения через AI не сохраняются на диск автоматически.

Web: включите доступ в меню AI и укажите точный origin внешнего агента.
SDK — `web/sdk/erlu-agent.js` и `.d.ts`; пример — `/sdk/example.html` в web-сборке.
При выключении флажка активные подключения отзываются.

Плагин цветов — настоящий WASM-компонент с одним WIT-контрактом для Wasmtime
и браузерного Worker. Новые плагины регистрируют действия, проверки, экспортёры
и формы; любые изменения проходят транзакционный движок.

Документация: [архитектура](docs/architecture.md), [формат](docs/format.md),
[AI API](docs/ai.md), [SDK плагинов](docs/plugins.md), [UX-эталон](docs/ux/baseline.md),
[приёмка](docs/acceptance.md).

## Проверка и поставка

```sh
cargo test --workspace --locked
python3 scripts/test_mcp.py "${CARGO_TARGET_DIR:-target}/debug/erlu"
trunk build --release --locked
npm --prefix web test
nix-build                  # Nix-пакет, включая компонент цветов
```

Для локального Chromium в Nix можно задать `CHROMIUM_PATH`; без него Playwright
использует свои Chromium, Firefox и WebKit (`npx playwright install --with-deps`).

`dist/web` размещается как статический сайт по HTTPS; аккаунт и серверная база
документов не требуются. Не публикуйте каталог исходников или сборочные кэши.
Native-архивы создаются `scripts/package.py --target <triple>` после release-сборки;
`--appimage` использует установленный `linuxdeploy` с AppImage output plugin.
macOS-пакет — `.app` внутри ZIP, Windows — portable ZIP. Подпись не выполняется.

Исходная Qt-реализация доступна в истории Git (ревизия `58198c6`). Сценарии и
снимки UX сохранены в `docs/ux`. Новые нотации и функции IDEF0 добавляются
отдельным этапом; BPMN не входит в этот перенос.

## Релизные артефакты

GitHub Actions workflow `.github/workflows/release.yml` запускается по тегам `v*`
или вручную. Он использует общий CI с тестами, собирает четыре desktop-архива
(Linux, Windows, macOS ARM64/x86_64), `erlu-web.zip` для статического хостинга
и `SHA256SUMS`. Результат доступен как artifact `erlu-release-bundles`.
Публикация GitHub Release и отправка crates на crates.io не выполняются.

Эти workflow рассчитаны на GitHub Actions. Для текущего Forgejo origin нужны
отдельная настройка runner и адаптация workflow; GitHub-зеркало может использовать
их непосредственно. Удалённые workflow в рамках переноса не запускались.
