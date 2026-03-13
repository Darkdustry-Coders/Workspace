# Buildscript Documentation

## Overview

Buildscript - это система сборки на Rust для управления сложными проектами Mindustry с множеством зависимостей. Она автоматизирует загрузку, сборку и запуск компонентов системы.

## Структура проекта

```
buildscript/
├── Cargo.toml           # Конфигурация Rust проекта
├── src/
│   ├── main.rs         # Точка входа
│   ├── args.rs         # Парсинг аргументов командной строки
│   ├── util.rs         # Утилиты и вспомогательные функции
│   ├── targets.rs      # Определения целей сборки
│   └── targets/        # Реализации целей сборки
│       ├── mprocs.rs      # Task runner
│       ├── coreutils.rs   # Core утилиты ОС
│       ├── rabbitmq.rs    # RabbitMQ брокер сообщений
│       ├── surrealdb.rs   # SurrealDB база данных
│       ├── mindustry.rs   # Mindustry сервер
│       ├── java.rs        # Java JDK
│       ├── coreplugin.rs  # Основной плагин
│       ├── forts.rs       # Forts плагин
│       ├── hub.rs         # Hub плагин
│       ├── hexed.rs       # Hexed плагин
│       ├── newtd.rs       # New Tower Defense плагин
│       └── java-version-check.java  # Проверка версии Java
└── assets/             # Шаблоны конфигураций
    ├── settings.gradle.in
    ├── shared.settings.gradle.in
    └── Cargo.toml.in
```

## Архитектура

### Модуль `args.rs`

Отвечает за парсинг аргументов командной строки.

#### Основные структуры:

- **`Args`** - Перечисление возможных команд:
  - `Build { build: BuildArgs, env: EnvTy }` - Сборка проекта
  - `Env { command: Vec<String>, env: EnvTy }` - Запуск команды в окружении
  - `Help` - Показать справку

- **`BuildArgs`** - Параметры сборки:
  - `mindustry_version` - Версия Mindustry (v146, v149, v150, v153, v154, v155, be)
  - `targets` - Список целей для сборки
  - `git_backend` - Бэкенд Git (SSH или HTTPS)
  - `ports_start` - Начальный порт для сервисов
  - `server_ip` - IP сервера
  - `rabbitmq_url` - URL RabbitMQ
  - `surrealdb_url` - URL SurrealDB
  - `java_stackstrace` - Включить stacktrace Java

- **`EnvTy`** - Тип окружения:
  - `Isolate` - Изолированное окружение
  - `Autoinstall` - Автоматическая установка
  - `Host` - Использовать системные инструменты

- **`MindustryVersion`** - Версии Mindustry
- **`GitBackend`** - Git бэкенд (Ssh/Https)

#### Функции:

- `print_help()` - Вывод справки
- `args()` - Парсинг аргументов командной строки

### Модуль `targets.rs`

Определяет систему целей сборки и их управление.

#### Основные трейты:

- **`TargetImpl`** - Базовый трейт для всех целей:
  - `build()` - Сборка цели
  - `run_init()` - Инициализация запуска
  - `run()` - Запуск цели

- **`TargetImplStatic`** - Статические методы целей:
  - `flags()` - Флаги цели
  - `depends()` - Зависимости
  - `initialize_host()` - Инициализация через системные инструменты
  - `initialize_cached()` - Инициализация из кэша
  - `initialize_local()` - Локальная инициализация

#### Структуры данных:

- **`TargetFlags`** - Флаги цели:
  - `always_local` - Всегда локальная установка
  - `deprecated` - Устаревшая цель

- **`TargetEnabled`** - Состояние цели:
  - `No` - Отключена
  - `Depend` - Зависимость
  - `Build` - Требуется сборка

- **`TargetList`** - Список целей
- **`Targets<'a>`** - Контейнер для всех целей
- **`InitParams`** - Параметры инициализации
- **`BuildParams`** - Параметры сборки
- **`RunParams`** - Параметры запуска

#### Доступные цели:

1. **mprocs** - Task runner для параллельного выполнения
2. **coreutils** - Базовые утилиты ОС
3. **rabbitmq** - Брокер сообщений RabbitMQ
4. **surrealdb** - База данных SurrealDB
5. **mindustry** - Сервер Mindustry
6. **java** - Java JDK (требуется Java 17+)
7. **coreplugin** - Основной плагин Mindustry
8. **forts** - Forts игровой режим
9. **hub** - Hub сервер
10. **hexed** - Hexed игровой режим
11. **newtd** - New Tower Defense игровой режим

### Модуль `util.rs`

Вспомогательные утилиты.

#### Глобальные переменные:

- `CURRENT_DIR` - Текущая директория

#### Функции:

- `current_dir()` - Получить текущую директорию
- `is_executable()` - Проверить, является ли файл исполняемым
- `find_executable()` - Найти исполняемый файл в PATH
- `write_if_diff()` - Записать файл, если он изменился
- `download()` - Скачать файл с прогрессом
- `untar_gz()` - Распаковать tar.gz
- `untar_xz()` - Распаковать tar.xz
- `symlink_file()` - Создать символическую ссылку

#### Макросы:

- `exe_path!` - Получить путь к исполняемому файлу (с .exe на Windows)

#### Итераторы:

- `Interject` - Итератор для вставки элементов
- `EitherIter` - Итератор-обёртка Either

### Модуль `main.rs`

Точка входа в приложение.

#### Логика работы:

