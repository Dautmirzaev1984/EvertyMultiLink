# Everty MultiLink

<p align="center">
  <strong>Современный нативный Windows-клиент умной VPN-маршрутизации на Rust с нулевым оверхедом и глубоким сетевым анализом.</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Language-Rust_2021-DEA584?style=flat-square&logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/Platform-Windows_10%2F11-0078D6?style=flat-square&logo=windows" alt="Windows" />
  <img src="https://img.shields.io/badge/UI-EvertyUI_(Zero_WebView)-green?style=flat-square" alt="Zero WebView" />
  <img src="https://img.shields.io/badge/Core-Mihomo-blue?style=flat-square" alt="Mihomo" />
  <img src="https://img.shields.io/badge/License-MIT-yellow?style=flat-square" alt="License" />
</p>

---

## О проекте

**Everty MultiLink** — высокопроизводительный нативный клиент для Windows, реализующий интеллектуальную раздельную маршрутизацию (Split Tunneling), управление VLESS/Reality-узлами и гибкую балансировку соединений. 

Приложение написано на **Rust** с использованием собственной сверхлёгкой графической библиотеки **EvertyUI** (программный рендеринг без Electron, Chromium или WebView2), что обеспечивает мгновенный отклик интерфейса, минимальное потребление RAM (< 25 МБ) и надёжное управление системным прокси через Windows WinINet и DPAPI.