1. Инициализация `CURRENT_DIR`
2. Загрузка `.env` файла
3. Парсинг аргументов
4. Обработка команд:
   - `Help` - Вывод справки
   - `Env` - Запуск команды в окружении
   - `Build` - Сборка и запуск целей

#### Процесс сборки:

1. Очистка директорий `.build` и `.bin`
2. Создание `.bin`
3. Инициализация всех целей
4. Генерация конфигурационных файлов
5. Сборка всех целей
6. (опционально) Запуск сервисов

## Цели сборки (Targets)

### MProcs

Task runner для параллельного выполнения процессов.

- URL: `https://github.com/pvolok/mprocs`
- Версия: 0.7.3
- Порт: динамический

### CoreUtils

Базовые утилиты ОС через busybox.

- URL: `https://busybox.net/downloads/binaries/`
- Проверяет наличие: `uname`, `yes`, `[`, `cat`, `touch`

### RabbitMQ

Брокер сообщений.

- URL: `https://github.com/rabbitmq/rabbitmq-server`
- Версия: 4.1.2
- Порт AMQP: динамический
- Порт Management: динамический

### SurrealDB

База данных.

- URL: `https://github.com/surrealdb/surrealdb`
- Версия: 3.0.0-beta.4
- Порт: динамический
- Пользователь: admin/password

### Mindustry

Сервер Mindustry.

Поддерживаемые версии:
- v146 (5GameMaker fork)
- v149, v150, v153, v154, v155 (Anuken официальные)
- Bleeding Edge

### Java

JDK для сборки Java-компонентов.

- Минимальная версия: 17
- Автоматическая загрузка: Eclipse Temurin JDK 21
- URL: `https://github.com/adoptium/temurin21-binaries`

### CorePlugin

Основной плагин Darkdustry.

- Репозиторий: `Darkdustry-Coders/CorePlugin`
- Зависимости: Java, RabbitMQ, SurrealDB, Mindustry
- Создаёт: `.bin/CorePlugin.jar`

### Forts

Игровой режим Forts.

- Репозиторий: `Darkdustry-Coders/Forts`
- Зависимости: Java, CorePlugin
- Создаёт: `.bin/Forts.jar`

### Hub

Hub сервер.

- Репозиторий: `Darkdustry-Coders/LightweightHub`
- Зависимости: Java, CorePlugin
- Создаёт: `.bin/LightweightHub.jar`

### Hexed

Игровой режим Hexed.

- Репозиторий: `Darkdustry-Coders/HexedPlugin`
- Зависимости: Java, CorePlugin
- Создаёт: `.bin/Hexed.jar`

### NewTD

Игровой режим New Tower Defense.

- Зависимости: Java, CorePlugin

## Использование

### Базовые команды

```bash
# Сборка всех целей
./buildscript all

# Сборка конкретных целей
./buildscript java coreplugin forts

# Сборка и запуск
./buildscript all run

# Показать справку
./buildscript --help
```

### Параметры окружения

```bash
# Изолированное окружение
./buildscript --isolate all

# Автоматическая установка
./buildscript --autoinstall all

# Использование SSH для Git
./buildscript --ssh all
```

### Параметры сборки

```bash
# Указать версию Mindustry
./buildscript --mindustry v155 all

# Включить stacktrace Java
./buildscript --stacktrace all

# Указать IP сервера
./buildscript --server-ip 192.168.1.1 all

# Использовать внешний RabbitMQ
./buildscript --rabbitmq amqp://user:pass@host:5672 all

# Использовать внешнюю SurrealDB
./buildscript --surrealdb ws://user:pass@host:8000/db all
```

### Запуск команд в окружении

```bash
# Запуск shell
./buildscript --env

# Запуск конкретной команды
./buildscript --env gradle build
```

## Файлы конфигурации

### Генерируемые файлы

- `settings.gradle` - Gradle настройки workspace
- `buildscript/assets/shared.settings.gradle` - Общие Gradle настройки
- `Cargo.toml` - Rust workspace конфигурация
- `.run/sharedConfig.toml` - Общая конфигурация для запуска

### Директории

- `.cache/tools/` - Локально установленные инструменты
- `.bin/` - Скомпилированные артефакты
- `.run/` - Директория для запуска сервисов
- `.build/` - Временные файлы сборки (очищается)

## Зависимости

### Rust зависимости

- `flate2` - Сжатие gzip
- `tar` - Работа с tar архивами
- `xz` - Сжатие xz
- `dotenvy` - Загрузка .env файлов
- `ureq` - HTTP клиент

### Внешние зависимости

- Git
- Для Unix: базовые утилиты (bash, tar, etc.)

## Особенности реализации

1. **Трёхуровневая инициализация** - Попытка использовать системные → кэшированные → локально загруженные инструменты

2. **Динамические порты** - Каждый сервис получает уникальный порт начиная с 4100

3. **Умная запись файлов** - `write_if_diff` не перезаписывает неизменённые файлы

4. **MProcs интеграция** - Все процессы запускаются через mprocs для удобного управления

5. **Символические ссылки** - Плагины линкуются в конфигурации через symlink

6. **Генерация settings.bin** - Бинарный формат конфигурации Mindustry

## Ограничения

- Требуется Unix-система (Linux/macOS)
- Windows поддержка частичная
- Требуется Java 17+ для сборки
- Некоторые цели всегда устанавливаются локально

## Лицензия

См. основной файл лицензии проекта.