- **Автор проекта:** [Артур Валиев (Arthur Valiev)](https://desk.everty.ru/opensource)
- **Лицензия:** [MIT](LICENSE)
- **Официальный портал:** [https://desk.everty.ru/opensource](https://desk.everty.ru/opensource)

---

## Ключевые возможности

- ⚡ **Нулевой оверхед (Zero WebView):** Нативный легковесный рендеринг на Rust, адаптивный тёмный интерфейс, плавная прокрутка, мультиязычность (RU, EN, DE, ES, ZH) с автоопределением языка системы.
- 🛡️ **Безопасное хранилище Windows DPAPI:** Подписки, узлы и правила защищены мастер-ключом пользователя Windows. Автоматическое резервирование конфигурации.
- 🎯 **Умная раздельная маршрутизация (Split Tunneling):**
  - Поддержка списков правил `DOMAIN-SUFFIX`, `IP-CIDR`, `PROCESS-NAME`.
  - Встроенный предустановленный набор правил для российских сервисов (`RU сайты`, `.ru`, `.рф`).
  - Привязка отдельных доменов или процессов к конкретным серверам, страновым пулам или прямому соединению (DIRECT).
- 🔄 **Политики выбора серверов:**
  - `Smart (Умный выбор)`: адаптивный `url-test` с независимыми подгруппами для ключевых доменов (`MULTILINK-SITE-X`).
  - `Balance`: балансировка нагрузки.
  - `Latency`: агрессивный выбор минимального RTT.
  - `Stable`: резервирование с отказоустойчивым переключением (Fallback).
- 🧩 **Универсальный импорт:** VLESS TCP / TLS / REALITY / WebSocket / gRPC, Base64, Mihomo YAML, HTTPS-подписки с автообновлением каждые 6 часов.
- 🪟 **Интеграция с Windows:** Режимы System Proxy (без прав администратора) и TUN-адаптер (с повышением привилегий), управление из системного трея, автозапуск.

---

## Архитектура проекта

```text
Everty MultiLink/
├── crates/
│   ├── multilink-app         # Главное приложение, оконная система EvertyUI, трей, обработка ввода
│   ├── multilink-core        # Доменные модели, валидация, дедупликация, парсинг подписок
│   ├── multilink-mihomo      # Генерация конфигураций, управление ядром Mihomo, REST API, ZERO Shadow
│   └── multilink-platform    # Системные вызовы Windows: WinINet System Proxy, DPAPI, Watchdog, TUN
├── docs/                     # Техническая документация и форматы доступа
└── vendor/mihomo/            # Исполняемый файл сетевого ядра (mihomo.exe)
```

---

## Исследовательский контур: Network Bottleneck Analysis & ZERO

В рамках проекта проведено глубокое эмпирическое исследование сетевых задержек и алгоритмов адаптивного мониторинга узлов:

### 1. Декомпозиция задержки (Cold vs Warm Connection)
*На основе 100 верифицированных парных замеров на физических 10 Gbit узлах:*
- **Холодный запрос (Cold Total):** ~420 мс
- **Повторный прогретый запрос (Warm Keep-Alive):** ~80 мс
- **Чистый оверхед туннелирования:** полезная нагрузка через VPN добавляет **всего 6–9 мс** по сравнению с прямым подключением без прокси.
- **80%+ времени холодного старта (~340 мс)** уходит на каскадное установление соединений (DNS + VLESS Handshake + Remote TCP + End-to-End TLS). При этом современные Chromium-браузеры полностью нивелируют эту задержку за счёт мультиплексирования HTTP/2-потоков на один открытый туннель.

### 2. Multi-Path Egress & Straggler Effect
Сравнение одиночного быстрейшего узла (`Single Best`) с распределением запросов (`Round-Robin` и `Consistent-Hashing`):
- Для интерактивного веб-серфинга одиночный быстрый узел превосходит Multi-Path на **~13%** по скорости загрузки страниц.
- Распределение компонентов страницы по разнородным узлам вызывает **эффект отстающего (straggler effect)**, когда общее время рендеринга страницы тормозится самым медленным из выбранных серверов.

### 3. ZERO Probe Scheduling: Парето-граница
Исследование обучаемого планировщика проверок ZERO на марковской модели стохастических сбоев при ограниченном бюджете:
- **Ультра-низкий бюджет ($\le 15$ проверок на 1000 тактов):** алгоритм ZERO с формулой *Value of Information* и экспоненциальным бэкоффом превосходит классические эвристики на ~8 мс RTT, затрачивая на 95% меньше зондирующих пакетов.
- **КПД проверок (Actionable Probes):** у ZERO **до 92.5% проверок** приводят к полезным переключениям (против 5–13% у фиксированного таймера, тратящего пакеты вслепую).
- В обычном проводном десктопном режиме штатная эвристика Mihomo обеспечивает максимальную простоту и надёжность, поэтому ZERO остаётся в изолированном исследовательском контуре.

Воспроизводимые бенчмарки доступны в `crates/multilink-mihomo/examples/`:
- `connection_profiler.rs` — профилирование компонентов задержки и QUIC/UDP 443.
- `cold_warm_verification.rs` — 100-серийная верификация прогретого соединения.
- `browser_connection_reuse.rs` — замер мультиплексирования реального движка Chromium.
- `multipath_benchmark.rs` — сравнение Single Best vs Consistent Hashing vs Round-Robin.
- `zero_pareto_benchmark.rs` — построение кривой Парето «стоимость проверок / задержка».

---

## Сборка и запуск

### Требования
- **Rust Stable** (редакция 2021 или новее)
- **Visual Studio Build Tools** с установленными компонентами C++
- Исполняемый файл [Mihomo](https://github.com/MetaCubeX/mihomo/releases) (поместите `mihomo.exe` в директорию `vendor/mihomo/` или задайте путь через переменную окружения `MULTILINK_MIHOMO`).

### Сборка и запуск из исходников

```powershell
# Запуск приложения в режиме разработки
cargo run -p multilink-app

# Сборка оптимизированного релизного бинарника
cargo build --release -p multilink-app
.\target\release\multilink-app.exe
```

### Запуск тестов

```powershell
cargo test --workspace
```

---

## Авторство и лицензия

- **Автор:** [Артур Валиев (Arthur Valiev)](https://desk.everty.ru/opensource)
- **Лицензия:** [MIT License](LICENSE) © 2026 Артур Валиев. Проект открыт для сообщества в рамках [Everty OpenSource](https://desk.everty.ru/opensource).
